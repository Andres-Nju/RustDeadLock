use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Mutex {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RwLock{
    pub is_write: bool,
}


#[derive(Debug, Clone)]
pub struct LockGuard{
    pub def_id: DefId,
    pub index: usize, 
    pub possible_locks: FxHashSet<Rc<LockObject>>,
}

impl PartialEq for LockGuard{
    fn eq(&self, other: &Self) -> bool {
        self.def_id == other.def_id && self.index == other.index
    }
}

// FIXME: the Hash may be not correct
#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct LockFact{
    lock: Rc<LockObject>,
    is_acquisation: bool,
    location: usize,
    state: bool,
}

impl LockFact{
    pub fn new(lock: Rc<LockObject>, is_acquisation: bool, location: usize, state: bool) -> Rc<Self>{
        Rc::new(LockFact { 
            lock, 
            is_acquisation, 
            location, 
            state,
        })
    }
}

pub type LockSetFact = FxHashSet<Rc<LockFact>>;
pub type LockSummary = Vec<LockSetFact>;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct LockObject {
    pub def_id: DefId,
    pub id: usize,
}

impl LockObject {
    pub fn new(def_id: DefId, id: usize) -> Rc<Self> {
        Rc::new(LockObject {def_id,  id })
    }

    pub fn id(&self) -> usize{
        self.id
    }
}
