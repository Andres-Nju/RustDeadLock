// single thread double lock

use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = Arc::clone(&mutex);
    let mu1 = mutex.lock().unwrap();
    // let mu2 = mutex_clone.lock().unwrap();
}
