use rustc_hash::FxHashSet;

pub struct Mutex{
    pub name: String,
}

pub struct RLock{
    pub name: String,
}

pub struct Wlock{
    pub name: String,
}

pub enum Lock{
    Mutex(Mutex),
    RLock(RLock),
    WLock(Wlock),
}

pub type LockSetFact = FxHashSet<Lock>;