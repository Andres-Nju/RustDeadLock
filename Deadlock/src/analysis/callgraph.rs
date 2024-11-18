use call_graph_node::Call;
use collector::FnCollector;
use rustc_hash::{FxHashMap, FxHashSet};

use rustc_hir::{def_id::DefId, intravisit::Visitor, BodyId, HirId, ItemKind};
use rustc_middle::mir::{Location, Operand, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;

pub mod call_graph_node;
pub mod collector;

#[derive(Default)]
pub struct CallGraph<'tcx> {
    pub edges: FxHashSet<(DefId, DefId)>,
    entry: Option<DefId>,
    pub topo: Vec<DefId>,
    pub calls_map: FxHashMap<DefId, FxHashSet<Call<'tcx>>>,
}

impl<'tcx> CallGraph<'tcx> {
    pub fn new() -> Self {
        Self {
            edges: FxHashSet::default(),
            entry: None,
            topo: vec![],
            calls_map: FxHashMap::default(),
        }
    }

    pub fn start(&mut self, collector: &mut FnCollector<'tcx>) {
        println!("Start callgraph analysis");
        let fn_items = collector.collect(self.tcx);
        for def_id in fn_items.clone().into_iter() {
            self.find_callees(def_id);
        }
        self.topo_sort();
        println!("Finish callgraph analysis");
        // self.print_topo();
        self.topo.reverse();
    }

    pub fn find_callees(&mut self, def_id: DefId) {
        let tcx = self.tcx;
        if tcx.is_mir_available(def_id) {
            let body = tcx.optimized_mir(def_id);
            for bb in body.basic_blocks.iter() {
                match &bb.terminator().kind {
                    TerminatorKind::Call { func, args, .. } => {
                        if let Operand::Constant(func_constant) = func {
                            if let ty::FnDef(ref callee_def_id, _) =
                                func_constant.const_.ty().kind()
                            {
                                self.edges.insert((def_id, *callee_def_id));
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
        let mut visited = FxHashSet::default();
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
    fn dfs(&self, node: DefId, visited: &mut FxHashSet<DefId>, stack: &mut Vec<DefId>) {
        visited.insert(node);

        for &(_, callee) in self.edges.iter().filter(|&&(caller, _)| caller == node) {
            if !visited.contains(&callee) {
                self.dfs(callee, visited, stack);
            }
        }

        stack.push(node);
    }

    pub fn add_call(&mut self, caller: DefId, call: Call<'tcx>) {
        let calls = self.calls_map.entry(caller).or_insert(FxHashSet::default());
        calls.insert(call);
    }

    pub fn print_calls(&self) {
        for a in self.calls_map.iter() {
            println!("{:?} -> \n", a.0);
            for call in a.1 {
                println!("  {:?}", call);
            }
        }
    }

    pub fn print_call_edges(&self) {
        println!("Show all edges of the call graph:");
        for (caller, callee) in &self.edges {
            println!(
                "  {} -> {}",
                self.tcx.def_path_str(*caller),
                self.tcx.def_path_str(*callee)
            );
        }
    }
    pub fn print_topo(&self) {
        println!("Show the topo sort of the call graph:");
        for f in &self.topo {
            println!("{} ", self.tcx.def_path_str(f));
        }
        println!();
    }
}
