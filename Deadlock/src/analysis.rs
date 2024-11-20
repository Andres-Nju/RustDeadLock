use std::{borrow::Borrow, fmt::format, rc::Rc, thread::current, usize};

use alias::{graph::AliasGraph, node::EdgeLabel};

use fact::VecFact;
use itertools::Itertools;
use lockgraph::LockGraph;
use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use lock::{Lock, LockFact, LockSummary};
use rustc_hir::{
    def_id::DefId,
    definitions::{DefPath, DefPathData},
};
use rustc_middle::{
    mir::{
        self, BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, Local, LocalDecls, Place,
        Rvalue, Successors, TerminatorKind, VarDebugInfoContents,
    },
    ty::{self, Ty, TyCtxt, TyKind},
};

use rustc_middle::mir::{Body, Location, Statement, Terminator};
use tools::{is_guard, is_mutex_method, is_smart_pointer};

use crate::context::MyTcx;

pub mod alias;
pub mod callgraph;
pub mod fact;
pub mod lock;
pub mod lockgraph;
pub mod tools;
mod visitor;
pub struct LockSetAnalysis<'a, 'tcx> {
    my_tcx: &'a mut MyTcx<'tcx>,

    current_func: Option<DefId>,
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<DefId, FxHashMap<usize, LockSummary>>,

    // intra-analysis data
    // record all variable debug info in current function body
    // TODO: shadow nested scope
    var_debug_info: FxHashMap<usize, String>,

    // lock graph
    pub lock_graph: LockGraph,
}

impl<'a, 'tcx> LockSetAnalysis<'a, 'tcx> {
    pub fn new(my_tcx: &'a mut MyTcx<'tcx>) -> Self {
        Self {
            my_tcx,
            current_func: None,
            lock_set_facts: FxHashMap::default(),
            var_debug_info: FxHashMap::default(),
            lock_graph: LockGraph::new(),
        }
    }

    pub fn run_analysis(&mut self) {
        self.before_run();

        self.intra_procedural_analysis();
        self.inter_procedural_analysis();

        self.after_run();
    }

    fn before_run(&mut self) {
        tracing::info!("Start alias analysis");
    }

    fn after_run(&self) {
        tracing::info!("Finish lock analysis");
    }

    pub fn print_lock_set_facts(&self) {
        for (def_id, summaries) in &self.lock_set_facts {
            println!("DefId: {:?}", def_id);
            let mut keys: Vec<usize> = summaries.keys().cloned().collect();
            keys.sort();
            for index in keys {
                let summary = summaries.get(&index).unwrap();
                println!("  Index: {}", index);
                for (i, lock_set) in summary.iter().enumerate() {
                    println!("    Lock Summary {:?}:", i);
                    for lock_fact in lock_set {
                        let is_acq;
                        if lock_fact.is_acquisition {
                            is_acq = "+";
                        } else {
                            is_acq = "-";
                        }
                        println!(
                            "      Lock: {:?}, Location: {:?}, {:?}, {:?}",
                            lock_fact.lock, lock_fact.s_location, is_acq, lock_fact.state as i32
                        );
                    }
                }
            }
        }
    }

    fn intra_procedural_analysis(&mut self) {
        // traverse the functions in a reversed topo order
        for def_id in self.my_tcx.call_graph.topo.clone() {
            if self.my_tcx.tcx.is_mir_available(def_id) {
                // each function is analyzed only once
                let body = self.my_tcx.tcx.optimized_mir(def_id);
                if def_id.is_local() && self.lock_set_facts.get(&def_id) == None {
                    // println!(
                    //     "Now analyze function {:?}, {:?}",
                    //     body.span,
                    //     self.my_tcx.tcx.def_path_str(def_id)
                    // );
                    // only analyze functions defined in current crate
                    // FIXME: closure?
                    self.lock_set_facts
                        .entry(def_id.clone())
                        .or_insert(FxHashMap::default());
                    self.visit_body(def_id, body);
                }
            }
        }
    }

    fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>) {
        self.current_func = Some(def_id);
        self.init_func(body);
        // FIXME: redundant clone
        for current_bb_index in self.my_tcx.control_flow_graph[&def_id].clone() {
            // println!("bb {:?} now under lock analysis ", current_bb_index);
            self.visit_bb(current_bb_index.as_usize(), body);
        }
    }
    fn init_func(&mut self, body: &Body) {
        let def_id = self.current_func.unwrap().clone();
        let lock_set_facts = self.lock_set_facts.get_mut(&def_id).unwrap();
        for bb_index in self.my_tcx.control_flow_graph.get(&def_id).unwrap().clone() {
            lock_set_facts.entry(bb_index.as_usize()).or_insert(vec![]);
        }
    }

    fn visit_bb(&mut self, bb_index: usize, body: &Body<'tcx>) {
        // let def_id = self.current_func.unwrap().clone();
        // merge the pres
        for pre in body
            .basic_blocks
            .predecessors()
            .get(BasicBlock::from_usize(bb_index))
            .unwrap()
        {
            // refactor the lock_set_facts access
            self.merge(pre, bb_index);
        }
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];
        // process the terminator
        self.visit_terminator(bb_index, &data.terminator().kind, body);
    }

    pub fn merge(&mut self, pre: &BasicBlock, bb_index: usize) {
        let def_id = self.current_func.unwrap().clone();
        // merge the lock set
        let pre_lock_fact = self.lock_set_facts[&def_id][&pre.as_usize()].clone();
        self.lock_set_facts
            .get_mut(&def_id)
            .unwrap()
            .get_mut(&bb_index)
            .unwrap()
            .clear();
        self.lock_set_facts
            .get_mut(&def_id)
            .unwrap()
            .get_mut(&bb_index)
            .unwrap()
            .meet(&pre_lock_fact);
    }

    fn visit_terminator(
        &mut self,
        bb_index: usize,
        terminator_kind: &TerminatorKind,
        body: &Body<'tcx>,
    ) {
        let def_id = self.current_func.unwrap().clone();
        let alias_graph = self
            .my_tcx
            .alias_graph
            .get_mut(&self.current_func.unwrap())
            .unwrap();
        match terminator_kind {
            rustc_middle::mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                match func {
                    mir::Operand::Constant(constant) => {
                        match constant.ty().kind() {
                            rustc_type_ir::TyKind::FnDef(fn_id, _) => {
                                // _* = func(args) -> [return: bb*, unwind: bb*] @ Call: FnDid: *
                                if fn_id.is_local() {
                                    // process local functions in the same crate
                                    let callee_size =
                                        self.my_tcx.control_flow_graph.get(fn_id).unwrap().len();
                                    let callee_summary_clone = self
                                        .lock_set_facts
                                        .get_mut(&def_id)
                                        .unwrap()
                                        .get(&callee_size)
                                        .unwrap()
                                        .clone();

                                    // for each o of all the acquired but not released locks in caller
                                    // and for each o' in the callee's summary,
                                    let current_set_facts = self.lock_set_facts
                                    .get_mut(&def_id)
                                    .unwrap()
                                    .get_mut(&bb_index)
                                    .unwrap();
                                    for lock_set_fact in current_set_facts.iter_mut(){
                                        for lock_fact in lock_set_fact.iter(){
                                            let caller_lock = lock_fact.
                                            for 
                                        }
                                    }
                                    // 1. if o' is acquired in the callee, add lock graph edge o -> o'

                                    // 2. if o' is released in the callee, and o' is alias to o
                                    // this means o' is released in the callee, so change the state from 0 to 1

                                    // 3. clone the lock summary from callee
                                    
                                        .extend(callee_summary_clone);
                                    return;
                                }
                                // now process special funcs
                                let def_path = self.my_tcx.tcx.def_path(fn_id.clone());
                                let def_path_str = self.my_tcx.tcx.def_path_str(fn_id);
                                if let DefPathData::ValueNs(name) =
                                    &def_path.data[def_path.data.len() - 1].data
                                {
                                    if is_mutex_method(&def_path_str) {
                                        if name.as_str() == "lock" {
                                            assert_eq!(1, args.len());
                                            match &args[0].node {
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) | mir::Operand::Move(p) => {
                                                    let guard = alias_graph
                                                        .resolve_project(&def_id, destination);
                                                    let mut new_lock_set_fact =
                                                        FxHashSet::default();
                                                    unsafe {
                                                        for lock_node in (*(*(*guard)
                                                            .get_out_vertex(&EdgeLabel::Guard)
                                                            .unwrap())
                                                        .alias_set)
                                                            .iter()
                                                        {
                                                            let new_lock = Lock::new(
                                                                (**lock_node).id.def_id.clone(),
                                                                (**lock_node).id.index,
                                                            );
                                                            for lock_set_fact in self
                                                                .lock_set_facts
                                                                .get_mut(&def_id)
                                                                .unwrap()
                                                                .entry(bb_index)
                                                                .or_insert(vec![])
                                                                .iter_mut()
                                                            {
                                                                for old_lock_fact in
                                                                    lock_set_fact.iter()
                                                                {
                                                                    if old_lock_fact.is_acquisition
                                                                        == true
                                                                        && old_lock_fact.state
                                                                            == false
                                                                    {
                                                                        let old_lock =
                                                                            old_lock_fact
                                                                                .lock
                                                                                .clone();
                                                                        self.lock_graph.add_edge(
                                                                            old_lock,
                                                                            new_lock.clone(),
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                            let new_lock_fact = LockFact {
                                                                is_acquisition: true,
                                                                state: false,
                                                                s_location: (
                                                                    def_id.clone(),
                                                                    body.terminator_loc(
                                                                        BasicBlock::from_usize(
                                                                            bb_index,
                                                                        ),
                                                                    ),
                                                                ),
                                                                lock: new_lock.clone(),
                                                            };
                                                            new_lock_set_fact.insert(new_lock_fact);
                                                        }
                                                    }
                                                    self.lock_set_facts
                                                        .get_mut(&def_id)
                                                        .unwrap()
                                                        .get_mut(&bb_index)
                                                        .unwrap()
                                                        .push(new_lock_set_fact);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            // maybe problematic
                            rustc_type_ir::TyKind::FnPtr(_) => panic!("TODO: FnPtr"),
                            rustc_type_ir::TyKind::Closure(_, _) => panic!("TODO: closure"),
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            rustc_middle::mir::TerminatorKind::Drop { place, .. } => {
                let dropped = alias_graph.resolve_project(&def_id, place);
                unsafe {
                    if let Some(lock) = (*dropped).get_out_vertex(&EdgeLabel::Guard) {
                        let alias_locks = (*lock).get_alias_set();
                        if (*alias_locks).len() == 1 {
                            // if the variable points to more than one locks, skip it
                            let lock_id = (**(*alias_locks).iter().next().unwrap()).id.clone();
                            let lock = Lock::new(lock_id.def_id, lock_id.index);
                            let mut flag = false;
                            let mut new_lock_set_fact = FxHashSet::default();
                            for lock_fact_set in self
                                .lock_set_facts
                                .get_mut(&def_id)
                                .unwrap()
                                .get_mut(&bb_index)
                                .unwrap()
                                .iter_mut()
                            {
                                for lock_fact in lock_fact_set.clone().into_iter() {
                                    if lock_fact.is_acquisition == true
                                        && lock_fact.lock == lock
                                        && lock_fact.state == false
                                    {
                                        flag = true;
                                        if let Some(mut u) = lock_fact_set.take(&lock_fact) {
                                            u.state = true;
                                            lock_fact_set.insert(u);
                                        }
                                        let new_fact = LockFact {
                                            is_acquisition: false,
                                            state: true,
                                            s_location: (
                                                def_id.clone(),
                                                body.terminator_loc(BasicBlock::from_usize(
                                                    bb_index,
                                                )),
                                            ),
                                            lock: lock.clone(),
                                        };
                                        new_lock_set_fact.insert(new_fact);
                                    }
                                }
                            }
                            if !flag {
                                let new_fact = LockFact {
                                    is_acquisition: false,
                                    state: false,
                                    s_location: (
                                        def_id.clone(),
                                        body.terminator_loc(BasicBlock::from_usize(bb_index)),
                                    ),
                                    lock: lock.clone(),
                                };
                                new_lock_set_fact.insert(new_fact);
                            }
                            self.lock_set_facts
                                .get_mut(&def_id)
                                .unwrap()
                                .get_mut(&bb_index)
                                .unwrap()
                                .push(new_lock_set_fact);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn inter_procedural_analysis(&mut self) {}
}
