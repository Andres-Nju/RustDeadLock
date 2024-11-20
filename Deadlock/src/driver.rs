use clap::Parser;
use rustc_compat::{CrateFilter, Plugin, RustcPluginArgs, Utf8Path};
use rustc_driver::Compilation;
use rustc_errors::registry;
use rustc_hash::FxHashMap;

use rustc_middle::ty::TyCtxt;
use rustc_session::config;
use std::{
    borrow::Cow,
    env,
    path::PathBuf,
    process::{self, Command},
    str,
    sync::Arc,
};
use structopt::StructOpt;

use crate::{
    analysis::{
        alias::AliasAnalysis,
        callgraph::{CallGraph, CallGraphPass},
        LockSetAnalysis,
    },
    context::MyTcx,
    option::Options,
    utils::{
        self,
        mir::{Display, ShowMir},
    },
};

/// a strategy consists of all necessary passes
struct Strategy {
    name: String,
    passes: Vec<Box<dyn AnalysisPass>>,
}

impl<'tcx> Strategy {
    fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            passes: Vec::new(),
        }
    }

    /// register pass in a strategy
    fn register_pass(&mut self, pass: Box<dyn AnalysisPass>) {
        self.passes.push(pass);
    }
}

pub(crate) trait AnalysisPass: Send {
    fn name(&self) -> String;

    fn before_run(&mut self) {
        tracing::info!("{} analysis is running.", self.name());
    }

    fn run_pass(&mut self) {}

    fn after_run(&mut self) {}
}

pub(crate) struct MyCallBacks {
    options: Options,
    strategy: FxHashMap<String, Strategy>,
}

impl MyCallBacks {
    pub(crate) fn new(options: &Options) -> Self {
        Self {
            options: options.clone(),
            strategy: FxHashMap::default(),
        }
    }

    /// print
    fn print_basic<'tcx>(&mut self, tcx: &TyCtxt<'tcx>) {
        // let mut show_mir = ShowMir::new(*tcx);
        // show_mir.start();
        // let mut call_graph = CallGraph::new(*tcx);
        // call_graph.start();
        // let mut alias_analysis = AliasAnalysis::new(*tcx, call_graph);
        // alias_analysis.run_analysis();
        // let (tcx, call_graph, alias_graph, control_flow_graph) =
        //     alias_analysis.consume_alias_results();
        // let mut lock_set_analysis =
        //     LockSetAnalysis::new(tcx, call_graph, alias_graph, control_flow_graph);
        // lock_set_analysis.run_analysis();
    }

    /// register strategies
    fn register_strategy(&mut self, my_tcx: MyTcx) {
        let tcx_boxed = Box::new(my_tcx);
        let my_tcx = Box::leak(tcx_boxed);
        let mut strategy = Strategy::new("Call Graph Construction");
        let pass1 = CallGraphPass::new(my_tcx);
        // strategy.register_pass(Box::new(pass1));
        self.add_strategy(strategy);

        // -----------------
        // TODO: more strategy
        // FIXME: 如何重用tcx？这里用strategy封装的话，内置数据结构不能用&my_tcx
    }

    fn run_strategy(&mut self, name: &str) {
        match self.strategy.get_mut(name) {
            Some(stra) => {
                for pass in &mut stra.passes {
                    pass.before_run();
                    // pass.run_pass(context);
                    pass.after_run();
                }
            }
            None => {
                panic!("No strategy named {}", name);
            }
        }
    }

    fn add_strategy(&mut self, stra: Strategy) {
        self.strategy.insert(stra.name.clone(), stra);
    }
}

impl rustc_driver::Callbacks for MyCallBacks {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        _queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> rustc_driver::Compilation {
        _queries.global_ctxt().unwrap().enter(|tcx| {
            if self.options.emit_mir {
                let mut show_mir = ShowMir::new(tcx);
                show_mir.start();
            }

            let my_tcx = MyTcx::new(tcx);

            // self.register_strategy(my_tcx);
            // TODO
            // self.run_strategy();

            let tcx_boxed = Box::new(my_tcx);
            let my_tcx = Box::leak(tcx_boxed);
            // call graph pre build pass
            let mut call_graph_pre_build_pass = CallGraphPass::new(my_tcx);
            call_graph_pre_build_pass.start();

            if self.options.emit_call_graph {
                call_graph_pre_build_pass.print_topo();
            }

            // alias analysis pass
            let mut alias_anaysis_pass = AliasAnalysis::new(my_tcx);
            alias_anaysis_pass.run_analysis();

            if self.options.emit_call_graph {
                alias_anaysis_pass.my_tcx.call_graph.print_calls();
            }

            if self.options.emit_alias_graph {
                for (def_id, alias_graph) in my_tcx.alias_graph.iter() {
                    println!("Function: {:?}", def_id);
                    alias_graph.print_graph();
                }
            }

            // lock set analysis pass
            let mut lock_analysis_pass = LockSetAnalysis::new(my_tcx);
            lock_analysis_pass.run_analysis();

            if self.options.emit_lock_graph {
                lock_analysis_pass.print_lock_set_facts();
                lock_analysis_pass.lock_graph.print_loops();
            }
        });
        Compilation::Continue
    }
}

#[derive(Default)]
pub struct MyDriver;

impl Plugin for MyDriver {
    type Args = Options;

    fn version(&self) -> Cow<'static, str> {
        env!("CARGO_PKG_VERSION").into()
    }

    fn driver_name(&self) -> Cow<'static, str> {
        "deadlock".into()
    }

    // In the CLI, we ask Clap to parse arguments and also specify a CrateFilter.
    // If one of the CLI arguments was a specific file to analyze, then you
    // could provide a different filter.
    fn args(&self, _target_dir: &Utf8Path) -> RustcPluginArgs<Self::Args> {
        let args = Options::parse_from(env::args().skip(1));
        println!("{:?}", args);
        let filter = CrateFilter::AllCrates;
        RustcPluginArgs { args, filter }
    }

    // Pass Cargo arguments (like --feature) from the top-level CLI to Cargo.
    fn modify_cargo(&self, cargo: &mut Command, args: &Self::Args) {
        cargo.args(&args.cargo_args);
    }

    // In the driver, we use the Rustc API to start a compiler session
    // for the arguments given to us by rustc_plugin.
    fn run(
        self,
        compiler_args: Vec<String>,
        plugin_args: Self::Args,
    ) -> rustc_interface::interface::Result<()> {
        tracing::debug!("Rust Probe start to run.");
        let mut callbacks = MyCallBacks::new(&plugin_args);
        let compiler = rustc_driver::RunCompiler::new(&compiler_args, &mut callbacks);
        compiler.run()
    }
}
