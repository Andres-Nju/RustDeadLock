use rustc_middle::ty::TyCtxt;
use serde::{Deserialize, Serialize};

use clap::Parser;

/// 总程序，里面很多子命令，通过如“rustc_tester init ...”调用
#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
#[structopt(about = "This is a bug detector for Rust.")]
pub struct Options{
    // /// 指定源码对应的文件
    // #[arg(long = "directory")]
    // pub input_dir: Option<PathBuf>,
    /// 展示所有函数的名字
    #[arg(long = "show-all-funcs")]
    pub show_all_funcs: bool,

    /// 展示所有函数的MIR
    #[arg(long = "show-all-mir")]
    pub show_all_mir: bool,

    /// 打印mir
    #[arg(long = "emit-mir")]
    pub emit_mir: bool,

    // /// 默认是从main开始分析
    // #[arg(long = "entry-fun", default_value = "main")]
    // pub entry_func: String,

    // FIXME: 在下面添加更多的编译选项
    #[structopt(last = true)]
    pub cargo_args: Vec<String>,
}

impl Options {
    pub fn verify_options<'tcx>(&mut self, tcx: TyCtxt<'tcx>) {
        tracing::info!("RustProbe runs under options: {:?}", self);
        //TODO: 在这里预处理一些选项，可能没用
    }
}
