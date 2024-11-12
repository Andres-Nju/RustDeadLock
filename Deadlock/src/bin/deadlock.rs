#![feature(rustc_private)]

use rust_deadlock::MyDriver;
use rustc_compat::rustc_main;

fn main() {
    rustc_main(MyDriver);
}
