use std::sync::{Arc, Mutex};
fn main(){
    let mutex1 = Arc::new(Mutex::new(0));
let mutex2 = mutex1.clone();
mutex1.lock().unwrap();
mutex2.lock().unwrap();
}

fn tt(b: &Mutex<i32>){
    tt2(b);
}

fn tt2(b: &Mutex<i32>){
    
}