//! 这个模块跟分析上下文有关，维护一些在分析中需要的全局变量
//!
//!

use rustc_hash::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId};
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;

use crate::{
    // model::{StmtTable, VarTable},
    option::Options,
};

/// 分析需要的上下文
#[derive(Clone)]
pub struct Context<'tcx> {
    /// 分析选项，贯穿始终
    pub options: Options,
    /// Rust编译器的核心结构！！！
    pub tcx: TyCtxt<'tcx>,
    /// 分析的入口函数
    pub entry_func: DefId,
    /// 存储所有的函数，主要是为了选项 show....
    pub all_funcs: FxHashMap<DefId, Symbol>,
    // FIXME: 添加更多成员为分析服务
    //pub(crate) stmts: StmtTable<'tcx>,
    //pub(crate) variables: VarTable<'tcx>,
}

impl<'tcx> Context<'tcx> {
    /// 构造上下文
    pub fn new(options: &Options, tcx: TyCtxt<'tcx>) -> Self {
        let mut entry_func: Option<DefId> = None;

        let mut all_funcs = FxHashMap::default();
        let hir_krate = tcx.hir();
        for item_id in hir_krate.items() {
            let local_did = item_id.owner_id.def_id;
            match tcx.def_kind(local_did) {
                DefKind::Fn | DefKind::AssocFn => {
                    let did = local_did.to_def_id();
                    let name = tcx.item_name(did);
                    if !all_funcs.contains_key(&did) {
                        if name.to_string() == options.entry_func {
                            entry_func = Some(local_did.to_def_id());
                        }
                        all_funcs.insert(did, name);
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
