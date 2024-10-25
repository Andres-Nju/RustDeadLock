use std::sync::{Mutex, Arc};
fn main() {
    
    // Check the values by adding assertions or other checks
    tt();
}


fn tt() {


    let a = 1;
    let aa = &a;
    let mut aaa = &aa;
    let b = 0;
    let bb = &b;
    aaa = &bb;

}
