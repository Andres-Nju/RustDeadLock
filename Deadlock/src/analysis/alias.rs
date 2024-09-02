use std::{
    cell::RefCell, hash::{Hash, Hasher}, rc::Rc
};

use rustc_hash::FxHashSet;


#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct LockObject {
    pub id: usize,
}

impl LockObject {
    pub fn new(id: usize) -> Rc<Self> {
        Rc::new(LockObject { id })
    }
}

#[derive(Debug)]
pub struct AliasSet {
    pub variables: RefCell<FxHashSet<Rc<VariableNode>>>,  
}

#[derive(Debug)]
pub struct VariableNode {
    pub index: usize,
    pub alias_set: Rc<AliasSet>, 
    possible_locks: Rc<RefCell<FxHashSet<Rc<LockObject>>>>,
}


impl PartialEq for VariableNode {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for VariableNode {}

impl Hash for VariableNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl AliasSet {
    pub fn new() -> Rc<Self> {
        Rc::new(AliasSet {
            variables: RefCell::new(FxHashSet::default()),
        })
    }

    pub fn add_variable(&self, var: Rc<VariableNode>) {
        self.variables.borrow_mut().insert(var);
    }

    pub fn merge(self: &Rc<Self>, other: &Rc<AliasSet>) {
        let mut self_vars = self.variables.borrow_mut();
        let other_vars = other.variables.borrow();

        for var in other_vars.iter() {
            self_vars.insert(Rc::clone(&var));
        }
    }
}

impl VariableNode {
    pub fn new(index: usize) -> Rc<Self> {
        Rc::new(VariableNode {
            index,
            alias_set: AliasSet::new(),
            possible_locks: Rc::new(RefCell::new(FxHashSet::default())),
        })
    }

    pub fn merge_alias_set(&self, other: &Rc<VariableNode>){
        self.alias_set.merge(&other.alias_set);
    }

    pub fn strong_update_possible_locks(&self, other: &Rc<VariableNode>) {
        *self.possible_locks.borrow_mut() = other.possible_locks.borrow().clone();
    }

    pub fn add_possible_lock(&self, lock: Rc<LockObject>) {
        self.possible_locks.borrow_mut().insert(lock);
    }

    pub fn get_possible_locks(&self) -> Rc<RefCell<FxHashSet<Rc<LockObject>>>> {
        self.possible_locks.clone()
    }
}