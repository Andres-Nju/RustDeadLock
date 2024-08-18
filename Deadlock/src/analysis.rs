use rustc_hash::FxHashMap;

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId};
use rustc_middle::ty::{self, TyCtxt};
use lock::LockSetFact;
use rustc_middle::mir::{
    Location,
    Body,
    Statement,
    Terminator,
};

mod visitor;
pub mod callgraph;
pub mod lock;

pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    // call_graph: CallGraph<'tcx>,
    // a DefId + location pair determines a statement
    lock_set_facts: FxHashMap<(DefId, Location), LockSetFact>,
}

impl<'tcx> LockSetAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>) -> Self{
        Self{
            tcx,
            // call_graph,
            lock_set_facts: FxHashMap::default(),
        }
    }

    pub fn run_analysis(&mut self){
        for mir_key in self.tcx.mir_keys(()){
            let def_id = mir_key.to_def_id();
            let body = self.tcx.instance_mir(ty::InstanceDef::Item(def_id));
            self.intra_lock_set_analysis(def_id, body);
        }
    }

    pub fn intra_lock_set_analysis(&mut self, def_id: DefId, body: &Body){
        let work_list: Vec<usize> = (0..body.basic_blocks.len()).collect();
        println!("bb number: {}", work_list.len());
    }
}

