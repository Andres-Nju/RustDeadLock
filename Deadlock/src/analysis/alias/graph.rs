use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::{self, Place};

use crate::analysis::alias::node::EdgeLabel;

use super::node::{self, AliasGraphNode, GraphNodeId};


pub struct AliasGraph{
    nodes: FxHashSet<*mut AliasGraphNode>,
    node_map: FxHashMap<*const AliasGraphNode, *mut AliasGraphNode>,
}

impl Drop for AliasGraph{
    fn drop(&mut self) {  
        // for &node_ptr in self.nodes.iter() {
        //     unsafe {
        //         drop(Box::from_raw(node_ptr));
        //     }
        // }
    }
}

impl AliasGraph{
    pub fn new() -> Self {
        AliasGraph {
            nodes: FxHashSet::default(),
            node_map: FxHashMap::default(),
        }
    }

    pub fn find_vertex(&self, val: *mut AliasGraphNode) -> Option<*mut AliasGraphNode> {
        self.node_map.get(&(val as *const _)).copied()
    }

    pub fn add_node(&mut self, id: GraphNodeId, name: Option<String>) -> *mut AliasGraphNode {
        let node_ptr = AliasGraphNode::new(id, name);
        self.nodes.insert(node_ptr);
        self.node_map.insert(node_ptr as *const AliasGraphNode, node_ptr);
        node_ptr
    }

    pub fn get_or_insert_node(&mut self, id: GraphNodeId, name: Option<String>) -> *mut AliasGraphNode {
        let node_ptr = AliasGraphNode::new(id, name);
        match self.node_map.get(&(node_ptr as *const _)){
            Some(ptr) => {
                *ptr
            }
            None => {
                self.nodes.insert(node_ptr);
                self.node_map.insert(node_ptr as *const AliasGraphNode, node_ptr);
                node_ptr
            }
        }
    }

    // Combine two nodes into one, merging NodeY into NodeX
    pub fn combine(&mut self, node_x: *mut AliasGraphNode, node_y: *mut AliasGraphNode) -> *mut AliasGraphNode {
        unsafe {
            assert!(self.nodes.contains(&node_x));
            assert!(self.nodes.contains(&node_y));

            if node_x == node_y {
                return node_x;
            }

            // // Ensure node_x has the higher degree
            // if (*node_x).degree() < (*node_y).degree() {
            //     let t = node_x;
            //     node_x = node_y;
            //     node_y = t;
            // }

            // Merge NodeY's outgoing labels and targets into NodeX
            let y_out_labels = &*(*node_y).out_labels;
            for label in y_out_labels.iter() {
                if (*node_y).contains_target(node_y, *label) {
                    if !(*node_x).contains_target(node_x, *label) {
                        (*node_x).add_target(node_x, *label);
                    }
                    (*node_y).remove_target(node_y, *label);
                }
            }

            // Move NodeY's outgoing vertices to NodeX
            for label in y_out_labels.iter() {
                if let Some(out_vertices) = (*node_y).get_out_vertices(*label) {
                    for w in (*out_vertices).iter() {
                        if !(*node_x).contains_target(*w, *label) {
                            (*node_x).add_target(*w, *label);
                        }
                        // Modify safely without affecting the iterator
                        let w_temp = *w;
                        (*out_vertices).remove(w);
                        (*(*w_temp).get_in_vertices(*label).unwrap()).remove(&node_y);
                    }
                }
            }

            // Merge NodeY's incoming vertices into NodeX
            let y_in_labels = &*(*node_y).in_labels;
            for label in y_in_labels.iter() {
                if let Some(in_vertices) = (*node_y).get_in_vertices(*label) {
                    for w in (*in_vertices).iter() {
                        if !(**w).contains_target(node_x, *label) {
                            (**w).add_target(node_x, *label);
                        }
                        let w_temp = *w;
                        (*in_vertices).remove(w);
                        (*(*w_temp).get_out_vertices(*label).unwrap()).remove(&node_y);
                    }
                }
            }

            // Move equivalence class of NodeY to NodeX
            let vals = (*node_y).get_alias_set();
            for &val in (*vals).iter() {
                self.node_map.insert(val as *const _, node_x);
            }
            (*node_y).mv_alias_set_to(node_x);

            // Remove NodeY from the graph and delete it
            self.nodes.remove(&node_y);
            // drop(Box::from_raw(node_y)); // Safe deletion of NodeY

            node_x
        }
    }

    pub fn resolve_project(&mut self, def_id: &DefId, p: &Place) -> FxHashSet<*mut AliasGraphNode> {
        unsafe{
            let mut cur_node_id = GraphNodeId::new(def_id.clone(), p.local.as_usize());
            let cur_node = self.get_or_insert_node(cur_node_id, None);

            for projection in p.projection{
                match &projection{ // TODO: complex types
                    mir::ProjectionElem::Deref => {
                        if let Some(targets) = (*cur_node).get_out_vertices(&mut EdgeLabel::Deref as *mut _){

                        }
                        else{
                            
                        }
                    },
                    mir::ProjectionElem::Field(_, _) => (),
                    mir::ProjectionElem::Index(_) => todo!(),
                    mir::ProjectionElem::ConstantIndex { .. } => todo!(),
                    mir::ProjectionElem::Subslice { .. } => todo!(),
                    mir::ProjectionElem::Downcast(_, _) => todo!(),
                    mir::ProjectionElem::OpaqueCast(_) => todo!(),
                    mir::ProjectionElem::Subtype(_) => todo!(),
                }
            }
            FxHashSet::default()
        }
    }

    pub fn print(&self){
        for &node_ptr in &self.nodes{
            unsafe{
                (*node_ptr).print_node();
            }
            println!();
        }
    }
    pub fn print_graph(&self) {
        println!("node map:");
        for (key, val) in self.node_map.iter(){
            unsafe{
                println!("  {:?} --> {:?}", **key, **val);
            }
        }
        for &node_ptr in &self.nodes {
            unsafe {
                let node = &*node_ptr;
                println!("Node ID: {:?}", *node.id);
                    
                // print alias_set
                println!("  Alias Set:");
                if !node.alias_set.is_null() {
                    let alias_set = &*node.alias_set;
                    for &alias in alias_set.iter() {
                        let alias_node = &*alias;
                        println!("    - Node ID: {:?}", *alias_node.id);
                    }
                }

                // print out_labels
                println!("  Out Labels:");
                if !node.out_labels.is_null() {
                    let out_labels_set = &*node.out_labels;
                    for &label in out_labels_set.iter() {
                        println!("    - {:?}", *label); 
                    }
                }

                // print in_labels
                println!("  In Labels:");
                if !node.in_labels.is_null() {
                    let in_labels_set = &*node.in_labels; 
                    for &label in in_labels_set.iter() {
                        println!("    - {:?}", *label); 
                    }
                }

                // print successors
                println!("  Successors:");
                if !node.successors.is_null() {
                    let successors_map = &*node.successors; 
                    for (label, successors_set) in successors_map.iter() {
                        println!("    - Label: {:?}", **label);
                        for &successor in &**successors_set {
                            let successor_node = &*successor; 
                            println!("      - Node ID: {:?}", *successor_node.id);
                        }
                    }
                }

                // print predecessors
                println!("  Predecessors:");
                if !node.predecessors.is_null() {
                    let predecessors_map = &*node.predecessors; 
                    for (label, predecessors_set) in predecessors_map.iter() {
                        println!("    - Label: {:?}", **label);
                        for &predecessor in &**predecessors_set {
                            let predecessor_node = &*predecessor; 
                            println!("      - Node ID: {:?}", *predecessor_node.id);
                        }
                    }
                }
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests{
    use rustc_hir::def_id::DefIndex;

    use super::*;
    #[test]
    fn test_add_remove_target(){
        let mut graph = AliasGraph::new();
    
        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), 20), Some(String::from("node1")));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), 21), Some(String::from("node2")));
        
        unsafe {
            let label1 = Box::into_raw(Box::new(EdgeLabel::Deref));
            (*node1).add_target(node2, label1);

            // print
            println!("test1 ");
            graph.print_graph();

            (*node1).remove_target(node2, label1);
            // print
            println!("test2 ");
            graph.print_graph();

            let label2 = Box::into_raw(Box::new(EdgeLabel::Guard));
            (*node2).add_target(node1, label2);
            // print
            println!("test3 ");
            graph.print_graph();

            graph.print();
        }
    }

    #[test]
    fn test_mv_alias_set(){
        let mut graph = AliasGraph::new();
    
        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), 20), Some(String::from("node1")));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), 21), Some(String::from("node2")));
        
        unsafe {
            // move node1's set into node2
            let label1 = Box::into_raw(Box::new(EdgeLabel::Deref));
            (*node1).add_target(node2, label1);
            (*node1).mv_alias_set_to(node2);
            // print
            println!("test1 ");
            graph.print_graph();

            // move node2's set into node1
            (*node2).mv_alias_set_to(node1);
            println!("test2 ");
            graph.print_graph();
        }
    }

    #[test]
    fn test_combine(){
        let mut graph = AliasGraph::new();
    
        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), 20), Some(String::from("node1")));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), 21), Some(String::from("node2")));
        let node3 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(3)), 22), Some(String::from("node3")));
    
        unsafe{
            let label1 = Box::into_raw(Box::new(EdgeLabel::Deref));
            (*node1).add_target(node2, label1);
            let label2 = Box::into_raw(Box::new(EdgeLabel::Guard));
            (*node2).add_target(node1, label2);
            graph.print_graph();
        }
        println!("Combine node1 into node3\n");
        graph.combine(node3, node1);
        graph.print_graph();
    }
}