#![feature(rustc_private)]

// use rust_probe::RustProbeDriver;
use rustc_compat::rustc_main;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::trace!("run deadlock");
    // rustc_main(RustProbeDriver);
}