use rustc_middle::ty::TyCtxt;
use serde::{Deserialize, Serialize};

use clap::Parser;

#[derive(Parser, Clone, Debug, Serialize, Deserialize)]
#[structopt(about = "This is a bug detector for Rust.")]
pub struct Options {
    /// emit mir
    #[arg(long = "emit-mir")]
    pub emit_mir: bool,

    #[arg(long = "emit-call-graph")]
    pub emit_call_graph: bool,

    #[arg(long = "emit-alias-graph")]
    pub emit_alias_graph: bool,

    #[arg(long = "emit-lock-graph")]
    pub emit_lock_graph: bool,

    // FIXME: more compilation options
    #[structopt(last = true)]
    pub cargo_args: Vec<String>,
}

impl Options {
    pub fn verify_options<'tcx>(&mut self, tcx: TyCtxt<'tcx>) {
        tracing::info!("RustProbe runs under options: {:?}", self);
        //TODO: 在这里预处理一些选项，可能没用
    }
}
