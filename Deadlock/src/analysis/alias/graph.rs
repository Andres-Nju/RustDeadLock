use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::{self, Place};

use crate::analysis::alias::node::EdgeLabel;

use super::node::{AliasGraphNode, GraphNodeId};


pub struct AliasGraph{
    nodes: FxHashSet<Rc<AliasGraphNode>>,
    node_map: FxHashMap<GraphNodeId, Rc<AliasGraphNode>>,
}

impl AliasGraph{
    pub fn new() -> Self{
        AliasGraph{
            nodes: FxHashSet::default(),
            node_map: FxHashMap::default(),
        }
    } 

    pub fn get_or_insert_node(&mut self, node_id: GraphNodeId) -> Rc<AliasGraphNode>{
        // if cannot find the node, create one and insert it to the graph
        self.node_map.entry(node_id.clone()).or_insert(AliasGraphNode::new(node_id, None)).clone()
    }

    pub fn combine(&mut self, node_x: AliasGraphNode, node_y: AliasGraphNode){
        
    }

    pub fn resolve_project(&mut self, def_id: &DefId, p: &Place) -> Rc<AliasGraphNode> {
        let mut cur_node_id = GraphNodeId::new(def_id.clone(), p.local.as_usize());
        let mut cur_node = self.get_or_insert_node(cur_node_id);
        println!("{:?}: {:?}", p.local, p.projection);
        for projection in p.projection{
            match &projection{ // TODO: complex types
                mir::ProjectionElem::Deref => {
                    let deref_set = cur_node.get_predecessors_by_label(&EdgeLabel::Deref);
                    if let Some(deref_set) = deref_set{
                        
                    }
                    else{
                        panic!("There is no deref target for {:?}", cur_node);
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
        cur_node
    }
}