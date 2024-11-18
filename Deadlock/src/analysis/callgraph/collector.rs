use rustc_hash::FxHashSet;
use rustc_hir::{def_id::DefId, intravisit::Visitor, BodyId, HirId, ItemKind};
use rustc_middle::ty::{self, TyCtxt};

pub struct FnCollector {
    pub entry: Option<DefId>,
    fn_set: FxHashSet<DefId>,
}

impl<'tcx> FnCollector {
    pub fn new() -> Self {
        FnCollector {
            entry: None,
            fn_set: FxHashSet::default(),
        }
    }

    pub fn functions(&self) -> FxHashSet<DefId> {
        self.fn_set.clone()
    }
}
