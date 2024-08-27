use std::{fmt::format, usize};

use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::DefPathData};
use rustc_middle::{
    mir::{BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, LocalDecls, Successors,VarDebugInfoContents,Place,Rvalue}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{Lock, LockSetFact};
use alias::Node;
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
    // call_graph: CallGraph<'tcx>,
    
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<(DefId, usize), LockSetFact>,


    // Lock Flow Graph: record 
    // lock_flow_graph: FxHashMap<DefId, IndexVec<Local, Node>>,
    alias_flow_graph: FxHashMap<DefId, FxHashMap<usize, Node<'tcx>>>,


    // intra-analysis data
    // record all local locks in current function body
    local_locks: FxHashMap<usize, Lock>,
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
            local_locks: FxHashMap::default(),
            var_debug_info: FxHashMap::default(),
        }
    }

    pub fn run_analysis(&mut self){
        for mir_key in self.tcx.mir_keys(()){
            // TODO: which order to traverse the program?
            let def_id = mir_key.to_def_id();
            // let body = self.tcx.instance_mir(ty::InstanceDef::Item(def_id));
            // TODO: which mir to choose? optimized or raw with storage statements?
            let body = &self.tcx.mir_built(def_id.as_local().unwrap()).steal();
            println!("{:?}, {:?}", body.span, self.tcx.def_path_str(def_id));
            self.intra_lock_set_analysis(def_id, body);
        }
    }
    pub fn intra_lock_set_analysis(&mut self, def_id: DefId, body: &Body<'tcx>){
        // first, read the LocalDecls to get all the local locks
        // note: need to resolve the var_debug_info to get the var names
        self.alias_flow_graph.entry(def_id).or_insert_with(|| {
            FxHashMap::default()
        });
        self.local_locks.clear();
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

        // than resolve all the local declarations before statements
        for arg in body.args_iter(){
            // TODO: the args, should be processed in inter-procedural analysis
        }
        let decls = body.local_decls();
        let alias_map = self.alias_flow_graph.get_mut(&def_id).unwrap();
        for (local, decl) in decls.iter_enumerated(){
            let ty = decl.ty;
            let index = local.as_usize();
            // if self.var_debug_info.contains_key(&index) && is_lock(&ty){
            //     // TODO: 这里读了ty两次，判断一次，插入一次
            //     self.local_locks.insert(local.as_usize(), Lock::new(self.var_debug_info[&index].clone(), &ty));
            // }

            // problematic
            if is_lock(&ty){
                self.local_locks.insert(local.as_usize(), Lock::new(local.as_usize().to_string(), &ty));
            }
            // add named owned variables into alias graph first
            // this is because all unnamed temps, or named references are derived from them
            // FIXME: 应该是最内层的内容，后续所有的内容都将指向它
            // 随后所有的deref操作都是返回的指向的内容的引用
            // 比如对Arc<Mutex>(记作变量a)做deref，首先调用的是&Arc<Mutex>.deref()
            // 即(&a).deref -> &Mutex, 而alias图中有a -> Mutex,
            // 综合起来就是有 &a -> a -> Mutex, deref之后图的样子应该是多出来&Mutex -> Mutex：
            // 即                         ^
            //                            |
            //                            |
            //                         &Mutex
            // _1:  @ std::sync::Arc<std::sync::Mutex<i32>, std::alloc::Global> 
            // _2:  @ std::sync::Mutex<i32> 
            if self.var_debug_info.contains_key(&index) && !ty.is_ref(){
                alias_map.insert(index, Node::new_owned(index, ty));
            }
        }
        // println!("{:?}", self.var_debug_info);
        // println!("{:?}", alias_map);

        let mut work_list = vec![0];
        while !work_list.is_empty(){
            let current_bb_index = work_list.pop().expect("Elements in non-empty work_list should always be valid!");
            println!("now analysis bb {}", current_bb_index);
            let current_bb_data = &body.basic_blocks[BasicBlock::from(current_bb_index)];
            if self.visit_bb(def_id, current_bb_index, &body.basic_blocks, decls){
                match &current_bb_data.terminator().kind{ // TODO: if return a lock?
                    rustc_middle::mir::TerminatorKind::Goto { target } => {
                        work_list.push(target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::SwitchInt { discr, targets } => {
                        for bb in targets.all_targets(){
                            work_list.push(bb.as_usize());
                        }
                    },
                    rustc_middle::mir::TerminatorKind::UnwindResume => (),
                    rustc_middle::mir::TerminatorKind::UnwindTerminate(_) => (),
                    rustc_middle::mir::TerminatorKind::Return => (),
                    rustc_middle::mir::TerminatorKind::Unreachable => (),
                    rustc_middle::mir::TerminatorKind::Drop { target, .. } => {
                        work_list.push(target.as_usize());
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
                                        if self.tcx.is_mir_available(fn_id){
                                            // TODO: interprocedural
                                        }

                                        // TODO: model some function calls:
                                        // deref, clone ... 
                                        // FIXME: now we just assume that the destination is an owned variable
                                        // i.e., we assume that functions never return a reference
                                        let mut alias_map = self.alias_flow_graph.get_mut(&def_id).unwrap();
                                        // or just let left = destination.local
                                        let left = resolve_project(destination, alias_map);
                                        alias_map.insert(left, Node::new_owned(left, decls[destination.local].ty));



                                        // FIXME
                                        let def_path = self.tcx.def_path(fn_id.clone());
                                        if let DefPathData::ValueNs(name) = &def_path.data[def_path.data.len() - 1].data{
                                            // println!("{:?}", call_source);
                                            if name.as_str() == "lock"{ // lock() is called, next resolve the arg 
                                                // _* = std::sync::Mutex::<T>::lock(move _*)
                                                assert_eq!(1, args.len());
                                                match &args[0]{
                                                    mir::Operand::Move(p) => {
                                                        assert!(self.local_locks.contains_key(&p.local.as_usize()));
                                                        let mut facts = self.lock_set_facts.get_mut(&(def_id, current_bb_index)).unwrap();
                                                        facts.insert(self.local_locks[&p.local.as_usize()].clone());
                                                        // println!("{:?}", self.lock_set_facts[&(def_id, current_bb_index)]);
                                                    },
                                                    _ => todo!(),
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
                        if let Some(bb) = target{
                            work_list.push(bb.as_usize());
                        }
                    },
                    rustc_middle::mir::TerminatorKind::Assert { target, .. } => {
                        work_list.push(target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::Yield { .. } => (),
                    rustc_middle::mir::TerminatorKind::CoroutineDrop => (),
                    rustc_middle::mir::TerminatorKind::FalseEdge { real_target, .. } => {
                        work_list.push(real_target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::FalseUnwind { real_target, .. } => {
                        work_list.push(real_target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::InlineAsm { destination, ..} => {
                        if let Some(bb) = destination{
                            work_list.push(bb.as_usize());
                        }
                    },
                }
            }
            println!("{:?}", self.alias_flow_graph[&def_id]);
            // println!("{:?}", self.lock_set_facts[&(def_id, current_bb_index)]);
        }  
    }

    pub fn visit_bb(&mut self, def_id: DefId, bb_index: usize, bbs: &BasicBlocks, decls: &LocalDecls) -> bool{
        // TODO: maybe clean up bb?
        let mut flag = false;
        // if fact[bb] is none, initialize one
        self.lock_set_facts.entry((def_id, bb_index)).or_insert_with(|| {
            flag = true;
            FxHashSet::default()
        });
        // merge the pres
        let temp = self.lock_set_facts[&(def_id, bb_index)].clone();
        for pre in bbs.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        // traverse the bb's statements
        let current_bb_data = &bbs[BasicBlock::from(bb_index)];
        current_bb_data.statements.iter().for_each(|statement| self.visit_statement(def_id, bb_index, statement, decls));
        flag |= temp.eq(&self.lock_set_facts[&(def_id, bb_index)]);
        flag
    }

    pub fn merge(&mut self, pre: &BasicBlock, def_id: DefId, bb_index: usize){
        // merge the lock set
        self.lock_set_facts.entry((def_id, pre.as_usize())).or_insert_with(|| {
            FxHashSet::default()
        }); 
        let pre_fact = self.lock_set_facts[&(def_id, pre.as_usize())].clone();
        self.lock_set_facts.get_mut(&(def_id, bb_index)).unwrap().extend(pre_fact);
    }

    pub fn visit_statement(&mut self, def_id: DefId, bb_index: usize, statement: &Statement, decls: &LocalDecls){
        match &statement.kind{
            rustc_middle::mir::StatementKind::Assign(ref assign) => {
                self.visit_assign(&def_id, &assign.0, &assign.1);
            },
            rustc_middle::mir::StatementKind::FakeRead(_) => (),
            rustc_middle::mir::StatementKind::SetDiscriminant { .. } => (),
            rustc_middle::mir::StatementKind::Deinit(_) => (),
            rustc_middle::mir::StatementKind::StorageLive(local) => {
                if self.local_locks.contains_key(&local.as_usize()) && is_lock(&decls[local.clone()].ty){ // is a lock
                    if let Some(lock) = self.local_locks.get_mut(&local.as_usize()) {
                        lock.set_live();
                    }
                    // println!("after set alive: {:?}", self.local_locks);
                }
            },
            rustc_middle::mir::StatementKind::StorageDead(local) => {
                if self.local_locks.contains_key(&local.as_usize()) && is_lock(&decls[local.clone()].ty){ // is a lock
                    if let Some(lock) = self.local_locks.get_mut(&local.as_usize()) {
                        lock.set_dead();
                    }
                    // kill the lock
                    // refactor
                    // TODO: 这里有问题，应该是MutexGuard dead的时候去掉锁
                    let to_remove = self.lock_set_facts.get(&(def_id, bb_index)).unwrap().iter().find(|lock| {
                        match (*lock){
                            Lock::Mutex(m) => m.name == local.as_usize().to_string(),
                            Lock::RwLock(m) => m.name == local.as_usize().to_string(),
                        }
                    }).cloned();
                    if let Some(lock) = to_remove{
                        self.lock_set_facts.get_mut(&(def_id, bb_index)).unwrap().remove(&lock);
                    }
                    // println!("after set dead: {:?}", self.local_locks);
                }
            },
            rustc_middle::mir::StatementKind::Retag(_, _) => (),
            rustc_middle::mir::StatementKind::PlaceMention(_) => (),
            rustc_middle::mir::StatementKind::AscribeUserType(_, _) => {

            },
            rustc_middle::mir::StatementKind::Coverage(_) => (),
            rustc_middle::mir::StatementKind::Intrinsic(_) => (),
            rustc_middle::mir::StatementKind::ConstEvalCounter => (),
            rustc_middle::mir::StatementKind::Nop => (),
        }
    }

    pub fn visit_assign(&mut self, def_id: &DefId, lhs: &Place, rhs: &Rvalue){
        let alias_map = self.alias_flow_graph.get_mut(def_id).unwrap();
        // resolve lhs
        let left = resolve_project(lhs, alias_map);
        // resolve rhs
        match rhs{
            Rvalue::Use(op) => {
                match op{
                    mir::Operand::Copy(p) => {
                        let right = resolve_project(p, alias_map);
                        // only if strict ssa can we insert directly
                        // if Copy, just clone the right to left
                        let mut lNode = alias_map.get(&right).unwrap().clone();
                        lNode.set_index(left);
                        alias_map.insert(left, lNode);
                    },
                    mir::Operand::Move(p) => {
                        let right = resolve_project(p, alias_map);
                        // only if strict ssa can we insert directly
                        // if Move, just clone the right to left
                        // we can do this because those referents that right points to can be cloned to left;
                        // and those references that point to right are definitely invalid after right is moved.
                        let mut lNode = alias_map.get(&right).unwrap().clone();
                        lNode.set_index(left);
                        alias_map.insert(left, lNode);
                    },
                    mir::Operand::Constant(_) => (),
                }
            },
            Rvalue::AddressOf(_, p) => {
                
            },
            Rvalue::Ref(_, _, p) => {
                let right = resolve_project(p, alias_map);
                // left is reference to right, so init a new ref node which points to right
                // only if strict ssa can we insert directly
                alias_map.insert(left, Node::new_ref(left, right));
            },
            Rvalue::Repeat(_, _) => todo!(),
            Rvalue::ThreadLocalRef(_) => todo!(),
            Rvalue::Len(_) => todo!(),
            Rvalue::Cast(_, _, _) => (),
            Rvalue::Discriminant(_) => todo!(),
            Rvalue::Aggregate(_, _) => (),
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

pub fn resolve_project<'tcx>(p: &Place, alias_map: &FxHashMap<usize, Node<'tcx>>) -> usize {
    let mut current = p.local.as_usize();
    for projection in p.projection{
        match &projection{
            mir::ProjectionElem::Deref => {
                let node = alias_map.get(&current).unwrap();
                match node{
                    Node::Owned(_) => panic!("You cannot deref an owned variable!"),
                    Node::Ref(r) => {
                        current = r.point_to;
                    },
                }
            },
            mir::ProjectionElem::Field(_, _) => (),
            mir::ProjectionElem::Index(_) => todo!(),
            mir::ProjectionElem::ConstantIndex { offset, min_length, from_end } => todo!(),
            mir::ProjectionElem::Subslice { from, to, from_end } => todo!(),
            mir::ProjectionElem::Downcast(_, _) => todo!(),
            mir::ProjectionElem::OpaqueCast(_) => todo!(),
            mir::ProjectionElem::Subtype(_) => todo!(),
        }
    }
    current
}