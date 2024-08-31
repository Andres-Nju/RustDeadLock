/// Copied from RAP
/// Reference: RAP: https://github.com/Artisan-Lab/RAP

use rustc_middle::{mir::{Operand, TerminatorKind}};
use rustc_middle::ty::{self,TyCtxt};
use rustc_hir::{def_id::DefId,intravisit::Visitor,BodyId,HirId,ItemKind};
use rustc_span::Span;
use std::collections::HashMap;
use std::collections::HashSet;

pub type FnMap = HashMap<Option<HirId>, Vec<(BodyId, Span)>>;

/* 
   The graph simply records all pairs of callers and callees; 
   TODO: it can be extended, e.g.,
     1) to manage the graph as a linked list of function nodes
     2) to record all attributes of each function
*/
pub struct CallGraph<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub edges: HashSet<(DefId, DefId)>,
    entry: Option<DefId>,
    pub topo: Vec<DefId>,
}

impl<'tcx> CallGraph<'tcx>{
    pub fn new(tcx: TyCtxt<'tcx>) -> Self{
        Self{
            tcx,
            edges: HashSet::new(),
            entry: None,
            topo: vec![],
        }
    }

    pub fn start(&mut self) {
	    println!("Start callgraph analysis");
        let fn_items = FnCollector::collect(self.tcx);
        for (_, &ref vec) in & fn_items {
            for (body_id, _) in vec{
                let body_did = self.tcx.hir().body_owner_def_id(*body_id).to_def_id();
                if self.tcx.def_path_str(body_did) == "main"{
                    self.entry = Some(body_did);
                }
                self.find_callees(body_did);
            }
        }
        self.topo_sort();
        println!("Finish callgraph analysis");
        self.print_topo();
        self.topo.reverse();
    }

    pub fn find_callees(&mut self,def_id: DefId) {
        let tcx = self.tcx;
        if tcx.is_mir_available(def_id) {
            let body = tcx.optimized_mir(def_id);
            for bb in body.basic_blocks.iter() {
                match &bb.terminator().kind {
                    TerminatorKind::Call{func, args, ..} => {
                        if let Operand::Constant(func_constant) = func{
                            if let ty::FnDef(ref callee_def_id, _) = func_constant.const_.ty().kind() {
				                self.edges.insert((def_id,*callee_def_id));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // TODO: if cannot find callee locally?
    }

    pub fn topo_sort(&mut self) {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        if let Some(entry_id) = self.entry {
            if !visited.contains(&entry_id) {
                self.dfs(entry_id, &mut visited, &mut stack);
            }
        } else {
            // if there's no entry, every caller is the entry.
            for &(caller, _) in &self.edges {
                if !visited.contains(&caller) {
                    self.dfs(caller, &mut visited, &mut stack);
                }
            }
        }

        while let Some(node) = stack.pop() {
            self.topo.push(node);
        }
    }

    // dfs to generate topo sort
    fn dfs(&self, node: DefId, visited: &mut HashSet<DefId>, stack: &mut Vec<DefId>) {
        visited.insert(node);
        
        for &(_, callee) in self.edges.iter().filter(|&&(caller, _)| caller == node) {
            if !visited.contains(&callee) {
                self.dfs(callee, visited, stack);
            }
        }
        
        stack.push(node);
    }

    pub fn print_call_edges(&self){
        println!("Show all edges of the call graph:");
        for (caller, callee) in &self.edges {
            println!("  {} -> {}", self.tcx.def_path_str(*caller), self.tcx.def_path_str(*callee));
        }
    }
    pub fn print_topo(&self){
        println!("Show the topo sort of the call graph:");
        for f in &self.topo{
            println!("{} ", self.tcx.def_path_str(f));
        }
    }
}



pub struct FnCollector {
    fn_map: FnMap,
}

impl FnCollector {
    pub fn collect<'tcx>(tcx: TyCtxt<'tcx>) -> FnMap {
        let mut collector = FnCollector {
            fn_map: FnMap::default(),
        };
        tcx.hir().visit_all_item_likes_in_crate(&mut collector);
        collector.fn_map
    }
}

impl<'tcx> Visitor<'tcx> for FnCollector {
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
        match &item.kind {
            ItemKind::Fn(_fn_sig, _generics, body_id) => {
                let key = Some(body_id.hir_id);
                let entry = self.fn_map.entry(key).or_insert(Vec::new());
                entry.push((*body_id, item.span));
            }
            _ => (),
        }
    }
}