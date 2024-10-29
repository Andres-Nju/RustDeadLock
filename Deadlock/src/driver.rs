use rustc_errors::registry;
use rustc_hash::FxHashMap;

use rustc_session::config;
use std::{path::PathBuf, process, str, sync::Arc};
use structopt::StructOpt;

use crate::{
    analysis::{
        alias::{
            AliasAnalysis
        }, 
        callgraph::CallGraph, 
        LockSetAnalysis
    }, 
    context::Context, 
    option::Options, 
    utils::{self, mir::{Display, ShowMir}}
};



/// 分析策略，每种策略是pass的数组，按顺序执行
struct Strategy {
    /// 策略的名字
    name: String,

    /// 趟
    passes: Vec<Box<dyn AnalysisPass>>,
}

impl<'tcx> Strategy {
    /// 创建新的策略
    fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            passes: Vec::new(),
        }
    }

    /// 在策略里面注册pass
    fn register_pass(&mut self, pass: Box<dyn AnalysisPass>) {
        self.passes.push(pass);
    }
}

/// 分析的pass
/// e.g. 建模pass -> 预分析pass -> 分析pass
///
pub(crate) trait AnalysisPass: Send {
    fn name(&self) -> String;

    /// 准备工作
    fn before_run(&mut self) {
        info!("Analysis Pass {} is running.", self.name());
    }

    /// 默认什么都不做
    fn go_pass(&mut self, context: &mut Context) {}

    /// 收尾工作
    fn after_run(&mut self) {}
}

/// 驱动器，里面有
/// 1. 分析/编译选项
/// 2. 分析策略
pub(crate) struct Driver {
    options: Options,
    strategy: FxHashMap<String, Strategy>,
}

impl Driver {
    /// 根据传入的Option创建一个驱动程序
    pub(crate) fn new(options: &Options) -> Self {
        if options.input_dir.is_none() {
            Options::clap().print_long_help().unwrap();
            process::exit(0);
        }

        Self {
            options: options.clone(),
            strategy: FxHashMap::default(),
        }
    }

    /// print
    fn print_basic(&mut self, context: &mut Context) {
        let tcx = context.tcx;
        let mut show_mir = ShowMir::new(tcx);
        let mut call_graph = CallGraph::new(tcx);
        show_mir.start();
        call_graph.start();
        let mut alias_analysis = AliasAnalysis::new(tcx, call_graph);
        alias_analysis.run_analysis();
        let (tcx, call_graph, alias_graph, control_flow_graph) = alias_analysis.consume_alias_results();
        let mut lock_set_analysis = LockSetAnalysis::new(tcx, call_graph, alias_graph, control_flow_graph);
        lock_set_analysis.run_analysis();
        // for (did, name) in &context.all_funcs {
        //     let mir = tcx.optimized_mir(did.as_local().unwrap()).clone();
        //     if self.options.show_all_funcs {
        //         println!("Discover Function: {} {}", did.display(), name);
        //     }
        //     if self.options.show_all_mir {
        //         println!("mir:{:#?}", mir.basic_blocks);
        //     }
        // }
    }

    /// 注册策略
    /// 根据编译选项来运行（TODO）
    fn register_strategy(&mut self) {
        // example:
        // 添加第一个策略，安德森
        // let mut stra1 = Strategy::new("Anderson");
        // stra1.register_pass(Box::new(ModelPass::default()));
        // stra1.register_pass(Box::new(AndersonPass::default()));
        // self.add_strategy(stra1);

        // -----------------
        // 添加更多策略
        // TODO
    }

    /// 执行策略
    /// 根据名字来逐个执行pass
    fn run_strategy(&mut self, name: &str, context: &mut Context<'_>) {
        match self.strategy.get_mut(name) {
            Some(stra) => {
                for pass in &mut stra.passes {
                    pass.before_run();
                    pass.go_pass(context);
                    pass.after_run();
                }
            }
            None => {
                panic!("No strategy named {}", name);
            }
        }
    }

    fn add_strategy(&mut self, stra: Strategy) {
        //println!("!!!!!!!!!!!!!!!! {}", stra.name.clone());
        self.strategy.insert(stra.name.clone(), stra);
    }

    /// 最关键的驱动函数，运行rustc编译成MIR，我们通过query来访问
    /// 这个函数主要做了以下几个事情：
    /// 1. 创建分析上下文
    /// 2. 进行分析
    /// 3. ...
    pub(crate) fn run_driver(&mut self) {
        let config = self.get_rustc_config();

        //let driver_mutex = Arc::new(Mutex::new(self));
        // 调用rustc接口，运行编译器
        // 官方手册：https://rustc-dev-guide.rust-lang.org/rustc-driver.html
        rustc_interface::run_compiler(config, |compiler| {
            compiler.enter(|queries| {
                //let mut driver = driver_mutex.lock().unwrap();

                queries.global_ctxt().unwrap().enter(|tcx| {
                    let mut context = Box::new(Context::new(&self.options, tcx));
                    // 初始化分析需要的上下文
                    context.tcx = tcx;
                    // 根据编译选项打印一些东西
                    self.print_basic(&mut context);

                    // 注册各种策略
                    self.register_strategy();

                    // 运行策略
                    // TODO: 重构为通过命令行选项来调用策略
                    // self.run_strategy("mir-print", &mut context);

                    // FIXME: 添加我们的逻辑，其余的Step，我理解是重构成一个个pass，然后每个pass，把结果存入pass里面
                    // ##############################
                });
            });
        });
    }

    /// 获取驱动rustc的configuration
    /// FIXME 如何编译一个crate
    pub(crate) fn get_rustc_config(&self) -> rustc_interface::Config {
        let out = process::Command::new("rustc")
            .arg("--print=sysroot")
            .current_dir(".")
            .output()
            .unwrap();
        let sysroot = str::from_utf8(&out.stdout).unwrap().trim();
        rustc_interface::Config {
            opts: config::Options {
                maybe_sysroot: Some(PathBuf::from(sysroot)),
                ..config::Options::default()
            },
            input: config::Input::File(self.options.input_dir.clone().unwrap().to_path_buf()),
            crate_cfg: Vec::new(),
            crate_check_cfg: Vec::new(),
            output_dir: None,
            output_file: None,
            file_loader: None,
            locale_resources: rustc_driver::DEFAULT_LOCALE_RESOURCES,
            lint_caps: rustc_hash::FxHashMap::default(),
            parse_sess_created: None,
            register_lints: None,
            override_queries: None,
            make_codegen_backend: None,
            registry: registry::Registry::new(&rustc_error_codes::DIAGNOSTICS),
            expanded_args: Vec::new(),
            ice_file: None,
            hash_untracked_state: None,
            using_internal_features: Arc::default(),
        }
    }
}
