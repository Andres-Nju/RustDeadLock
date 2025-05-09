use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    hash::Hash,
    rc::Rc,
    sync::Mutex,
};

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;

use lazy_static::lazy_static;
lazy_static! {
    static ref LOCAL_INDEX: Mutex<usize> = Mutex::new(0);
}

pub fn next_local_id() -> usize {
    let mut id = LOCAL_INDEX.lock().unwrap();
    *id += 1;
    *id
}

pub fn get_local_id() -> usize {
    let id = LOCAL_INDEX.lock().unwrap();
    *id
}

pub fn set_local_id(num: usize) {
    let mut id = LOCAL_INDEX.lock().unwrap();
    *id = num;
}

pub struct AliasGraphNode {
    pub id: GraphNodeId,
    // pub name: Option<String>,
    pub alias_set: *mut FxHashSet<*mut AliasGraphNode>,

    pub out_labels: FxHashSet<EdgeLabel>,
    pub in_labels: FxHashSet<EdgeLabel>,

    /// target nodes pointed by this node
    pub successors: *mut FxHashMap<EdgeLabel, *mut FxHashSet<*mut AliasGraphNode>>,
    /// source nodes pointing to this node
    pub predecessors: *mut FxHashMap<EdgeLabel, *mut FxHashSet<*mut AliasGraphNode>>,
}

impl Debug for AliasGraphNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            f.debug_struct("AliasGraphNode")
                .field("id", &self.id)
                .finish()
        }
    }
}
impl AliasGraphNode {
    pub fn new(id: GraphNodeId) -> *mut AliasGraphNode {
        let alias_set = Box::new(FxHashSet::default());
        let out_labels = FxHashSet::default();
        let in_labels = FxHashSet::default();
        let successors = Box::new(FxHashMap::default());
        let predecessors = Box::new(FxHashMap::default());

        let node = Box::into_raw(Box::new(AliasGraphNode {
            id,
            // name,
            alias_set: Box::into_raw(alias_set),
            out_labels,
            in_labels,
            successors: Box::into_raw(successors),
            predecessors: Box::into_raw(predecessors),
        }));
        // add self into the alias set
        unsafe {
            (*(*node).alias_set).insert(node);
        }
        node
    }

    // // Get the node's name
    // pub fn get_name(&self) -> &Option<String> {
    //     &self.name
    // }

    // Get the number of outgoing vertices for a given label
    pub fn out_num_vertices(&self, label: &EdgeLabel) -> usize {
        unsafe {
            let out_map = &*self.successors; // Dereference to get the map
            if let Some(vertices) = out_map.get(&label) {
                (**vertices).len()
            } else {
                0
            }
        }
    }

    // Get the number of incoming vertices for a given label
    pub fn in_num_vertices(&self, label: &EdgeLabel) -> usize {
        unsafe {
            let in_map = &*self.predecessors; // Dereference to get the map
            if let Some(vertices) = in_map.get(&label) {
                (**vertices).len()
            } else {
                0
            }
        }
    }

    // Get the degree of the node
    pub fn degree(&self) -> usize {
        unsafe {
            let mut ret = 0;

            // Calculate incoming vertices count
            for vertices in (*self.predecessors).values() {
                ret += (**vertices).len();
            }

            // Calculate outgoing vertices count
            for vertices in (*self.successors).values() {
                ret += (**vertices).len();
            }

            ret
        }
    }

    // Add a target node with a label
    pub fn add_target(&mut self, node: *mut AliasGraphNode, label: EdgeLabel) {
        unsafe {
            let out_map = &mut *self.successors;
            self.out_labels.insert(label);
            let out_nodes = &mut **out_map
                .entry(label)
                .or_insert_with(|| Box::into_raw(Box::new(FxHashSet::default())));
            out_nodes.insert(node);
            (*node).add_source(self as *mut _, label);
        }
    }

    // Remove a target node with a label
    pub fn remove_target(&mut self, node: *mut AliasGraphNode, label: &EdgeLabel) {
        unsafe {
            let out_map = &mut *self.successors; // Dereference the pointer

            if let Some(set) = out_map.get_mut(&label) {
                (&mut **set).remove(&node);
            }
            (*node).remove_source(self as *mut _, label); // Use unsafe to remove source
        }
    }

    // Check if the node contains a specific target node
    pub fn contains_target(&self, target: *mut AliasGraphNode, label: &EdgeLabel) -> bool {
        unsafe {
            (*self.successors)
                .get(&label)
                .map_or(false, |set| (&mut **set).contains(&target))
        }
    }

    // Get the set of incoming vertices for a given label
    pub fn get_in_vertices(
        &self,
        label: &EdgeLabel,
    ) -> Option<*mut FxHashSet<*mut AliasGraphNode>> {
        unsafe { (*self.predecessors).get(&label).copied() }
    }

    // Get the set of outgoing vertices for a given label
    pub fn get_out_vertices(
        &self,
        label: &EdgeLabel,
    ) -> Option<*mut FxHashSet<*mut AliasGraphNode>> {
        unsafe { (*self.successors).get(&label).copied() }
    }

    // Get one incoming vertex for a label, if exists
    pub fn get_in_vertex(&self, label: &EdgeLabel) -> Option<*mut AliasGraphNode> {
        unsafe {
            let set = self.get_in_vertices(label)?;
            if !(*set).is_empty() {
                (*set).iter().next().copied() // Copy the raw pointer
            } else {
                None
            }
        }
    }

    // Get one outgoing vertex for a label, if exists
    pub fn get_out_vertex(&self, label: &EdgeLabel) -> Option<*mut AliasGraphNode> {
        unsafe {
            let set = self.get_out_vertices(label)?;
            if !(*set).is_empty() {
                (*set).iter().next().copied() // Copy the raw pointer
            } else {
                None
            }
        }
    }

    // Returns a reference to the alias set
    pub fn get_alias_set(&self) -> *mut FxHashSet<*mut AliasGraphNode> {
        unsafe { self.alias_set }
    }

    // Moves the alias set of the current node to the RootRep node
    pub fn mv_alias_set_to(&mut self, root_rep: *mut AliasGraphNode) {
        // If the root representative is the current node, no action is needed
        if root_rep == self as *mut _ {
            return;
        }

        unsafe {
            // Get alias sets for both the current node and the root representative
            let root_alias_set = &mut *(*root_rep).alias_set; // Dereference to get the alias set of root_rep
            let this_alias_set = &mut *self.alias_set; // Dereference to get the alias set of self

            // Insert all elements from the current node's alias set into the root_rep's alias set
            root_alias_set.extend(this_alias_set.iter().copied());
        }
    }

    // Add a source node with a label (private function)
    fn add_source(&mut self, node: *mut AliasGraphNode, label: EdgeLabel) {
        unsafe {
            let in_map = &mut *self.predecessors; // Dereference the pointer
            self.in_labels.insert(label);
            let in_nodes = in_map
                .entry(label)
                .or_insert_with(|| Box::into_raw(Box::new(FxHashSet::default())));
            (&mut **in_nodes).insert(node);
        }
    }

    // Remove a source node with a label (private function)
    fn remove_source(&mut self, node: *mut AliasGraphNode, label: &EdgeLabel) {
        unsafe {
            if let Some(set) = (*self.predecessors).get_mut(&label) {
                (**set).remove(&node);
            }
        }
    }
}

impl Drop for AliasGraphNode {
    fn drop(&mut self) {
        println!("drop the node");
        // unsafe {
        //     // drop(Box::from_raw(self.id));
        //     if !self.out_labels.is_null() {
        //         let out_labels_set = Box::from_raw(self.out_labels);
        //         for &label_ptr in out_labels_set.iter() {
        //             drop(Box::from_raw(label_ptr));
        //         }
        //     }

        //     if !self.in_labels.is_null() {
        //         let in_labels_set = Box::from_raw(self.in_labels);
        //         for &label_ptr in in_labels_set.iter() {
        //             drop(Box::from_raw(label_ptr));
        //         }
        //     }

        //     drop(Box::from_raw(self.out_labels));
        //     drop(Box::from_raw(self.in_labels));
        // }
    }
}

impl AliasGraphNode {
    pub fn print_node(&self) {
        unsafe {
            // print current node id
            println!("node id: {:?}", self.id);

            // print alias set
            println!("  alias set:");
            let alias_set = &*self.alias_set;
            for node in alias_set.iter() {
                println!("      {:?}", **node);
            }

            // Print outgoing edges in the format A --label--> B
            println!("  outgoing edges:");
            let out_map = &*self.successors; // Dereference the pointer to get the map
            for (label, successors) in out_map.iter() {
                for &successor in (**successors).iter() {
                    let successor_node = &*successor; // Dereference successor node
                    let label_str = label; // Dereference to get the label value
                    let succ_id = &successor_node.id;
                    println!("      self --{:?}--> {:?}", label_str, succ_id);
                }
            }

            println!();

            // Print incoming edges in the format A --label--> B
            println!("  incoming edges:");
            let in_map = &*self.predecessors;
            for (label, predecessors) in in_map.iter() {
                for &predecessor in (**predecessors).iter() {
                    let predecessor_node = &*predecessor; // Dereference successor node
                    let label_str = label; // Dereference to get the label value
                    let pre_id = &predecessor_node.id;
                    println!("      {:?} --{:?}--> self", pre_id, label_str);
                }
            }
        }
    }
}

/// 1. a local's node index is its local index in mir
/// 2. a temp's node index is started from the number of locals
/// i.e., if there's 10 locals in a function, then:
/// assign _0 = _1.field_0.field1 ==>
/// _1 --field--> _10 --field--> _11, than make_alias(_0, _11)
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct GraphNodeId {
    pub def_id: DefId,
    pub index: usize,
}

impl GraphNodeId {
    pub fn new(def_id: DefId, index: Option<usize>) -> Self {
        GraphNodeId {
            def_id,
            index: match index {
                Some(i) => i,
                None => next_local_id(),
            },
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum EdgeLabel {
    Deref,
    Guard,
    // todo: field, array access
    Field(usize),
}

impl From<&str> for EdgeLabel {
    fn from(label: &str) -> Self {
        match label {
            "Deref" => EdgeLabel::Deref,
            "Guard" => EdgeLabel::Guard,
            _ => panic!("Unknown edge label!"),
        }
    }
}
impl EdgeLabel {
    pub fn new_field(field_idx: usize) -> Self {
        EdgeLabel::Field(field_idx)
    }
}
