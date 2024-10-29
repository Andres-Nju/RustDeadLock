use rustc_hash::FxHashSet;
use rustc_middle::ty::{self,TyCtxt};
use rustc_hir::{def_id::DefId,intravisit::Visitor,BodyId,HirId,ItemKind};

pub struct FnCollector<'tcx>{
    pub tcx: TyCtxt<'tcx>,
    pub entry: Option<DefId>,
    fn_set: FxHashSet<DefId>,
}
 
impl<'tcx> FnCollector<'tcx>{
    pub fn new(tcx: TyCtxt<'tcx>) -> Self{
        FnCollector{
            tcx,
            entry: None,
            fn_set: FxHashSet::default()
        }
    }

    pub fn collect(&mut self, tcx: TyCtxt<'tcx>) -> &FxHashSet<DefId> {
        tcx.hir().visit_all_item_likes_in_crate(self);
        &self.fn_set
    }

    pub fn functions(&self) -> FxHashSet<DefId>{
        self.fn_set.clone()
    }
}

impl<'tcx> Visitor<'tcx> for FnCollector<'tcx>{
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
        match &item.kind {
            ItemKind::Fn(_fn_sig, _generics, body_id) => {
                let def_id = self.tcx.hir().body_owner_def_id(*body_id).to_def_id();
                if self.tcx.def_path_str(def_id) == "main"{
                    self.entry = Some(def_id);
                }
                self.fn_set.insert(def_id);
            }
            _ => (),
        }
    }
}