use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;


pub struct AliasGraphNode{
    id: NodeId,
    name: Option<String>,

    alias_set: FxHashSet<Rc<AliasGraphNode>>,
    
    successors: FxHashMap<EdgeLabel, FxHashSet<Rc<AliasGraphNode>>>,

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
    // todo: field, array access
}