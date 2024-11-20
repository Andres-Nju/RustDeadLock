//! MyTcx for analysis
//!
//!

use rustc_hash::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::{mir::BasicBlock, ty::TyCtxt};
use rustc_span::Symbol;

use crate::{
    analysis::{alias::graph::AliasGraph, callgraph::CallGraph},
    option::Options,
};

#[derive(Clone)]
pub struct MyTcx<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub call_graph: CallGraph<'tcx>,
    pub alias_graph: FxHashMap<DefId, AliasGraph>,
    // the traversing order of bbs in each function
    pub control_flow_graph: FxHashMap<DefId, Vec<BasicBlock>>,
}

unsafe impl<'tcx> Send for MyTcx<'tcx> {}
impl<'tcx> MyTcx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            call_graph: CallGraph::new(),
            alias_graph: FxHashMap::default(),
            control_flow_graph: FxHashMap::default(),
        }
    }
}
