use std::{fmt::format, usize, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::DefPathData};
use rustc_middle::{
    mir::{BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, LocalDecls, Place, Rvalue, Successors, TerminatorKind, VarDebugInfoContents}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{Lock, LockSetFact};
use alias::{AliasSet, LockObject, VariableNode};

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


pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    call_graph: CallGraph<'tcx>,
    
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<(DefId, usize), LockSetFact>,


    // Lock Flow Graph: record 
    // lock_flow_graph: FxHashMap<DefId, IndexVec<Local, Node>>,

    // alias_flow_graph: record the alias relationship for each function
    alias_flow_graph: FxHashMap<DefId, FxHashMap<usize, Rc<VariableNode>>>,

    // map whether each bb has been visited
    // used for alias analysis
    visited: Vec<bool>,

    // intra-analysis data
    // record all variable debug info in current function body
    // TODO: shadow nested scope
    var_debug_info: FxHashMap<usize, String>,
}

impl<'tcx> LockSetAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>) -> Self{
        Self{
            tcx,
            lock_set_facts: FxHashMap::default(),
            alias_flow_graph: FxHashMap::default(),
            var_debug_info: FxHashMap::default(),
            call_graph,
            visited: vec![],
        }
    }

    pub fn run_analysis(&mut self){
        // traverse the functions in a reversed topo order 
        for def_id in self.call_graph.topo.clone(){
            // let body = self.tcx.instance_mir(ty::InstanceDef::Item(def_id));
            // TODO: which mir to choose? optimized or raw with storage statements?
            if self.tcx.is_mir_available(def_id) {
                // let body = &self.tcx.mir_built(def_id.as_local().unwrap()).steal();
                let body = self.tcx.optimized_mir(def_id);
                println!("Now analyze function {:?}, {:?}", body.span, self.tcx.def_path_str(def_id));
                if self.tcx.def_path(def_id).data.len() == 1{
                    self.visit_body(def_id, body);
                }
                
            }
            else {
                println!("Function {:?} MIR Unavailable!", def_id);
            }
        }
        for item in self.alias_flow_graph.iter(){
            println!("{:?}", item.0);
            for i in item.1{
                println!("{:?}", i);
            }
        }
    }

    fn init(&mut self, def_id: &DefId, body: &Body){
        // initialize current alias_graph
        self.alias_flow_graph.entry(def_id.clone()).or_insert_with(|| {
            FxHashMap::default()
        });
        self.visited = vec![false; body.basic_blocks.len()];
        // 1. resolve the var_debug_info to get the var names
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

        let alias_map = self.alias_flow_graph.get_mut(&def_id).unwrap();
        // 2. resolve all the function arguments (parameters, actually)
        for arg in body.args_iter(){
            // TODO: the args, should be processed in inter-procedural analysis
            if is_lock(&body.local_decls[arg].ty) {
                let index = arg.as_usize();
                alias_map.entry(index).or_insert(VariableNode::new(index));
            }
        }
        // 3. resolve all the local declarations before statements
        let decls = body.local_decls();
        
        for (local, decl) in decls.iter_enumerated(){
            let ty = decl.ty;
            let index = local.as_usize();
        }


    }
    pub fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>){
        self.init(&def_id, body);
        
        let mut work_list = vec![0];
        while !work_list.is_empty(){
            let current_bb_index = work_list.pop().expect("Elements in non-empty work_list should always be valid!");
            // println!("{:?}", self.alias_flow_graph[&def_id]);
            println!("now analysis bb {}", current_bb_index);
            if let Some(targets) = self.visit_bb(def_id, current_bb_index, body){
                work_list.extend(targets);
            }
        }
    }

    pub fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body<'tcx>) -> Option<Vec<usize>>{
        self.visited[bb_index] = true;
        // todo: if the bb is visited, do not need to analysis alias again
        let mut gotos = vec![];
        // TODO: maybe clean up bb?
        let mut flag = false;
        // if fact[bb] is none, initialize one
        self.lock_set_facts.entry((def_id, bb_index)).or_insert_with(|| {
            flag = true;
            FxHashSet::default()
        });
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];

        // merge the pres
        let temp = self.lock_set_facts[&(def_id, bb_index)].clone();
        for pre in body.basic_blocks.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        // traverse the bb's statements
        data.statements.iter().for_each(|statement| self.visit_statement(def_id, bb_index, statement, body.local_decls()));
        // process the terminator
        self.visit_terminator(&def_id, &data.terminator().kind, &mut gotos);
        
        flag |= temp.eq(&self.lock_set_facts[&(def_id, bb_index)]);
        if flag{
            Some(gotos)
        }
        else {
            None
        }
    }

    fn visit_terminator(&mut self, def_id: &DefId, terminator_kind: &TerminatorKind, gotos: &mut Vec<usize>){
        let alias_map = self.alias_flow_graph.get_mut(def_id).unwrap() ;
        match terminator_kind{ // TODO: if return a lock?
            rustc_middle::mir::TerminatorKind::Goto { target } => {
                gotos.push(target.as_usize());
            },
            rustc_middle::mir::TerminatorKind::SwitchInt { discr, targets } => {
                for bb in targets.all_targets(){
                    gotos.push(bb.as_usize());
                }
            },
            rustc_middle::mir::TerminatorKind::UnwindResume => (),
            rustc_middle::mir::TerminatorKind::UnwindTerminate(_) => (),
            rustc_middle::mir::TerminatorKind::Return => (),
            rustc_middle::mir::TerminatorKind::Unreachable => (),
            rustc_middle::mir::TerminatorKind::Drop { target, .. } => {
                gotos.push(target.as_usize());
            },
            rustc_middle::mir::TerminatorKind::Call { func, args, destination, target, unwind, call_source, fn_span } => {
                if let Some(bb) = target{
                    gotos.push(bb.as_usize());
                }
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
                                // println!("{:?}", def_path);
                                if let DefPathData::ValueNs(name) = &def_path.data[def_path.data.len() - 1].data{
                                    if self.tcx.def_path_str(fn_id.clone()).contains("sync::Mutex") && name.as_str() == "new"{
                                        let left = resolve_project(&destination);
                                        assert_eq!(1, args.len());
                                        match &args[0]{
                                            mir::Operand::Copy(_) => {
                                                panic!("should not go to this branch!");
                                            }
                                            mir::Operand::Constant(_) |
                                            mir::Operand::Move(_) => {
                                                let lock_object = LockObject::new(left);
                                                let left_var = VariableNode::new(left);
                                                left_var.add_possible_lock(lock_object);
                                                alias_map.insert(left, left_var);
                                            },
                                        }
                                    }
                                    else if name.as_str() == "deref" || name.as_str() == "unwrap"{
                                        assert_eq!(1, args.len());
                                        match &args[0]{
                                            // must be move _*
                                            mir::Operand::Copy(_) => todo!(),
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Move(p) => {
                                                // right is &a
                                                let r =  resolve_project(p);
                                                let right  = alias_map.get(&r).unwrap();
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(left);
                                                left_var.merge_alias_set(right);
                                                left_var.strong_update_possible_locks(right);
                                                alias_map.insert(left, left_var);
                                            },
                                        }
                                    } //TODO: Clone?
                                    else if name.as_str() == "lock"{
                                        // _1 = std::sync::Mutex::<T>::lock(move _2)
                                        assert_eq!(1, args.len());
                                        assert_eq!(1, args.len());
                                        match &args[0]{
                                            // must be move _*
                                            mir::Operand::Copy(_) => todo!(),
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Move(p) => {
                                                // right is &a
                                                let r =  resolve_project(p);
                                                let right  = alias_map.get(&r).unwrap();
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(left);
                                                left_var.merge_alias_set(right);
                                                left_var.strong_update_possible_locks(right);
                                                alias_map.insert(left, left_var);
                                            },
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
            rustc_middle::mir::TerminatorKind::Assert { target, .. } => {
                gotos.push(target.as_usize());
            },
            rustc_middle::mir::TerminatorKind::Yield { .. } => (),
            rustc_middle::mir::TerminatorKind::CoroutineDrop => (),
            rustc_middle::mir::TerminatorKind::FalseEdge { real_target, .. } => {
                gotos.push(real_target.as_usize());
            },
            rustc_middle::mir::TerminatorKind::FalseUnwind { real_target, .. } => {
                gotos.push(real_target.as_usize());
            },
            rustc_middle::mir::TerminatorKind::InlineAsm { destination, ..} => {
                if let Some(bb) = destination{
                    gotos.push(bb.as_usize());
                }
            },
        }
    }
    pub fn merge(&mut self, pre: &BasicBlock, def_id: DefId, bb_index: usize){
        // merge the lock set
        self.lock_set_facts.entry((def_id, pre.as_usize())).or_insert_with(|| {
            FxHashSet::default()
        }); 
        let pre_fact = self.lock_set_facts[&(def_id, pre.as_usize())].clone();
        self.lock_set_facts.get_mut(&(def_id, bb_index)).unwrap().extend(pre_fact);
    }

    pub fn visit_statement(&mut self, def_id: DefId, bb_index: usize, statement: &Statement<'tcx>, decls: &LocalDecls){
        match &statement.kind{
            rustc_middle::mir::StatementKind::Assign(ref assign) => {
                self.visit_assign(&def_id, &assign.0, &assign.1);
            },
            rustc_middle::mir::StatementKind::FakeRead(_) => (),
            rustc_middle::mir::StatementKind::SetDiscriminant { .. } => (),
            rustc_middle::mir::StatementKind::Deinit(_) => (),
            rustc_middle::mir::StatementKind::StorageLive(_) => (),
            rustc_middle::mir::StatementKind::StorageDead(_) => (),
            rustc_middle::mir::StatementKind::Retag(_, _) => (),
            rustc_middle::mir::StatementKind::PlaceMention(_) => (),
            rustc_middle::mir::StatementKind::AscribeUserType(_, _) => (),
            rustc_middle::mir::StatementKind::Coverage(_) => (),
            rustc_middle::mir::StatementKind::Intrinsic(_) => (),
            rustc_middle::mir::StatementKind::ConstEvalCounter => (),
            rustc_middle::mir::StatementKind::Nop => (),
        }
    }

    pub fn visit_assign(&mut self, def_id: &DefId, lhs: &Place, rhs: &Rvalue<'tcx>){
        let alias_map = self.alias_flow_graph.get_mut(def_id).unwrap();
        // resolve lhs
        let left = resolve_project(lhs);
        // resolve rhs
        match rhs{
            Rvalue::Use(op) => {
                match op{
                    mir::Operand::Copy(p) |
                    mir::Operand::Move(p) => {
                        let right = resolve_project(p);
                        let right_var = alias_map.get(&right).unwrap();
                        let left_var = VariableNode::new(left);
                        left_var.merge_alias_set(right_var);
                        left_var.alias_set.add_variable(right_var.clone());
                        left_var.strong_update_possible_locks(right_var);
                        alias_map.insert(left, left_var);
                    },
                    mir::Operand::Constant(_) => panic!("Mutex should not be constant!"),
                }
            },
            Rvalue::AddressOf(_, p) |
            Rvalue::Ref(_, _, p) => {
                let right = resolve_project(p);
                let right_var = alias_map.get(&right).unwrap();
                let left_var = VariableNode::new(left);
                left_var.merge_alias_set(right_var);
                left_var.alias_set.add_variable(right_var.clone());
                left_var.strong_update_possible_locks(right_var);
                alias_map.insert(left, left_var);
            },
            Rvalue::Repeat(_, _) => todo!(),
            Rvalue::ThreadLocalRef(_) => todo!(),
            Rvalue::Len(_) => todo!(),
            Rvalue::Cast(_, _, _) => (),
            Rvalue::Discriminant(_) => todo!(),
            Rvalue::Aggregate(_, _) => (), // TODO: 直接创建struct时
            Rvalue::ShallowInitBox(_, _) => todo!(),
            Rvalue::CopyForDeref(_) => todo!(),
            _ => (),
        }
    }

}

// copied from rustc_mir_dataflow::storage::always_storage_live_locals
// The set of locals in a MIR body that do not have `StorageLive`/`StorageDead` annotations.
//
// These locals have fixed storage for the duration of the body.
use rustc_index::{bit_set::BitSet, IndexVec};
use rustc_middle::mir::{self, Local};
use rustc_target::abi::VariantIdx;
pub fn always_storage_live_locals(body: &Body<'_>) -> BitSet<Local> {
    let mut always_live_locals = BitSet::new_filled(body.local_decls.len());

    for block in &*body.basic_blocks {
        for statement in &block.statements {
            use mir::StatementKind::{StorageDead, StorageLive};
            if let StorageLive(l) | StorageDead(l) = statement.kind {
                always_live_locals.remove(l);
            }
        }
    }

    always_live_locals
}

/// whether a type is lock
pub fn is_lock(ty: &Ty) -> bool{
    // TODO: better logic
    let ty = format!("{:?}", ty);
    return (ty.contains("Mutex") && !ty.contains("MutexGuard")) || ty.contains("Rwlock"); // TODO: RwLock
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