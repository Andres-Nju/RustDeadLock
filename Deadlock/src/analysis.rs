use std::collections::HashSet;

use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

mod visitor;
pub mod callgraph;

pub struct ControlFlowAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    visited: HashSet<DefId>,
}