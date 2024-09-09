use std::{
    any::Any, cell::RefCell, hash::{Hash, Hasher}, rc::Rc
};
use std::fmt;
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::{DefId, LocalDefId};
pub type AliasFact = FxHashMap<usize, (Rc<VariableNode>, Rc<AliasSet>)>;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct LockObject {
    pub id: usize,
}

impl LockObject {
    pub fn new(id: usize) -> Rc<Self> {
        Rc::new(LockObject { id })
    }

    pub fn id(&self) -> usize{
        self.id
    }
}

impl fmt::Debug for AliasSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indices: Vec<_> = self.variables.borrow().iter().map(|node| format!("{:?}::{:?}", node.def_id.index.as_usize(), node.index)).collect();
        f.debug_struct("AliasSet")
         .field("variables", &indices)
         .finish()
    }
}

pub struct AliasSet {
    pub variables: RefCell<FxHashSet<Rc<VariableNode>>>,  
}

impl fmt::Debug for VariableNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // let lock_ids: Vec<_> = self.possible_locks.borrow().iter().map(|lock| (lock.def_id, lock.id)).collect();
        f.debug_struct("VariableNode")
         .field("index", &self.index)
        //  .field("alias_set", &self.alias_set)
        //  .field("possible_locks", &lock_ids)
         .finish()
    }
}

impl PartialEq for AliasSet{
    fn eq(&self, other: &Self) -> bool{
        self.variables == other.variables
    }
}

pub struct VariableNode {
    pub def_id: DefId,
    pub index: usize,
    // pub alias_set: Rc<AliasSet>, 
    // possible_locks: Rc<RefCell<FxHashSet<Rc<LockObject>>>>,
}


impl PartialEq for VariableNode {
    fn eq(&self, other: &Self) -> bool {
        self.def_id == other.def_id && self.index == other.index
    }
}

impl Eq for VariableNode {}

impl Hash for VariableNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.def_id.hash(state);
        self.index.hash(state);
    }
}

impl AliasSet {
    pub fn new() -> Rc<Self> {
        Rc::new(AliasSet {
            variables: RefCell::new(FxHashSet::default()),
        })
    }

    pub fn new_self(var: Rc<VariableNode>) -> Rc<Self> {
        let set = Rc::new(AliasSet {
            variables: RefCell::new(FxHashSet::default()),
        });
        set.add_variable(var);
        set
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
    pub fn new(def_id: DefId, index: usize) -> Rc<Self> {
        let node = Rc::new(VariableNode {
            def_id,
            index,
            // alias_set: AliasSet::new(),
            // possible_locks: Rc::new(RefCell::new(FxHashSet::default())),
        });
        // node.alias_set.add_variable(node.clone());
        node
    }
    // pub fn merge_alias_set(&self, other: &Rc<VariableNode>){
    //     self.alias_set.merge(&other.alias_set);
    // }

    // pub fn strong_update_possible_locks(&self, other: &Rc<VariableNode>) {
    //     *self.possible_locks.borrow_mut() = other.possible_locks.borrow().clone();
    // }

    // pub fn add_possible_lock(&self, lock: Rc<LockObject>) {
    //     self.possible_locks.borrow_mut().insert(lock);
    // }

    // pub fn get_possible_locks(&self) -> Rc<RefCell<FxHashSet<Rc<LockObject>>>> {
    //     self.possible_locks.clone()
    // }
}

#[cfg(test)]
mod tests{
    use rustc_index::Idx;
    use super::*;

    #[test]
    fn compare_alias_set() {
        let alias_set_1 = AliasSet::new();
        let alias_set_2 = AliasSet::new();
        assert_eq!(alias_set_1, alias_set_2);
        let node_1 = VariableNode::new(DefId::from(LocalDefId::new(10)), 1);
        let node_2 = VariableNode::new(DefId::from(LocalDefId::new(10)), 1);
        alias_set_1.add_variable(node_1);
        alias_set_2.add_variable(node_2);
        assert_eq!(alias_set_1, alias_set_2);
    }
}