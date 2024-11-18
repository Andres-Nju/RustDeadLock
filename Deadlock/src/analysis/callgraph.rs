use call_graph_node::Call;
use collector::FnCollector;
use rustc_hash::{FxHashMap, FxHashSet};

use rustc_hir::{def_id::DefId, intravisit::Visitor, BodyId, HirId, ItemKind};
use rustc_middle::mir::{Location, Operand, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;

use crate::context::MyTcx;
use crate::driver::AnalysisPass;

pub mod call_graph_node;
pub mod collector;

#[derive(Clone)]
pub struct CallGraph<'tcx> {
    pub edges: FxHashSet<(DefId, DefId)>,
    entry: Option<DefId>,
    pub topo: Vec<DefId>,
    pub calls_map: FxHashMap<DefId, FxHashSet<Call<'tcx>>>,
    pub fn_set: FxHashSet<DefId>,
}

pub struct CallGraphPass<'a, 'tcx> {
    my_tcx: &'a mut MyTcx<'tcx>,
}

impl<'tcx> Visitor<'tcx> for CallGraphPass<'_, '_> {
    fn visit_item(&mut self, item: &'tcx rustc_hir::Item<'tcx>) {
        match &item.kind {
            ItemKind::Fn(_fn_sig, _generics, body_id) => {
                let def_id = self
                    .my_tcx
                    .tcx
                    .hir()
                    .body_owner_def_id(*body_id)
                    .to_def_id();
                if self.my_tcx.tcx.def_path_str(def_id) == "main" {
                    self.my_tcx.call_graph.entry = Some(def_id);
                }
                self.my_tcx.call_graph.fn_set.insert(def_id);
            }
            _ => (),
        }
    }
}

impl<'a, 'tcx> CallGraphPass<'a, 'tcx> {
    pub fn new(my_tcx: &'a mut MyTcx<'tcx>) -> Self {
        Self { my_tcx }
    }

    pub fn collect(&mut self) -> &FxHashSet<DefId> {
        self.my_tcx.tcx.hir().visit_all_item_likes_in_crate(self);
        &self.my_tcx.call_graph.fn_set
    }

    pub fn start(&mut self) {
        println!("Start callgraph analysis");
        let fn_items = self.collect();
        for def_id in fn_items.clone().into_iter() {
            self.find_callees(def_id);
        }
        self.topo_sort();
        println!("Finish callgraph analysis");
        // self.print_topo();
        self.my_tcx.call_graph.topo.reverse();
    }

    pub fn find_callees(&mut self, def_id: DefId) {
        let tcx = self.my_tcx.tcx;
        if tcx.is_mir_available(def_id) {
            let body = tcx.optimized_mir(def_id);
            for bb in body.basic_blocks.iter() {
                match &bb.terminator().kind {
                    TerminatorKind::Call { func, args, .. } => {
                        if let Operand::Constant(func_constant) = func {
                            if let ty::FnDef(ref callee_def_id, _) =
                                func_constant.const_.ty().kind()
                            {
                                self.my_tcx
                                    .call_graph
                                    .edges
                                    .insert((def_id, *callee_def_id));
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

        if let Some(entry_id) = &self.my_tcx.call_graph.entry {
            if !visited.contains(entry_id) {
                self.dfs(entry_id.clone(), &mut visited, &mut stack);
            }
        } else {
            // if there's no entry, every caller is the entry.
            for (caller, _) in &self.my_tcx.call_graph.edges {
                if !visited.contains(caller) {
                    self.dfs(caller.clone(), &mut visited, &mut stack);
                }
            }
        }

        while let Some(node) = stack.pop() {
            self.my_tcx.call_graph.topo.push(node);
        }
    }

    // dfs to generate topo sort
    fn dfs(&self, node: DefId, visited: &mut FxHashSet<DefId>, stack: &mut Vec<DefId>) {
        visited.insert(node.clone());

        for (_, callee) in self
            .my_tcx
            .call_graph
            .edges
            .iter()
            .filter(|(caller, _)| *caller == node)
        {
            if !visited.contains(callee) {
                self.dfs(callee.clone(), visited, stack);
            }
        }

        stack.push(node);
    }

    pub fn add_call(&mut self, caller: DefId, call: Call<'tcx>) {
        let calls = self
            .my_tcx
            .call_graph
            .calls_map
            .entry(caller)
            .or_insert(FxHashSet::default());
        calls.insert(call);
    }

    pub fn print_calls(&self) {
        for a in self.my_tcx.call_graph.calls_map.iter() {
            println!("{:?} -> \n", a.0);
            for call in a.1 {
                println!("  {:?}", call);
            }
        }
    }

    pub fn print_call_edges(&self) {
        println!("Show all edges of the call graph:");
        for (caller, callee) in &self.my_tcx.call_graph.edges {
            println!(
                "  {} -> {}",
                self.my_tcx.tcx.def_path_str(caller),
                self.my_tcx.tcx.def_path_str(callee)
            );
        }
    }
    pub fn print_topo(&self) {
        println!("Show the topo sort of the call graph:");
        for f in &self.my_tcx.call_graph.topo {
            println!("{} ", self.my_tcx.tcx.def_path_str(f));
        }
        println!();
    }
}

impl<'tcx> CallGraph<'tcx> {
    pub fn new() -> Self {
        Self {
            edges: FxHashSet::default(),
            entry: None,
            topo: vec![],
            calls_map: FxHashMap::default(),
            fn_set: FxHashSet::default(),
        }
    }
}

impl<'a, 'tcx> AnalysisPass for CallGraphPass<'a, 'tcx> {
    fn name(&self) -> String {
        "[Call Graph pre build]".to_string()
    }

    fn before_run(&mut self) {
        tracing::info!("{} analysis is running.", self.name());
    }

    fn run_pass(&mut self) {
        self.start();
    }

    fn after_run(&mut self) {}
}
