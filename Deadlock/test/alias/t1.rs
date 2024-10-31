use std::sync::{Mutex, Arc};
fn main() {
    
    // Check the values by adding assertions or other checks
    tt();
    a();
}


fn tt() {

    // let a = Arc::new(Mutex::new(123));
    // let aa = &a;
    // let b = a.clone();
    // let c = a.lock().unwrap();
    // let d = b.lock().unwrap();
}

fn a(){
    b();
    c();
}

fn b(){ }

fn c(){ b();}