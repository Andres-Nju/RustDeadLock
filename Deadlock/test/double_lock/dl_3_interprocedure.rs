// single thread, interprocedural double lock

use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = Arc::clone(&mutex);
    
    let a = mutex.lock();
    
    get_lock(&mutex_clone);
}

fn get_lock(lock: &Mutex<u32>){
    let mu2 = lock.lock().unwrap();
}