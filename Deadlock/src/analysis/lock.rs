use rustc_middle::ty::Ty;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Mutex{
    pub name: String,
    pub live: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RwLock{
    pub name: String,
    pub live: bool,
    pub is_write: bool,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Lock{
    Mutex(Mutex),
    RwLock(RwLock),
}


impl Mutex {
    pub fn new(name: String) -> Self {
        Self {
            name,
            live: false, 
        }
    }

    pub fn set_live(&mut self){
        self.live = true;
    }

    pub fn set_dead(&mut self){
        self.live = false;
    }
}

impl RwLock {
    pub fn new(name: String) -> Self {
        Self {
            name,
            live: false,
            is_write: false, 
        }
    }

    pub fn set_live(&mut self){
        self.live = true;
    }

    pub fn set_dead(&mut self){
        self.live = false;
    }
}

impl Lock{
    pub fn new(name: String, ty: &Ty) -> Self{
        
        let ty = format!("{:?}", ty);
        if ty.contains("Mutex") && !ty.contains("MutexGuard"){
            Lock::Mutex(Mutex::new(name))
        }
        else if ty.contains("RwLock"){
            Lock::RwLock(RwLock::new(name))
        }
        else {
            panic!("Must be a Mutex or RwLock")
        }
    }

    pub fn set_live(&mut self){
        match self{
            Lock::Mutex(lock) => { lock.set_live(); },
            Lock::RwLock(lock) => { lock.set_live(); },
        }
    }

    pub fn set_dead(&mut self){
        match self{
            Lock::Mutex(lock) => { lock.set_dead(); },
            Lock::RwLock(lock) => { lock.set_dead(); },
        }
    }
}

pub type LockSetFact = FxHashSet<Lock>;