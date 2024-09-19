use std::{rc::Rc, cell::RefCell};

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;


pub struct AliasGraphNode{
    id: NodeId,
    name: Option<String>,

    alias_set: RefCell<FxHashSet<Rc<AliasGraphNode>>>,
    
    /// target nodes pointed by this node
    successors: FxHashMap<EdgeLabel, FxHashSet<Rc<AliasGraphNode>>>,
    /// source nodes pointing to this node
    predecessors: FxHashMap<EdgeLabel, FxHashSet<Rc<AliasGraphNode>>>,
}

impl AliasGraphNode{
    
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct NodeId{
    def_id: DefId,
    index: usize,
}


#[derive(Debug, Hash, PartialEq, Eq)]
enum EdgeLabel{
    Deref,
    Guard,
    // todo: field, array access
}