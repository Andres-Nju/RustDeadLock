#![feature(rustc_private)]
#![feature(box_patterns)]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

extern crate rustc_ast_pretty;
extern crate rustc_driver;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_infer;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;
extern crate rustc_type_ir;


mod option;
mod driver;
mod context;
mod utils;
// mod model;

use option::Options;
use structopt::StructOpt;

fn main() {
    // 初始化logger，log使用方式请见README.md
    pretty_env_logger::init();
    info!("Begin to run RustProbe!");

    // 利用structopt从命令行参数中解析出options
    let rust_probe_options = Options::from_args();

    // 创建驱动实例并运行
    let mut driver = driver::Driver::new(&rust_probe_options);
    driver.run_driver();
}
