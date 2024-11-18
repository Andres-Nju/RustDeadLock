//! 这个模块跟分析上下文有关，维护一些在分析中需要的全局变量
//!
//!

use rustc_hash::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

use crate::{
    // model::{StmtTable, VarTable},
    analysis::callgraph::CallGraph,
    option::Options,
};

// #[derive(Clone)]
pub struct MyTcx<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub call_graph: CallGraph<'tcx>,
}

impl<'tcx> MyTcx<'tcx> {
    /// 构造上下文
    pub fn new(options: &Options, tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            call_graph: CallGraph::default(),
        }
    }
}
