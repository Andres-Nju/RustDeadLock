use std::{collections::VecDeque, fmt::format, rc::Rc, thread::current};

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::{self, Place};

use crate::analysis::alias::node::EdgeLabel;

use super::node::{self, AliasGraphNode, GraphNodeId};

#[derive(Clone)]
pub struct AliasGraph {
    // might be problematic, as Rust hashes the raw pointer literally, not according to its data
    nodes: FxHashSet<*mut AliasGraphNode>,
    node_map: FxHashMap<GraphNodeId, *mut AliasGraphNode>,
}

impl Drop for AliasGraph {
    fn drop(&mut self) {
        // for &node_ptr in self.nodes.iter() {
        //     unsafe {
        //         drop(Box::from_raw(node_ptr));
        //     }
        // }
    }
}

impl AliasGraph {
    pub fn new() -> Self {
        AliasGraph {
            nodes: FxHashSet::default(),
            node_map: FxHashMap::default(),
        }
    }

    pub fn find_vertex(&self, val: *mut AliasGraphNode) -> Option<*mut AliasGraphNode> {
        unsafe { self.node_map.get(&(*val).id).copied() }
    }

    pub fn add_node(&mut self, id: GraphNodeId) -> *mut AliasGraphNode {
        let node_ptr = AliasGraphNode::new(id);
        self.nodes.insert(node_ptr);
        self.node_map.insert(id, node_ptr);
        node_ptr
    }

    pub fn get_or_insert_node(&mut self, id: GraphNodeId) -> *mut AliasGraphNode {
        unsafe {
            let node_ptr = AliasGraphNode::new(id);
            match self.node_map.get(&(*node_ptr).id) {
                Some(ptr) => *ptr,
                None => {
                    self.nodes.insert(node_ptr);
                    self.node_map.insert((*node_ptr).id, node_ptr);
                    node_ptr
                }
            }
        }
    }

    // Combine two nodes into one, merging NodeY into NodeX
    pub fn combine(
        &mut self,
        mut node_x: *mut AliasGraphNode,
        mut node_y: *mut AliasGraphNode,
    ) -> *mut AliasGraphNode {
        unsafe {
            assert!(self.nodes.contains(&node_x));
            assert!(self.nodes.contains(&node_y));

            if node_x == node_y {
                return node_x;
            }

            // Ensure node_x has the higher degree
            if (*node_x).degree() < (*node_y).degree() {
                std::mem::swap(&mut node_x, &mut node_y);
            }

            // Merge NodeY's outgoing labels and targets into NodeX
            for label in (*node_y).out_labels.iter() {
                if (*node_y).contains_target(node_y, label) {
                    if !(*node_x).contains_target(node_x, label) {
                        (*node_x).add_target(node_x, label.clone());
                    }
                    (*node_y).remove_target(node_y, label);
                }
            }

            // Move NodeY's outgoing vertices to NodeX
            for label in (*node_y).out_labels.iter() {
                if let Some(out_vertices) = (*node_y).get_out_vertices(label) {
                    for w in (*out_vertices).iter() {
                        if !(*node_x).contains_target(*w, label) {
                            (*node_x).add_target(*w, label.clone());
                        }
                        // Modify safely without affecting the iterator
                        let w_temp = *w;
                        (*out_vertices).remove(w);
                        (*(*w_temp).get_in_vertices(label).unwrap()).remove(&node_y);
                    }
                }
            }

            // Merge NodeY's incoming vertices into NodeX
            for label in (*node_y).in_labels.iter() {
                if let Some(in_vertices) = (*node_y).get_in_vertices(label) {
                    for w in (*in_vertices).iter() {
                        if !(**w).contains_target(node_x, label) {
                            (**w).add_target(node_x, label.clone());
                        }
                        let w_temp = *w;
                        (*in_vertices).remove(w);
                        (*(*w_temp).get_out_vertices(label).unwrap()).remove(&node_y);
                    }
                }
            }

            // Move equivalence class of NodeY to NodeX
            let vals = (*node_y).get_alias_set();
            for &val in (*vals).iter() {
                self.node_map.insert((*val).id, node_x);
            }
            (*node_y).mv_alias_set_to(node_x);

            // Remove NodeY from the graph and delete it
            self.nodes.remove(&node_y);
            // drop(Box::from_raw(node_y)); // Safe deletion of NodeY

            node_x
        }
    }

    pub fn qirun_algorithm(&mut self) {
        let mut work_list = VecDeque::new();
        unsafe {
            for node in self.nodes.iter() {
                for label in (**node).out_labels.iter() {
                    if (**node).out_num_vertices(label) > 1 {
                        work_list.push_back((*node, label.clone()));
                    }
                }
            }

            while let Some((z_node, label)) = work_list.pop_front() {
                let nodes = (*z_node).get_out_vertices(&label).unwrap();
                if (*nodes).len() <= 1 {
                    continue;
                }

                let mut nodes_iter = (*nodes).iter();

                let mut x = *nodes_iter.next().unwrap();
                while let Some(y) = nodes_iter.next() {
                    let mut y = *y;
                    if (*x).degree() < (*y).degree() {
                        std::mem::swap(&mut x, &mut y);
                    }

                    assert_ne!(x, y);
                    self.nodes.remove(&y);

                    let y_equiv_set = (*y).get_alias_set();
                    for &val in (*y_equiv_set).iter() {
                        self.node_map.insert((*val).id, x);
                    }
                    (*y).mv_alias_set_to(x);

                    // Process Y's outgoing edges
                    for &out_label in (*y).out_labels.iter() {
                        if (*y).contains_target(y, &out_label) {
                            if !(*x).contains_target(x, &out_label) {
                                (*x).add_target(x, out_label);
                                if (*x).out_num_vertices(&out_label) > 1 {
                                    work_list.push_back((x, out_label));
                                }
                            }
                            (*y).remove_target(y, &out_label);
                        }
                    }

                    // Transfer remaining targets
                    for label in (*y).out_labels.iter() {
                        if let Some(out_vertices) = (*y).get_out_vertices(label) {
                            for &w in (*out_vertices).iter() {
                                if !(*x).contains_target(w, label) {
                                    (*x).add_target(w, *label);
                                    if (*x).out_num_vertices(label) > 1 {
                                        work_list.push_back((x, *label));
                                    }
                                }
                                // Modify safely without affecting the iterator
                                let w_temp = w;
                                (*out_vertices).remove(&w);
                                (*(*w_temp).get_in_vertices(label).unwrap()).remove(&y);
                            }
                        }
                    }

                    // Process Y's incoming edges
                    for label in (*y).in_labels.iter() {
                        if let Some(in_vertices) = (*y).get_in_vertices(label) {
                            for &w in (*in_vertices).iter() {
                                if !(*w).contains_target(x, label) {
                                    (*w).add_target(x, *label);
                                }
                                // Modify safely without affecting the iterator
                                let w_temp = w;
                                (*in_vertices).remove(&w);
                                (*(*w_temp).get_out_vertices(label).unwrap()).remove(&y);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn resolve_project(&mut self, def_id: &DefId, p: &Place) -> *mut AliasGraphNode {
        unsafe {
            let cur_node_id = GraphNodeId::new(def_id.clone(), Some(p.local.as_usize()));
            let mut cur_node = self.get_or_insert_node(cur_node_id);
            // let mut current_node_set = Box::into_raw(Box::new(FxHashSet::default()));
            // (*current_node_set).insert(cur_node);
            for projection in p.projection {
                match &projection {
                    // TODO: complex types
                    mir::ProjectionElem::Deref => {
                        // (*p).* ... get q of all p --deref--> q; if there's no such q, create one
                        let deref_label = EdgeLabel::Deref;
                        if let Some(targets) = (*cur_node).get_out_vertices(&deref_label) {
                            // if the set is empty
                            if (*targets).is_empty() {
                                let target_node =
                                    self.add_node(GraphNodeId::new(def_id.clone(), None));
                                (*cur_node).add_target(target_node, deref_label);
                                // (*current_node_set).insert(target_node);
                                cur_node = target_node;
                            } else {
                                cur_node = *(*targets).iter().next().unwrap();
                            }
                        } else {
                            let target_id = GraphNodeId::new(def_id.clone(), None);
                            let target_node = self.add_node(target_id);
                            (*cur_node).add_target(target_node, deref_label);
                            // (*current_node_set).insert(target_node);
                            cur_node = target_node;
                        }
                    }
                    mir::ProjectionElem::Field(field_idx, _) => {
                        let field_label = EdgeLabel::new_field(field_idx.as_usize());
                        if let Some(targets) = (*cur_node).get_out_vertices(&field_label) {
                            // if the set is empty
                            if (*targets).is_empty() {
                                let target_node =
                                    self.add_node(GraphNodeId::new(def_id.clone(), None));
                                (*cur_node).add_target(target_node, field_label);
                                // (*current_node_set).insert(target_node);
                                cur_node = target_node;
                            } else {
                                cur_node = *(*targets).iter().next().unwrap();
                            }
                        } else {
                            let target_id = GraphNodeId::new(def_id.clone(), None);
                            let target_node = self.add_node(target_id);
                            (*cur_node).add_target(target_node, field_label);
                            // (*current_node_set).insert(target_node);
                            cur_node = target_node;
                        }
                    }
                    mir::ProjectionElem::Index(_) => todo!(),
                    mir::ProjectionElem::ConstantIndex { .. } => todo!(),
                    mir::ProjectionElem::Subslice { .. } => todo!(),
                    mir::ProjectionElem::Downcast(_, _) => (),
                    mir::ProjectionElem::OpaqueCast(_) => todo!(),
                    mir::ProjectionElem::Subtype(_) => todo!(),
                }
            }
            // current_node_set
            cur_node
        }
    }

    pub fn print(&self) {
        for &node_ptr in &self.nodes {
            unsafe {
                (*node_ptr).print_node();
            }
            println!();
        }
    }
    pub fn print_graph(&self) {
        println!("node map:");
        for (key, val) in self.node_map.iter() {
            unsafe {
                println!("  {:?} --> {:?}", key, **val);
            }
        }
        for &node_ptr in &self.nodes {
            unsafe {
                let node = &*node_ptr;
                println!("Node ID: {:?}", node.id);

                // print alias_set
                println!("  Alias Set:");
                if !node.alias_set.is_null() {
                    let alias_set = &*node.alias_set;
                    for &alias in alias_set.iter() {
                        let alias_node = &*alias;
                        println!("    - Node ID: {:?}", alias_node.id);
                    }
                }

                // print out_labels
                println!("  Out Labels:");
                if !node.out_labels.is_empty() {
                    for &label in node.out_labels.iter() {
                        println!("    - {:?}", label);
                    }
                }

                // print in_labels
                println!("  In Labels:");
                if !node.in_labels.is_empty() {
                    for &label in node.in_labels.iter() {
                        println!("    - {:?}", label);
                    }
                }

                // print successors
                println!("  Successors:");
                if !node.successors.is_null() {
                    let successors_map = &*node.successors;
                    for (label, successors_set) in successors_map.iter() {
                        println!("    - Label: {:?}", label);
                        for &successor in &**successors_set {
                            let successor_node = &*successor;
                            println!("      - Node ID: {:?}", successor_node.id);
                        }
                    }
                }

                // print predecessors
                println!("  Predecessors:");
                if !node.predecessors.is_null() {
                    let predecessors_map = &*node.predecessors;
                    for (label, predecessors_set) in predecessors_map.iter() {
                        println!("    - Label: {:?}", label);
                        for &predecessor in &**predecessors_set {
                            let predecessor_node = &*predecessor;
                            println!("      - Node ID: {:?}", predecessor_node.id);
                        }
                    }
                }
            }
            println!();
        }
    }
}

#[cfg(test)]
mod tests {
    use rustc_hir::def_id::DefIndex;

    use super::*;
    #[test]
    fn test_add_remove_target() {
        let mut graph = AliasGraph::new();

        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), None));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), None));

        unsafe {
            let label1 = EdgeLabel::Deref;
            (*node1).add_target(node2, label1);

            // print
            println!("test1 ");
            graph.print_graph();

            (*node1).remove_target(node2, &label1);
            // print
            println!("test2 ");
            graph.print_graph();

            let label2 = EdgeLabel::Guard;
            (*node2).add_target(node1, label2);
            // print
            println!("test3 ");
            graph.print_graph();

            graph.print();
        }
    }

    #[test]
    fn test_mv_alias_set() {
        let mut graph = AliasGraph::new();

        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), None));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), None));
        unsafe {
            // move node1's set into node2
            let label1 = EdgeLabel::Deref;
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
    fn test_combine() {
        let mut graph = AliasGraph::new();

        // add 2 nodes
        let node1 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(1)), None));
        let node2 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(2)), None));
        let node3 = graph.add_node(GraphNodeId::new(DefId::local(DefIndex::from_u32(3)), None));

        unsafe {
            let label1 = EdgeLabel::Deref;
            (*node1).add_target(node2, label1);
            let label2 = EdgeLabel::Guard;
            (*node2).add_target(node1, label2);
            graph.print_graph();
        }
        println!("Combine node1 into node3\n");
        graph.combine(node3, node1);
        graph.print_graph();

        unsafe {
            (*node3).remove_target(node2, &EdgeLabel::Deref);
        }
        println!("Remove node2 from node3's target\n");
        graph.print_graph();
    }
}
