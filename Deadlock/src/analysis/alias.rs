use std::{
    any::Any, cell::RefCell, hash::{Hash, Hasher}, rc::Rc
};
use std::fmt;
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::{def_id::{DefId, LocalDefId}, definitions::DefPathData};
use rustc_middle::{mir::{self, BasicBlock, Body, HasLocalDecls, Local, LocalDecls, Place, Rvalue, Statement, TerminatorKind}, ty::{Ty, TyCtxt}};
use rustc_span::sym::call;

use super::{callgraph::CallGraph, fact::MapFact, is_mutex_method, is_smart_pointer, resolve_project};
pub type AliasFact = FxHashMap<usize, (Rc<VariableNode>, Rc<AliasSet>)>;


pub struct AliasAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>,
    call_graph: CallGraph<'tcx>,
    // alias_flow_graph: record the alias relationship for each function
    alias_map: FxHashMap<DefId, FxHashMap<usize, AliasFact>>,

}

impl<'tcx> AliasAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>) -> Self{
        Self{
            tcx,
            call_graph,
            alias_map: FxHashMap::default(),
        }
    }

    pub fn run_analysis(&mut self){
        // traverse the functions in a reversed topo order 
        for def_id in self.call_graph.topo.clone(){
            if self.tcx.is_mir_available(def_id) && !self.alias_map.contains_key(&def_id){
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
    }

    fn init_func(&mut self, def_id: &DefId, body: &Body){
        // init the alias facts
        if self.tcx.is_mir_available(def_id) && self.tcx.def_path(def_id.clone()).data.len() == 1 {
            self.alias_map.entry(def_id.clone()).or_insert(FxHashMap::default());
            for (index, basic_block_data) in self.tcx.optimized_mir(def_id).basic_blocks.iter().enumerate(){
                if !basic_block_data.is_cleanup{
                    self.alias_map.get_mut(&def_id).unwrap().entry(index).or_insert(FxHashMap::default());
                }
            }
        }

        let alias_map = self.alias_map.get_mut(&def_id).unwrap();
        // resolve all the function arguments (parameters, actually)
        for arg in body.args_iter(){
            // TODO: the args, should be processed in inter-procedural analysis
            if is_lock(&body.local_decls[arg].ty) {
                let index = arg.as_usize();
                // create a node for the arg (parameter, actually)
                let alias_map = alias_map.get_mut(&0).unwrap();
                let var = VariableNode::new(def_id.clone(), index);
                alias_map.update(index,(var.clone(), AliasSet::new_self(var)));
            }
        }
    }

    fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>){
        self.init_func(&def_id, body);
        
        let mut work_list = vec![0];
        while !work_list.is_empty(){
            let current_bb_index = work_list.pop().expect("Elements in non-empty work_list should always be valid!");
            println!("bb {} is now under alias analysis", current_bb_index);
            if let Some(targets) = self.visit_bb(def_id, current_bb_index, body){
                work_list.extend(targets);
            }
        }
    }

    fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body<'tcx>) -> Option<Vec<usize>>{
        let mut flag = false;
        let mut gotos = vec![];
        // if fact[bb] is none, initialize one
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];

        // merge the pres
        let temp = self.alias_map[&def_id][&bb_index].clone();
        for pre in body.basic_blocks.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        // traverse the bb's statements
        data.statements.iter().for_each(|statement| self.visit_statement(def_id, bb_index, statement, body.local_decls()));
        // process the terminator
        self.visit_terminator(&def_id, bb_index, &data.terminator().kind, &mut gotos);
        flag |= temp.ne(&self.alias_map[&def_id][&bb_index]);
        if flag{
            Some(gotos)
        }
        else {
            None
        }
    }


    pub fn merge(&mut self, pre: &BasicBlock, def_id: DefId, bb_index: usize){
        // merge the alias_map
        let pre_alias_fact = self.alias_map[&def_id][&pre.as_usize()].clone();
        self.alias_map.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().meet(pre_alias_fact);
        
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
                        let right = resolve_project(p);
                        let right_var_set = &alias_map.get(&right).unwrap().1;
                        let left_var = VariableNode::new(def_id.clone(), left);
                        right_var_set.add_variable(left_var.clone());
                        alias_map.update(left, (left_var, right_var_set.clone()));
                    },
                    mir::Operand::Constant(_) => panic!("Mutex should not be constant!"),
                }
            },
            Rvalue::AddressOf(_, p) |
            Rvalue::Ref(_, _, p) => {
                let right = resolve_project(p);
                let right_var_set = &alias_map.get(&right).unwrap().1;
                let left_var = VariableNode::new(def_id.clone(), left);
                right_var_set.add_variable(left_var.clone());
                alias_map.update(left, (left_var, right_var_set.clone()));
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

    fn visit_terminator(&mut self, def_id: &DefId, bb_index: usize, terminator_kind: &TerminatorKind, gotos: &mut Vec<usize>){
        let alias_map = self.alias_map.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap();
        match terminator_kind{ // TODO: if return a lock?
            rustc_middle::mir::TerminatorKind::Drop { place, target, .. } => {
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
                                                    let left_var = VariableNode::new(def_id.clone(), left);
                                                    assert!(!alias_map.contains_key(&left));
                                                    alias_map.update(left, (left_var.clone(), AliasSet::new_self(left_var)));
                                                },
                                            }
                                        }
                                        else if name.as_str() == "lock"{
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) |
                                                mir::Operand::Move(p) => {
                                                    let right =  resolve_project(p);
                                                    let left = resolve_project(&destination);
                                                    let left_var = VariableNode::new(def_id.clone(), left);

                                                    // update alias_map
                                                    assert!(!alias_map.contains_key(&left));
                                                    alias_map.update(left, (left_var.clone(), AliasSet::new_self(left_var)));
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
                                                    let right_var_set = &alias_map.get(&right).unwrap().1;
                                                    let left_var = VariableNode::new(def_id.clone(), left);
                                                    assert!(!alias_map.contains_key(&left));
                                                    right_var_set.add_variable(left_var.clone());
                                                    alias_map.update(left, (left_var, right_var_set.clone()));
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
                                                let left_var = VariableNode::new(def_id.clone(), left);
                                                let right_var_set  = &alias_map.get(&right).unwrap().1;
                                                assert!(!alias_map.contains_key(&left));
                                                right_var_set.add_variable(left_var.clone());
                                                alias_map.update(left, (left_var, right_var_set.clone()));
                                            },
                                        }
                                    } 
                                    else if name.as_str() == "deref"{
                                        // FIXME: if the arg is not a smart pointer which wraps mutex
                                        assert_eq!(1, args.len());
                                        match &args[0]{
                                            // must be move _*
                                            mir::Operand::Constant(_) => todo!(),
                                            mir::Operand::Copy(p) |
                                            mir::Operand::Move(p) => {
                                                // right is &a
                                                let right =  resolve_project(p);
                                                let right_var_set  = &alias_map.get(&right).unwrap().1;
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(def_id.clone(), left);
                                                assert!(!alias_map.contains_key(&left));
                                                right_var_set.add_variable(left_var.clone());
                                                alias_map.update(left, (left_var, right_var_set.clone()));
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
                                                let right_var_set  = &alias_map.get(&right).unwrap().1;
                                                let left = resolve_project(&destination);
                                                let left_var = VariableNode::new(def_id.clone(), left);
                                                assert!(!alias_map.contains_key(&left));
                                                right_var_set.add_variable(left_var.clone());
                                                alias_map.update(left, (left_var, right_var_set.clone()));
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
}


pub fn consume_alias_results(alias_analysis: AliasAnalysis) -> (TyCtxt, CallGraph, FxHashMap<DefId, FxHashMap<usize, AliasFact>>) {
    let AliasAnalysis { tcx, call_graph, alias_map } = alias_analysis;
    (tcx, call_graph, alias_map)
}
/// whether a type is lock
pub fn is_lock(ty: &Ty) -> bool{
    // TODO: better logic
    let ty = format!("{:?}", ty);
    return ty.contains("Mutex") || ty.contains("Rwlock"); // TODO: RwLock
}




#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct LockObject {
    pub id: usize,
}

impl LockObject {
    pub fn new(id: usize) -> Rc<Self> {
        Rc::new(LockObject { id })
    }

    pub fn id(&self) -> usize{
        self.id
    }
}

impl fmt::Debug for AliasSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indices: Vec<_> = self.variables.borrow().iter().map(|node| format!("{:?}::{:?}", node.def_id.index.as_usize(), node.index)).collect();
        f.debug_struct("AliasSet")
         .field("variables", &indices)
         .finish()
    }
}

pub struct AliasSet {
    pub variables: RefCell<FxHashSet<Rc<VariableNode>>>,  
}

impl fmt::Debug for VariableNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VariableNode")
         .field("index", &self.index)
         .finish()
    }
}

impl PartialEq for AliasSet{
    fn eq(&self, other: &Self) -> bool{
        self.variables == other.variables
    }
}

pub struct VariableNode {
    pub def_id: DefId,
    pub index: usize,
}


impl PartialEq for VariableNode {
    fn eq(&self, other: &Self) -> bool {
        self.def_id == other.def_id && self.index == other.index
    }
}

impl Eq for VariableNode {}

impl Hash for VariableNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.def_id.hash(state);
        self.index.hash(state);
    }
}

impl AliasSet {
    pub fn new() -> Rc<Self> {
        Rc::new(AliasSet {
            variables: RefCell::new(FxHashSet::default()),
        })
    }

    pub fn new_self(var: Rc<VariableNode>) -> Rc<Self> {
        let set = Rc::new(AliasSet {
            variables: RefCell::new(FxHashSet::default()),
        });
        set.add_variable(var);
        set
    }

    pub fn add_variable(&self, var: Rc<VariableNode>) {
        self.variables.borrow_mut().insert(var);
    }

    pub fn merge(self: &Rc<Self>, other: &Rc<AliasSet>) {
        let mut self_vars = self.variables.borrow_mut();
        let other_vars = other.variables.borrow();

        for var in other_vars.iter() {
            self_vars.insert(Rc::clone(&var));
        }
    }
}

impl VariableNode {
    pub fn new(def_id: DefId, index: usize) -> Rc<Self> {
        let node = Rc::new(VariableNode {
            def_id,
            index,
        });
        node
    }
}

#[cfg(test)]
mod tests{
    use rustc_index::Idx;
    use super::*;

    #[test]
    fn compare_alias_set() {
        let alias_set_1 = AliasSet::new();
        let alias_set_2 = AliasSet::new();
        assert_eq!(alias_set_1, alias_set_2);
        let node_1 = VariableNode::new(DefId::from(LocalDefId::new(10)), 1);
        let node_2 = VariableNode::new(DefId::from(LocalDefId::new(10)), 1);
        alias_set_1.add_variable(node_1);
        alias_set_2.add_variable(node_2);
        assert_eq!(alias_set_1, alias_set_2);
    }
}