use std::{
    cell::RefCell, hash::{Hash, Hasher}, rc::Rc
};

use rustc_hash::FxHashSet;


#[derive(Debug)]
pub struct AliasSet {
    variables: RefCell<FxHashSet<Rc<VariableNode>>>,  
}

/// a chain list form:
/// e.g. for the sample code
/// fn main{
///     let a = ...;
///     let p1 = &a;
///     let p2 = &p1;
/// }
/// the alias relationship:
/// node a's AliasSet: a; a points to None
/// node p1's AliasSet: p1, p1 points to a
/// node p2's AliasSet: p2, p2 points to p1
#[derive(Debug)]
pub struct PointsTo {
    /// the reference's correspond node
    base: Rc<VariableNode>,      
    /// the node it points to                 
    next: Option<Rc<PointsTo>>,             
}

/// model of all the variables and temps in mir
#[derive(Debug)]
pub struct VariableNode {
    /// its local index
    index: usize,       
    /// its alias set, containing all the alias nodes of this node                      
    alias_set: Rc<AliasSet>,            
    /// it can be a pointer or reference, if so, record the referent
    /// FIXME: now we just assume each reference may at most point to one referent
    points_to: Option<Rc<PointsTo>>,      
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

    fn add_variable(&self, var: Rc<VariableNode>) {
        self.variables.borrow_mut().insert(var);
    }

    fn merge(self: &Rc<Self>, other: Rc<AliasSet>) {
        let mut self_vars = self.variables.borrow_mut();
        let mut other_vars = other.variables.borrow_mut();

        for mut var in other_vars.drain() {
            self_vars.insert(Rc::clone(&var));
            var.alias_set = Rc::clone(self); 
        }
    }

}

impl VariableNode {
    pub fn new(index: usize, alias_set: Rc<AliasSet>, points_to: Option<Rc<PointsTo>>) -> Rc<Self> {
        let var = Rc::new(VariableNode {
            index,
            alias_set,
            points_to,
        });
        var.alias_set.add_variable(Rc::clone(&var));
        var
    }
}
