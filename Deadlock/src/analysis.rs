use std::{borrow::Borrow, fmt::format, rc::Rc, thread::current, usize};

use alias::{graph::AliasGraph, node::EdgeLabel};

use fact::VecFact;
use itertools::Itertools;
use lockgraph::LockGraph;
use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::{DefPath, DefPathData}};
use rustc_middle::{
    mir::{self, BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, Local, LocalDecls, Place, Rvalue, Successors, TerminatorKind, VarDebugInfoContents}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{Lock, LockFact, LockSummary};

use rustc_middle::mir::{
    Location,
    Body,
    Statement,
    Terminator,
};
use tools::{is_guard, is_mutex_method, is_smart_pointer};


mod visitor;
pub mod callgraph;
pub mod lock;
pub mod alias;
pub mod tools;
pub mod lockgraph;
pub mod fact;
pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    call_graph: CallGraph<'tcx>,
    
    // whole-program data
    // a DefId + BasicBlock's index pair determines a bb
    lock_set_facts: FxHashMap<DefId, FxHashMap<usize, LockSummary>>,

    // alias_flow_graph: record the alias relationship for each function
    alias_graph: AliasGraph,

    // the traversing order of bbs in each function
    control_flow_graph: FxHashMap<DefId, Vec<BasicBlock>>,

    // intra-analysis data
    // record all variable debug info in current function body
    // TODO: shadow nested scope
    var_debug_info: FxHashMap<usize, String>,

    // lock graph
    lock_graph: LockGraph,
}

impl<'tcx> LockSetAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>, alias_graph: AliasGraph, control_flow_graph: FxHashMap<DefId, Vec<BasicBlock>>) -> Self{
        Self{
            tcx,
            lock_set_facts: FxHashMap::default(),
            alias_graph,
            var_debug_info: FxHashMap::default(),
            call_graph,
            control_flow_graph,
            lock_graph: LockGraph::new(),
        }
    }

    pub fn run_analysis(&mut self){
        self.before_run();

        self.intra_procedural_analysis();
        self.inter_procedural_analysis();
       
        self.after_run();
    }

    fn before_run(&mut self){
        
    } 

    fn after_run(&self){
        // self.alias_graph.print_graph();
        self.print_lock_set_facts();
        println!("lock graph:\n{:?}", self.lock_graph);
        self.lock_graph.print_loops();
    }

    pub fn print_lock_set_facts(&self) {
        for (def_id, summaries) in &self.lock_set_facts {
            println!("DefId: {:?}", def_id);
            for (index, summary) in summaries {
                println!("  Index: {}", index);
                for (i, lock_set) in summary.iter().enumerate() {
                    println!("    Lock Summary {:?}:", i);
                    for lock_fact in lock_set {
                        println!("      Lock: {:?}, Acquired: {:?}, State: {:?}, Location: {:?}",
                                 lock_fact.lock,
                                 lock_fact.is_acquisition,
                                 lock_fact.state,
                                 lock_fact.s_location);
                    }
                }
            }
        }
    }

    fn intra_procedural_analysis(&mut self){
        // traverse the functions in a reversed topo order 
        for def_id in self.call_graph.topo.clone(){
            if self.tcx.is_mir_available(def_id){
                // each function is analyzed only once
                let body = self.tcx.optimized_mir(def_id);
                println!("Now analyze function {:?}, {:?}", body.span, self.tcx.def_path_str(def_id));
                if self.tcx.def_path(def_id).data.len() == 1{
                    // only analyze functions defined in current crate
                    // FIXME: closure?
                    self.lock_set_facts.entry(def_id.clone()).or_insert(FxHashMap::default());
                    self.visit_body(def_id, body);
                }      
            }
        }
    }

    fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>){
        self.init_func(&def_id, body);
        // FIXME: redundant clone
        for current_bb_index in self.control_flow_graph[&def_id].clone(){
            println!("bb {:?} now under lock analysis ", current_bb_index);
            self.visit_bb(def_id, current_bb_index.as_usize(), body);
        }
    }
    fn init_func(&mut self, def_id: &DefId, body: &Body){
        let lock_set_facts = self.lock_set_facts.get_mut(def_id).unwrap();
        for bb_index in self.control_flow_graph.get(def_id).unwrap().clone(){
            lock_set_facts.entry(bb_index.as_usize()).or_insert(vec![]);
        }
    }

    fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body<'tcx>){
        // merge the pres
        for pre in body.basic_blocks.predecessors().get(BasicBlock::from_usize(bb_index)).unwrap(){
            // refactor the lock_set_facts access
            self.merge(pre, def_id, bb_index);
        }
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];
        // process the terminator
        self.visit_terminator(&def_id, bb_index, &data.terminator().kind, body);
    }

    pub fn merge(&mut self, pre: &BasicBlock, def_id: DefId, bb_index: usize){
        // merge the lock set
        let pre_lock_fact = self.lock_set_facts[&def_id][&pre.as_usize()].clone();
        self.lock_set_facts.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().clear();
        self.lock_set_facts.get_mut(&def_id).unwrap().get_mut(&bb_index).unwrap().meet(&pre_lock_fact);     
    }

    fn visit_terminator(&mut self, def_id: &DefId, bb_index: usize, terminator_kind: &TerminatorKind, body: &Body<'tcx>){
        match terminator_kind{ 
            rustc_middle::mir::TerminatorKind::Call { func, args, destination, target, unwind, call_source, fn_span } => {
                match func{
                    mir::Operand::Constant(constant) => {
                        match constant.ty().kind(){
                            rustc_type_ir::TyKind::FnDef(fn_id, _) => {
                                // _* = func(args) -> [return: bb*, unwind: bb*] @ Call: FnDid: *
                                let def_path = self.tcx.def_path(fn_id.clone());
                                let def_path_str = self.tcx.def_path_str(fn_id);
                                if let DefPathData::ValueNs(name) = &def_path.data[def_path.data.len() - 1].data{
                                    if is_mutex_method(&def_path_str){
                                        if name.as_str() == "lock"{
                                            assert_eq!(1, args.len());
                                            match &args[0]{
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) |
                                                mir::Operand::Move(p) => {
                                                    let guard = self.alias_graph.resolve_project(def_id, destination);
                                                    let mut new_lock_set_fact = FxHashSet::default();
                                                    unsafe{
                                                        for lock_node in  (*(*(*guard).get_out_vertex(&EdgeLabel::Guard).unwrap()).alias_set).iter(){
                                                            let new_lock = Lock::new((**lock_node).id.def_id.clone(), (**lock_node).id.index);
                                                            for lock_set_fact in self.lock_set_facts.get_mut(def_id).unwrap()
                                                                .entry(bb_index).or_insert(vec![]).iter_mut()
                                                            {
                                                                for old_lock_fact in lock_set_fact.iter(){
                                                                    if old_lock_fact.is_acquisition == true && old_lock_fact.state == false{
                                                                        let old_lock = old_lock_fact.lock.clone();
                                                                        self.lock_graph.add_edge(old_lock, new_lock.clone());
                                                                    }
                                                                }
                                                            }
                                                            let new_lock_fact = LockFact{
                                                                is_acquisition: true,
                                                                state: false,
                                                                s_location: (def_id.clone(), body.terminator_loc(BasicBlock::from_usize(bb_index))),
                                                                lock: new_lock.clone(),
                                                            };
                                                            new_lock_set_fact.insert(new_lock_fact);
                                                        }
                                                    }  
                                                    self.lock_set_facts.get_mut(def_id).unwrap().get_mut(&bb_index).unwrap().push(new_lock_set_fact);
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
            rustc_middle::mir::TerminatorKind::Drop { place, .. } => {
                
            },
            _ => {}
        }
    }

    fn inter_procedural_analysis(&mut self){

    }

}
    
    
//     fn print_lock_facts(&self){
//         let mut grouped_map: FxHashMap<DefId, Vec<(usize, &Vec<FxHashSet<Rc<LockFact>>>)>> = FxHashMap::default();
//         for (def_id, value) in &self.lock_set_facts {
//             for (key_usize, value) in value{
//                 grouped_map
//                 .entry(def_id.clone())
//                 .or_insert_with(Vec::new)
//                 .push((*key_usize, value));
//             }
//         }

//         println!("Lock set facts: ");
//         for (def_id, mut vec) in grouped_map {
//             vec.sort_by_key(|k| k.0); 
//             println!("{:?}:", def_id);
//             for (key_usize, value) in vec {
//                 println!("bb {} -> {:?}", key_usize, value);
//             }
//             println!();
//         }
//     }
// }



