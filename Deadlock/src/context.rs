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

/// 分析需要的上下文
#[derive(Clone)]
pub struct Context<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub call_graph: CallGraph<'tcx>,
}

impl<'tcx> Context<'tcx> {
    /// 构造上下文
    pub fn new(options: &Options, tcx: TyCtxt<'tcx>) -> Self {
        let mut entry_func: Option<DefId> = None;

        let mut all_funcs = FxHashMap::default();
        for each_mir in tcx.mir_keys(()) {
            let def_id = each_mir.to_def_id();
            match tcx.def_kind(def_id) {
                DefKind::Fn | DefKind::AssocFn => {
                    let name = tcx.item_name(def_id);
                    if !all_funcs.contains_key(&def_id) {
                        // if name.to_string() == options.entry_func {
                        //     entry_func = Some(def_id);
                        // }
                        all_funcs.insert(def_id, name);
                    }
                }
                _ => {}
            }
        }

        let entry_func = match entry_func {
            Some(did) => did,
            None => {
                panic!("No entry function.")
            }
        };

        Self {
            options: options.clone(),
            tcx,
            entry_func,
            all_funcs,
            //stmts: StmtTable::default(),
            //variables: VarTable::default(),
        }
    }
}
