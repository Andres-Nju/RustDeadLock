use std::{fmt::format, usize, rc::Rc};

use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::{DefPath, DefPathData}};
use rustc_middle::{
    mir::{BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, LocalDecls, Place, Rvalue, Successors, TerminatorKind, VarDebugInfoContents}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{Lock, LockGuard, LockSetFact};
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

pub type AliasFact = FxHashMap<usize, Rc<AliasSet>>;
pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    call_graph: CallGraph<'tcx>,
    
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<DefId, FxHashMap<usize, LockSetFact>>,


    // Lock Flow Graph: record 
    // lock_flow_graph: FxHashMap<DefId, IndexVec<Local, Node>>,

    // alias_flow_graph: record the alias relationship for each function
    alias_map: FxHashMap<DefId, FxHashMap<usize, AliasFact>>,


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
            alias_map: FxHashMap::default(),
            var_debug_info: FxHashMap::default(),
            call_graph,
        }
    }

    pub fn run_analysis(&mut self){
        // initialize
        self.init();
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
        self.print_alias();
        self.print_lock_facts();
    }

    fn init(&mut self){
        for def_id in self.call_graph.topo.clone(){
            if self.tcx.is_mir_available(def_id) && self.tcx.def_path(def_id).data.len() == 1 {
                self.alias_map.entry(def_id).or_insert(FxHashMap::default());
                self.lock_set_facts.entry(def_id).or_insert(FxHashMap::default());
                for (index, basic_block_data) in self.tcx.optimized_mir(def_id).basic_blocks.iter().enumerate(){
                    if !basic_block_data.is_cleanup{
                        self.alias_map.get_mut(&def_id).unwrap().entry(index).or_insert(FxHashMap::default());
                        self.lock_set_facts.get_mut(&def_id).unwrap().entry(index).or_insert(FxHashMap::default());
                    }
                }
                
            }
        }
    }
    fn print_alias(&self){
        let mut grouped_map: FxHashMap<DefId, Vec<(usize, &FxHashMap<usize, Rc<AliasSet>>)>> = FxHashMap::default();
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
            vec.sort_by_key(|k| k.0); // 按 usize 排序
            println!("{:?}:", def_id);
            for (key_usize, value) in vec {
                println!("bb {}   ", key_usize);
                let mut v: Vec<&usize> = value.keys().collect();
                v.sort();
                for i in v{
                    println!("variable {} -> {:?}", i, value[i]);
                }
            }
        }
        println!();
    }

    fn print_lock_facts(&self){
        let mut grouped_map: FxHashMap<DefId, Vec<(usize, &FxHashMap<usize, LockGuard>)>> = FxHashMap::default();
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
        }
        println!();
    }

    fn init_func(&mut self, def_id: &DefId, body: &Body){
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

        let alias_map = self.alias_map.get_mut(&def_id).unwrap();
        // 2. resolve all the function arguments (parameters, actually)
        for arg in body.args_iter(){
            // TODO: the args, should be processed in inter-procedural analysis
            if is_lock(&body.local_decls[arg].ty) {
                let index = arg.as_usize();
                // create a node for the arg (parameter, actually)
                let alias_map = alias_map.get_mut(&0).unwrap();
                alias_map.entry(index).or_insert(AliasSet::new_self(VariableNode::new(index)));
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
        self.init_func(&def_id, body);
        
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
        let mut flag = false;
        let mut gotos = vec![];
        // if fact[bb] is none, initialize one
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];

        // merge the pres
        let temp1 = self.lock_set_facts[&def_id][&bb_index].clone();
        let temp2 = self.alias_map[&def_id][&bb_index].clone();
        for pre in body.basic_blocks.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        // traverse the bb's statements
        data.statements.iter().for_each(|statement| self.visit_statement(def_id, bb_index, statement, body.local_decls()));
        // process the terminator
        self.visit_terminator(&def_id, bb_index, &data.terminator().kind, &mut gotos);
        
        flag |= temp1.keys().collect::<FxHashSet<_>>().eq(&self.lock_set_facts[&def_id][&bb_index].keys().collect());
    
        // TODO: how to judge the differences in aliases?
        // flag |= temp2.keys().collect::<FxHashSet<_>>().eq(&self.lock_set_facts[&def_id][&bb_index].keys().collect());
        if flag{
            Some(gotos)
        }
        else {
            None
        }
    }

    fn visit_terminator(&mut self, def_id: &DefId, bb_index: usize, terminator_kind: &TerminatorKind, gotos: &mut Vec<usize>){
        let alias_map = self.alias_map.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
        match terminator_kind{ // TODO: if return a lock?
            rustc_middle::mir::TerminatorKind::Drop { place, target, .. } => {
                gotos.push(target.as_usize());
                // if drop a lock guard, find it and kill it in lock fact
                let local = resolve_project(place);
                let lock_fact = self.lock_set_facts.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
                if !lock_fact.contains_key(&local){
                    return;
                }
                // look at whether the lock fact contains local, if so, just remove it
                println!("the lock need to be remove: {:?}", local);
                if let None = lock_fact.remove(&local) {
                    panic!("Can not find any lock guard of index {:?}", local);
                }
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
                                let def_path_str = self.tcx.def_path_str(fn_id);
                                let left = resolve_project(&destination);
                                if let DefPathData::ValueNs(name) = &def_path.data[def_path.data.len() - 1].data{
                                    if is_mutex_method(&def_path_str){
                                        if name.as_str() == "new"{
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                mir::Operand::Copy(_) => {
                                                    panic!("should not go to this branch!");
                                                }
                                                mir::Operand::Constant(_) |
                                                mir::Operand::Move(_) => {
                                                    let lock_object = LockObject::new(def_id.clone(), left);
                                                    let left_var = VariableNode::new(left);
                                                    assert!(!alias_map.contains_key(&left));
                                                    alias_map.insert(left, AliasSet::new_self(left_var));
                                                    // todo: how to manage the lock
                                                },
                                            }
                                        }
                                        //TODO: Clone?
                                        else if name.as_str() == "lock"{
                                            // FIXME：这里可能要重新设计一下，
                                            // 1. 每个bb的Lock fact是HashMap <HashSet<Lock>>的情形，或许要合并？
                                            // 2. 如果是同一个锁，如何存储？
                                            // _1 = std::sync::Mutex::<T>::lock(move _2)
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) |
                                                mir::Operand::Move(p) => {
                                                    let right =  resolve_project(p);
                                                    let right_var  = alias_map.get(&right).unwrap();
                                                    
                                                    let left = resolve_project(&destination);
                                                    let left_var = VariableNode::new(left);

                                                    // TODO: update the lock set facts
                                                    // let fact = self.lock_set_facts.get_mut(&(def_id.clone(), bb_index)).unwrap();
                                                    // fact.insert(left, LockGuard::new(right_var.get_possible_locks()));
                                                    
                                                    // update alias_map
                                                    // left_var.merge_alias_set(right_var);
                                                    // left_var.strong_update_possible_locks(right_var);
                                                    assert!(!alias_map.contains_key(&left));
                                                    alias_map.insert(left, AliasSet::new_self(left_var));
                                                },
                                            }
                                        }
                                    }
                                    else if is_smart_pointer(&def_path_str){
                                        if name.as_str() == "new"{
                                            // the same as ref assign
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                mir::Operand::Constant(_) |
                                                mir::Operand::Copy(_) => {
                                                    panic!("should not go to this branch!");
                                                }
                                                mir::Operand::Move(p) => {
                                                    let right = resolve_project(p);
                                                    let right_var_set = alias_map.get(&right).unwrap();
                                                    let left_var = VariableNode::new(left);
                                                    // left_var.merge_alias_set(right_var);
                                                    // left_var.strong_update_possible_locks(right_var);
                                                    assert!(!alias_map.contains_key(&left));
                                                    right_var_set.add_variable(left_var);
                                                    alias_map.insert(left, right_var_set.clone());
                                                },
                                            }
                                        }
                                    }
                                    else if name.as_str() == "unwrap"{ // just update the arg with destination
                                        // unwrap and lock all not merge the alias of right to left
                                        assert_eq!(1, args.len());
                                        if !is_lock(&self.tcx.optimized_mir(def_id).local_decls[Local::from_usize(left)].ty){
                                            return;
                                        }
                                        match &args[0]{
                                            // must be move _*
                                            mir::Operand::Copy(_) => todo!(),
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Move(p) => {
                                                // like move assign, replace the old guard with new guard
                                                let right = resolve_project(p);
                                                let left = resolve_project(destination);
                                                let lock_fact = self.lock_set_facts.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
                                                
                                                // if let Some(value) = lock_fact.remove(&right){
                                                //     lock_fact.insert(left, value);
                                                // }
                                                // else {
                                                //     panic!("Can not find any lock guard of index {:?}", right);
                                                // }
                                                // update alias map
                                                let left_var = VariableNode::new(left);
                                                let right_var_set  = alias_map.get(&right).unwrap();
                                                // left_var.strong_update_possible_locks(right_var);
                                                assert!(!alias_map.contains_key(&left));
                                                // alias_map.insert(left, AliasSet::new_self(left_var));
                                                right_var_set.add_variable(left_var);
                                                alias_map.insert(left, right_var_set.clone());
                                            },
                                        }
                                    } 
                                    else if name.as_str() == "deref"{
                                        // FIXME: if the arg is not a smart pointer which wraps mutex
                                        assert_eq!(1, args.len());
                                        match &args[0]{
                                            // must be move _*
                                            mir::Operand::Copy(_) => todo!(),
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Move(p) => {
                                                // right is &a
                                                let right =  resolve_project(p);
                                                let right_var_set  = alias_map.get(&right).unwrap();
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(left);
                                                // left_var.merge_alias_set(right);
                                                // left_var.strong_update_possible_locks(right);
                                                assert!(!alias_map.contains_key(&left));
                                                // alias_map.insert(left, AliasSet::new_self(left_var));
                                                right_var_set.add_variable(left_var);
                                                alias_map.insert(left, right_var_set.clone());
                                            },
                                        }
                                    }
                                    else if name.as_str() == "clone"{
                                        assert_eq!(1, args.len());
                                        // FIXME: if the cloned object is not a smart pointer which wraps mutex
                                        match &args[0]{
                                            // must be copy _*
                                            mir::Operand::Move(_) => todo!(),
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Copy(p) => {
                                                let right =  resolve_project(p);
                                                let right_var_set  = alias_map.get(&right).unwrap();
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(left);
                                                // left_var.merge_alias_set(right);
                                                // left_var.strong_update_possible_locks(right);
                                                assert!(!alias_map.contains_key(&left));
                                                right_var_set.add_variable(left_var);
                                                alias_map.insert(left, right_var_set.clone());
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
        // TODO: the correct logic of merge
        // merge the lock set
        let pre_lock_fact = self.lock_set_facts[&def_id][&pre.as_usize()].clone();
        self.lock_set_facts.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().extend(pre_lock_fact);
        // merge the alias_map
        let pre_alias_fact = self.alias_map[&def_id][&pre.as_usize()].clone();
        self.alias_map.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().extend(pre_alias_fact);
        
    }

    pub fn visit_statement(&mut self, def_id: DefId, bb_index: usize, statement: &Statement<'tcx>, decls: &LocalDecls){
        match &statement.kind{
            rustc_middle::mir::StatementKind::Assign(ref assign) => {
                let left = resolve_project(&assign.0);
                if is_lock(&decls[Local::from_usize(left)].ty) {
                    self.visit_assign(&def_id, bb_index,&assign.0, &assign.1);
                }
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

    pub fn visit_assign(&mut self, def_id: &DefId, bb_index: usize, lhs: &Place, rhs: &Rvalue<'tcx>){
        let alias_map = self.alias_map.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
        // resolve lhs
        let left = resolve_project(lhs);
        // resolve rhs
        match rhs{
            Rvalue::Use(op) => {
                match op{
                    mir::Operand::Copy(p) => panic!("Mutex-related variables cannot be copied!"),
                    mir::Operand::Move(p) => {
                        
                    },
                    mir::Operand::Constant(_) => panic!("Mutex should not be constant!"),
                }
            },
            Rvalue::AddressOf(_, p) |
            Rvalue::Ref(_, _, p) => {
                let right = resolve_project(p);
                let right_var_set = alias_map.get(&right).unwrap();
                let left_var = VariableNode::new(left);
                // left_var.merge_alias_set(right_var);
                // left_var.strong_update_possible_locks(right_var);
                // alias_map.insert(left, left_var);
                assert!(!alias_map.contains_key(&left));
                right_var_set.add_variable(left_var);
                alias_map.insert(left, right_var_set.clone());
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

    pub fn get_ty(&self, def_id: &DefId, local_id: usize) -> Ty<'tcx>{
        self.tcx.optimized_mir(def_id).local_decls[Local::from_usize(local_id)].ty
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

