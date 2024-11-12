#![feature(rustc_private)]

use rust_deadlock::MyDriver;
use rustc_compat::cargo_main;

fn main() {
    dotenvy::dotenv().ok();
    cargo_main(MyDriver);
}
