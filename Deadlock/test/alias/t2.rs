use std::sync::{Mutex, Arc};
fn main() {
    let aa = a();
    let bb = b();
}

fn a() -> i32{
    b();
    return c();
}

fn b() -> i32{ return c(); }

fn c() -> i32{ let cc = b(); return 10; }

