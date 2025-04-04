use rustc_hash::FxHashSet;
use rustc_hir::def_id::DefId;
use rustc_middle::{
    mir::{self, Local, Place},
    ty::{self, Ty},
};

use super::LockSetAnalysis;

/// whether a type is lock
pub fn is_lock(ty: &Ty) -> bool {
    // TODO: better logic
    let ty = format!("{:?}", ty);
    return ty.contains("Mutex") || ty.contains("Rwlock"); // TODO: RwLock
}

pub fn is_primitive<'tcx>(ty: &Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) => true,
        ty::Array(ref t, _) => is_primitive(t),
        ty::Adt(_, ref args) => {
            for t in args.types() {
                if !is_primitive(&t) {
                    return false;
                }
            }
            true
        }
        ty::Tuple(ref tys) => {
            for t in tys.iter() {
                if !is_primitive(&t) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

pub fn is_mutex_method(def_path: &String) -> bool {
    def_path.starts_with("std::sync::Mutex")
}

pub fn is_smart_pointer(def_path: &String) -> bool {
    def_path.starts_with("std::sync::Arc")
}

pub fn is_guard(ty: &Ty) -> bool {
    format!("{:?}", ty).starts_with("std::sync::MutexGuard")
}

impl<'a, 'tcx> LockSetAnalysis<'a, 'tcx> {
    pub fn get_ty(&self, def_id: &DefId, index: usize) -> Ty<'tcx> {
        self.my_tcx.tcx.optimized_mir(def_id).local_decls[Local::from_usize(index)].ty
    }
}
