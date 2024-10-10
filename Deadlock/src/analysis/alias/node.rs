use std::{cell::{Ref, RefCell, RefMut}, fmt::Debug, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;


pub struct AliasGraphNode{
    id: GraphNodeId,
    name: Option<String>,

    alias_set: FxHashSet<Rc<AliasGraphNode>>,
    
    out_labels: FxHashSet<Rc<EdgeLabel>>,
    in_labels: FxHashSet<Rc<EdgeLabel>>,

    /// target nodes pointed by this node
    successors: FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>,
    /// source nodes pointing to this node
    predecessors: FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>,
}

impl Debug for AliasGraphNode{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AliasGraphNode").field("id", &self.id).field("name", &self.name).finish()
    }
}
impl AliasGraphNode{
    pub fn new(id: GraphNodeId, name: Option<String>) -> Rc<Self>{
        Rc::new(AliasGraphNode{
            id,
            name,
            alias_set: FxHashSet::default(),
            in_labels: FxHashSet::default(),
            out_labels: FxHashSet::default(),
            successors: FxHashMap::default(),
            predecessors: FxHashMap::default(),
        })
    }

    pub fn get_alias_set(&self) -> &FxHashSet<Rc<AliasGraphNode>> {
        &self.alias_set
    }

    pub fn get_alias_set_mut(&mut self) -> &mut FxHashSet<Rc<AliasGraphNode>> {
        &mut self.alias_set
    }

    // pub fn get_successors(&self) -> Ref<FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>> {
    //     self.successors.borrow()
    // }

    // pub fn get_successors_mut(&self) -> RefMut<FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>> {
    //     self.successors.borrow_mut()
    // }

    // pub fn get_predecessors(&self) -> Ref<FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>> {
    //     self.predecessors.borrow()
    // }

    // pub fn get_predecessors_mut(&self) -> RefMut<FxHashMap<EdgeLabel, RefCell<FxHashSet<Rc<AliasGraphNode>>>>> {
    //     self.predecessors.borrow_mut()
    // }

    pub fn get_successors_by_label(&self, label: &EdgeLabel) -> Option<FxHashSet<Rc<AliasGraphNode>>> {
        self.successors.get(label).map(|set_refcell| {
            set_refcell.borrow().clone()
        })
    }

    pub fn get_predecessors_by_label(&self, label: &EdgeLabel) -> Option<FxHashSet<Rc<AliasGraphNode>>> {
        self.predecessors.get(label).map(|set_refcell| {
            set_refcell.borrow().clone()
        })
    }

}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GraphNodeId{
    def_id: DefId,
    index: usize,
}

impl GraphNodeId{
    pub fn new(def_id: DefId, index: usize) -> Self{
        GraphNodeId{
            def_id,
            index,
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum EdgeLabel{
    Deref,
    Guard,
    // todo: field, array access
}
