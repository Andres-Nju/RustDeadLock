use std::path::PathBuf;

use rustc_middle::ty::TyCtxt;
use structopt::StructOpt;

#[derive(StructOpt, Clone, Debug)]
#[structopt(about = "This is a bug detector for Rust.")]
pub(crate) struct Options {
    /// 指定源码对应的文件
    #[structopt(short = "d", long = "directory")]
    pub input_dir: Option<PathBuf>,

    /// show name of all functions
    #[structopt(long = "show-all-funs")]
    pub show_all_funcs: bool,

    /// show mir of all functions
    #[structopt(long = "show-all-mir")]
    pub show_all_mir: bool,

    /// print mir
    #[structopt(long = "emit-mir")]
    pub emit_mir: bool,

    /// default: main
    #[structopt(long = "entry-fun", default_value = "main")]
    pub entry_func: String,
    // todo: more compilation options
}

impl Options {
    pub fn verify_options<'tcx>(&mut self, tcx: TyCtxt<'tcx>) {
        info!("Tool runs under options: {:?}", self);
    }
}
