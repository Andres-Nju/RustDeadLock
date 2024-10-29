use std::{borrow::Borrow, fmt::format, rc::Rc, thread::current, usize};

use alias::graph::AliasGraph;
use fact::{MapFact, SetFact, VecFact};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId, definitions::{DefPath, DefPathData}};
use rustc_middle::{
    mir::{self, BasicBlock, BasicBlockData, BasicBlocks, HasLocalDecls, Local, LocalDecls, Place, Rvalue, Successors, TerminatorKind, VarDebugInfoContents}, 
    ty::{self, Ty, TyCtxt, TyKind}
};
use lock::{LockFact, LockGuard, LockSetFact, LockSummary};

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
pub mod fact;
pub mod tools;

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

    }

    fn intra_procedural_analysis(&mut self){
        // traverse the functions in a reversed topo order 
        for def_id in self.call_graph.collector.functions(){
            if self.tcx.is_mir_available(def_id){
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

    fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>){
        // FIXME: redundant clone
        for current_bb_index in self.control_flow_graph[&def_id].clone(){
            println!("bb {:?} now under alias analysis ", current_bb_index);
            self.visit_bb(def_id, current_bb_index.as_usize(), body);
        }
    }

    fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body<'tcx>){
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];
        // process the terminator
        self.visit_terminator(&def_id, bb_index, &data.terminator().kind);
    }

    fn visit_terminator(&mut self, def_id: &DefId, bb_index: usize, terminator_kind: &TerminatorKind){
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
                                                    let lock_ref = self.alias_graph.resolve_project(def_id, p);
                            
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



