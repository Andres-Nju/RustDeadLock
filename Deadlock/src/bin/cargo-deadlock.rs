#![feature(rustc_private)]

use rustc_compat::cargo_main;

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    tracing::debug!("run cargo deadlock");
    // cargo_main(RustProbeDriver);
}