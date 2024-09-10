use std::{borrow::Borrow, fmt::format, rc::Rc, thread::current, usize};

use fact::MapFact;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::{DefPath, DefPathData}};
use rustc_middle::{
    mir::{self, BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, Local, LocalDecls, Place, Rvalue, Successors, TerminatorKind, VarDebugInfoContents}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{Lock, LockGuard, LockSetFact};
use alias::{AliasAnalysis, AliasFact, AliasSet, LockObject, VariableNode};

use rustc_middle::mir::{
    Location,
    Body,
    Statement,
    Terminator,
};


mod visitor;
pub mod callgraph;
pub mod lock;
pub mod alias;
pub mod fact;

pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    call_graph: CallGraph<'tcx>,
    
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<DefId, FxHashMap<usize, LockSetFact>>,

    // alias_flow_graph: record the alias relationship for each function
    alias_map: FxHashMap<DefId, FxHashMap<usize, AliasFact>>,

    // the traversing order of bbs in each function
    control_flow_graph: FxHashMap<DefId, Vec<BasicBlock>>,
    // intra-analysis data
    // record all variable debug info in current function body
    // TODO: shadow nested scope
    var_debug_info: FxHashMap<usize, String>,
}

impl<'tcx> LockSetAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>, alias_map: FxHashMap<DefId, FxHashMap<usize, AliasFact>>) -> Self{
        Self{
            tcx,
            lock_set_facts: FxHashMap::default(),
            alias_map,
            var_debug_info: FxHashMap::default(),
            call_graph,
            control_flow_graph: FxHashMap::default(),
        }
    }

    pub fn run_analysis(&mut self){
        // traverse the functions in a reversed topo order 
        for def_id in self.call_graph.topo.clone(){
            if self.tcx.is_mir_available(def_id) && self.alias_map.contains_key(&def_id){
                // each function is analyzed only once
                let body = self.tcx.optimized_mir(def_id);
                println!("Now analyze function {:?}, {:?}", body.span, self.tcx.def_path_str(def_id));
                if self.tcx.def_path(def_id).data.len() == 1{
                    // only analyze functions defined in current crate
                    // FIXME: closure?
                    self.visit_body(def_id, body);
                }      
            }
        }
        self.after_run();
    }

    fn init_func(&mut self, def_id: &DefId, body: &Body){
        // 1. init the facts
        if self.tcx.is_mir_available(def_id) && self.tcx.def_path(def_id.clone()).data.len() == 1 {
            self.alias_map.entry(def_id.clone()).or_insert(FxHashMap::default());
            self.lock_set_facts.entry(def_id.clone()).or_insert(FxHashMap::default());
            for (index, basic_block_data) in self.tcx.optimized_mir(def_id).basic_blocks.iter().enumerate(){
                if !basic_block_data.is_cleanup{
                    self.alias_map.get_mut(&def_id).unwrap().entry(index).or_insert(FxHashMap::default());
                    self.lock_set_facts.get_mut(&def_id).unwrap().entry(index).or_insert(FxHashMap::default());
                }
            }
        }

        // 2. resolve the var_debug_info to get the var names
        self.var_debug_info.clear(); // TODO: closure move? how to clear
        for (_, var) in body.var_debug_info.iter().enumerate(){
            let mut a = usize::MAX;
            if let VarDebugInfoContents::Place(p) = &var.value{
                a = p.local.as_usize();
            }
            else {
                todo!();
            }
            self.var_debug_info.insert(a, var.name.to_string());
        }
        println!("{:?}", self.var_debug_info);

        // 3. compute the confrol flow graph in a reverse post-order
        let mut reverse_post_order = vec![];
        for bb in body.basic_blocks.reverse_postorder(){
            if !body.basic_blocks.get(*bb).unwrap().is_cleanup{
                reverse_post_order.push(bb.clone());
            }
        }
        println!("{:?}", reverse_post_order);
        self.control_flow_graph.entry(def_id.clone()).or_insert(reverse_post_order);
        

        // 4. resolve all the local declarations before statements, maybe?
        let decls = body.local_decls();
        for (local, decl) in decls.iter_enumerated(){
            let ty = decl.ty;
            let index = local.as_usize();
        }

    }

    pub fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>){
        self.init_func(&def_id, body);
        // FIXME: redundant clone
        for current_bb_index in self.control_flow_graph[&def_id].clone(){
            println!("bb {:?} now under lock set analysis ", current_bb_index);
            self.visit_bb(def_id, current_bb_index.as_usize(),  body);
        }
    }

    pub fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body){
        // merge the pres
        for pre in body.basic_blocks.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        // only need to visit the terminator (function call)
        // 1. guard = *.lock()
        // 2. drop(guard)
        self.visit_terminator(&def_id, bb_index, &body.basic_blocks[BasicBlock::from(bb_index)].terminator().kind);
    }

    pub fn merge(&mut self, pre: &BasicBlock, def_id: DefId, bb_index: usize){
        // merge the lock set
        let pre_lock_fact = self.lock_set_facts[&def_id][&pre.as_usize()].clone();
        self.lock_set_facts.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().clear();
        self.lock_set_facts.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().meet(pre_lock_fact);     
    }

    fn visit_terminator(&mut self, def_id: &DefId, bb_index: usize, terminator_kind: &TerminatorKind){
        let alias_map = self.alias_map.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
        match terminator_kind{ // TODO: if return a lock?
            rustc_middle::mir::TerminatorKind::Drop { place, target, .. } => {
                // if drop a lock guard, find it/its alias and kill it in lock fact
                let to_be_dropped = resolve_project(place);
                let lock_set_fact = self.lock_set_facts.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
                
                // FIXME: drop all the alias may be unsound, if multiple locks are going to be dropped here
                // Peahen only drops those variable which only points to one lock in the fact
                for alias in self.alias_map.get(def_id).unwrap().get(&bb_index).unwrap().get(&to_be_dropped).unwrap().1.variables.borrow().iter(){
                    if lock_set_fact.contains_key(&alias.index){
                        lock_set_fact.remove(&alias.index);
                    }
                }
            },
            rustc_middle::mir::TerminatorKind::Call { func, args, destination, target, unwind, call_source, fn_span } => {
                match func{
                    mir::Operand::Constant(constant) => {
                        match constant.ty().kind(){
                            rustc_type_ir::TyKind::FnDef(fn_id, _) => {
                                // _* = func(args) -> [return: bb*, unwind: bb*] @ Call: FnDid: *
                                // ^
                                // |
                                // This _* is always a variable/temp to receive the return value
                                // i.e., do not need to resolve the projection of destination
                                // interprocedural analysis just resolves the `func(args)` part, need to resolve the 
                                // TODO: for those imported modules or functions which are available?
                                // e.g., Mutex::new() is available for mir

                                // TODO: first we model some function calls:
                                // new, deref, clone ... 

                                // if not available ==> 2 situations:
                                // 1. the destination (i.e., return value) is an owned =>
                                //    search the decls for it, and init a new owned node
                                //    FIXME: if the destination is a smart pointer or struct?
                                // 2. the destination is a reference =>
                                // it must point to one of the args
                                let def_path = self.tcx.def_path(fn_id.clone());
                                let def_path_str = self.tcx.def_path_str(fn_id);
                                let left = resolve_project(&destination);
                                if let DefPathData::ValueNs(name) = &def_path.data[def_path.data.len() - 1].data{
                                    if is_mutex_method(&def_path_str){
                                        if name.as_str() == "lock"{
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) |
                                                mir::Operand::Move(p) => {
                                                    let right =  resolve_project(p);
                                                    // update the lock set facts
                                                    let lock_set_fact = self.lock_set_facts.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
                                                    lock_set_fact.update(left, LockGuard::new(left, LockObject::new(right)));
                                                },
                                            }
                                        }
                                    }
                                }
                            },
                            // maybe problematic
                            rustc_type_ir::TyKind::FnPtr(_) => panic!("TODO: FnPtr"),
                            rustc_type_ir::TyKind::Closure(_, _) => panic!("TODO: closure"),
                            _ => (),
                        }
                    },
                    _ => (),
                }
            },

            rustc_middle::mir::TerminatorKind::Goto { .. } => (),
            rustc_middle::mir::TerminatorKind::SwitchInt { .. } => (),
            rustc_middle::mir::TerminatorKind::UnwindResume => (),
            rustc_middle::mir::TerminatorKind::UnwindTerminate(_) => (),
            rustc_middle::mir::TerminatorKind::Return => (),
            rustc_middle::mir::TerminatorKind::Unreachable => (),
            rustc_middle::mir::TerminatorKind::Assert { .. } => (),
            rustc_middle::mir::TerminatorKind::Yield { .. } => (),
            rustc_middle::mir::TerminatorKind::CoroutineDrop => (),
            rustc_middle::mir::TerminatorKind::FalseEdge { .. } => (),
            rustc_middle::mir::TerminatorKind::FalseUnwind { .. } => (),
            rustc_middle::mir::TerminatorKind::InlineAsm { .. } => (),
        }
    }
    
    pub fn after_run(&self){
        self.print_alias();
        self.print_lock_facts();
    }

    fn print_alias(&self){
        let mut grouped_map: FxHashMap<DefId, Vec<(usize, &FxHashMap<usize, (Rc<VariableNode>, Rc<AliasSet>)>)>> = FxHashMap::default();
        for (def_id, value) in &self.alias_map {
            for (key_usize, value) in value{
                grouped_map
                .entry(def_id.clone())
                .or_insert_with(Vec::new)
                .push((*key_usize, value));
            }
        }

        println!("Alias facts: ");
        for (def_id, mut vec) in grouped_map {
            vec.sort_by_key(|k| k.0);
            println!("{:?}:", def_id);
            for (key_usize, value) in vec {
                println!("bb {}   ", key_usize);
                let mut v: Vec<&usize> = value.keys().collect();
                v.sort();
                for i in v{
                    println!("variable {} -> {:?}", i, value[i]);
                }
            }
            println!();
        }
    }

    
    fn print_lock_facts(&self){
        let mut grouped_map: FxHashMap<DefId, Vec<(usize, &FxHashMap<usize, Rc<LockGuard>>)>> = FxHashMap::default();
        for (def_id, value) in &self.lock_set_facts {
            for (key_usize, value) in value{
                grouped_map
                .entry(def_id.clone())
                .or_insert_with(Vec::new)
                .push((*key_usize, value));
            }
        }

        println!("Lock set facts: ");
        for (def_id, mut vec) in grouped_map {
            vec.sort_by_key(|k| k.0); // 按 usize 排序
            println!("{:?}:", def_id);
            for (key_usize, value) in vec {
                println!("bb {} -> {:?}", key_usize, value);
            }
            println!();
        }
    }
}


/// whether a type is lock
pub fn is_lock(ty: &Ty) -> bool{
    // TODO: better logic
    let ty = format!("{:?}", ty);
    return ty.contains("Mutex") || ty.contains("Rwlock"); // TODO: RwLock
}

pub fn resolve_project(p: &Place) -> usize {
    let mut cur = p.local.as_usize();
    for projection in p.projection{
        match &projection{ // TODO: complex types
            mir::ProjectionElem::Deref => (),
            mir::ProjectionElem::Field(_, _) => (),
            mir::ProjectionElem::Index(_) => todo!(),
            mir::ProjectionElem::ConstantIndex { offset, min_length, from_end } => todo!(),
            mir::ProjectionElem::Subslice { from, to, from_end } => todo!(),
            mir::ProjectionElem::Downcast(_, _) => todo!(),
            mir::ProjectionElem::OpaqueCast(_) => todo!(),
            mir::ProjectionElem::Subtype(_) => todo!(),
        }
    }
    cur
}

pub fn is_primitive<'tcx>(ty: &Ty<'tcx>) -> bool{
    match ty.kind() {
        ty::Bool
        | ty::Char
        | ty::Int(_)
        | ty::Uint(_)
        | ty::Float(_) => true,
        ty::Array(ref t,_) => is_primitive(t),
        ty::Adt(_, ref args) => {
            for t in args.types() {
                if !is_primitive(&t) {
                    return false;
                }
            }
            true
        },
        ty::Tuple(ref tys) => {
            for t in tys.iter() {
                if !is_primitive(&t) {
                    return false;
                }
            }
            true
        },
        _ => false,
    }
}

pub fn is_mutex_method(def_path: &String) -> bool{
    def_path.starts_with("std::sync::Mutex")
}

pub fn is_smart_pointer(def_path: &String) -> bool{
    def_path.starts_with("std::sync::Arc")
}

fn find_guard(lock_fact: &LockSetFact, id: usize){
    
}

