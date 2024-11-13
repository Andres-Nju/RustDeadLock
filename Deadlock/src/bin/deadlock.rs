#![feature(rustc_private)]

use rust_deadlock::MyDriver;
use rustc_compat::rustc_main;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::trace!("run deadlock detection");
    rustc_main(MyDriver);
}
