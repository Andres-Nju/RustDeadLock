use std::sync::{Mutex, Arc};
fn main() {
    let a = Mutex::new(123);
    // Check the values by adding assertions or other checks
    tt();
}


fn tt(m: &Mutex<i32>) {
    m.lock();
    let a = Arc::new(Mutex::new(123));
    let aa = &a;
    let b = a.clone();
    let c = a.lock().unwrap();
    let d = b.lock().unwrap();
}
