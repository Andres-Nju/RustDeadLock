use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::Location;

type StatementSite = (DefId, Location);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Mutex {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RwLock{
    pub is_write: bool,
}

pub type LockSetFact = FxHashSet<LockFact>;
pub type LockSummary = Vec<LockSetFact>;


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Lock {
    pub(crate) def_id: DefId,
    pub index: usize,
}

impl Lock{
    pub fn new(def_id: DefId, index: usize) -> Self{
        Lock { 
            def_id,
            index 
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct LockFact{
    /// true: the lock is acquired
    /// false: the lock is released
    pub is_acquisition: bool,
    pub state: bool,
    pub s_location: StatementSite,
    pub lock: Lock,
}


