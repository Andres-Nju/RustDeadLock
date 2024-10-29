use call_graph_node::CallGraphNode;
use collector::FnCollector;
use rustc_hash::FxHashSet;
use std::rc::Rc;
use rustc_middle::{mir::{Operand, TerminatorKind}};
use rustc_middle::ty::{self,TyCtxt};
use rustc_hir::{def_id::DefId,intravisit::Visitor,BodyId,HirId,ItemKind};
use rustc_span::Span;

pub mod collector;
pub mod call_graph_node;

pub struct CallGraph<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub collector: FnCollector<'tcx>,
    pub edges: FxHashSet<(Rc<CallGraphNode>, Rc<CallGraphNode>)>,
    entry: Option<Rc<CallGraphNode>>,
    pub topo: Vec<Rc<CallGraphNode>>,
}

impl<'tcx> CallGraph<'tcx>{
    pub fn new(tcx: TyCtxt<'tcx>) -> Self{ 
        Self{
            tcx,
            collector: FnCollector::new(tcx),
            edges: FxHashSet::default(),
            entry: None,
            topo: vec![],
        }
    }

    pub fn start(&mut self) {
        self.collector.collect(self.tcx);
    }


    pub fn topo_sort(&mut self) {
        let mut visited = FxHashSet::default();
        let mut stack = Vec::new();

        if let Some(entry_id) = &self.entry {
            if !visited.contains(entry_id) {
                self.dfs(entry_id.clone(), &mut visited, &mut stack);
            }
        } else {
            // if there's no entry, every caller is the entry.
            for (caller, _) in &self.edges {
                if !visited.contains(caller) {
                    self.dfs(caller.clone(), &mut visited, &mut stack);
                }
            }
        }

        while let Some(node) = stack.pop() {
            self.topo.push(node);
        }
    }

    // dfs to generate topo sort
    fn dfs(&self, node: Rc<CallGraphNode>, visited: &mut FxHashSet<Rc<CallGraphNode>>, stack: &mut Vec<Rc<CallGraphNode>>) {
        visited.insert(node.clone());
        
        for (_, callee) in self.edges.iter().filter(|(caller, _)| node.eq(caller)) {
            if !visited.contains(callee) {
                self.dfs(callee.clone(), visited, stack);
            }
        }
        
        stack.push(node);
    }

    pub fn print_call_edges(&self){
        println!("Show all edges of the call graph:");
        for (caller, callee) in &self.edges {
            println!("  {:?} -> {:?}", caller, callee);
        }
    }
    pub fn print_topo(&self){
        println!("Show the topo sort of the call graph:");
        for f in &self.topo{
            println!("{:?} ", f);
        }
        println!();
    }
}


