// multi-thread double lock

use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = Arc::clone(&mutex);
    let _lock1 = mutex.lock().unwrap();
    println!("Acquired first lock");
    let handle = thread::spawn(move || {
        // get the same lock
        let _lock2 = mutex_clone.lock().unwrap();
        println!("Acquired second lock");
    });
    handle.join().unwrap();
}
