use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use rustc_hash::{FxHashMap, FxHashSet};

use super::alias::LockObject;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Mutex {}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RwLock{
    pub is_write: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Lock{
    Mutex(Mutex),
    RwLock(RwLock),
}

#[derive(Debug, Clone)]
pub struct LockGuard{
    pub possible_locks: Rc<RefCell<FxHashSet<Rc<LockObject>>>>,
}

impl Mutex {
    pub fn new() -> Self {
        Self {}
    }
}

impl RwLock {
    pub fn new(is_write: bool) -> Self {
        Self {
            is_write
        }
    }

}

impl Lock{
    pub fn new_mutex() -> Self{
        Lock::Mutex(Mutex::new())
    }

    pub fn new_rwlock(is_write: bool) -> Self{
        Lock::RwLock(RwLock::new(is_write))
    }
}

pub type LockSetFact = FxHashMap<usize, LockGuard>;

impl LockGuard{
    pub fn new(possible_locks: Rc<RefCell<FxHashSet<Rc<LockObject>>>>) -> Self {
        LockGuard{
            possible_locks,
        }
    }
}