use std::sync::{Arc, Mutex};
use std::thread;
fn main() {
    let mutex = Arc::new(Mutex::new(0));
    let mutex_clone = Arc::clone(&mutex);
    let mu1 = mutex.lock().unwrap();
    let mu2 = mutex_clone.lock().unwrap();
    println!("12123");
    // let handle = thread::spawn(move || {
    //     let _lock1 = mutex_clone.lock().unwrap();
    //     println!("Acquired first lock");
    //     // 尝试再次获取相同的锁
    //     let _lock2 = mutex_clone.lock().unwrap();
    //     println!("Acquired second lock");
    // });
    // handle.join().unwrap();
}
