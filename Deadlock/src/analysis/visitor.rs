use rustc_middle::{
    mir::{
        visit::MutVisitor,
    },
    ty::{Ty, TyCtxt}
};

pub struct IntraVisitor<'tcx> {
    tcx: TyCtxt<'tcx>,
    
}