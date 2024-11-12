use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let lock1 = Arc::new(Mutex::new(()));
    let lock2 = Arc::new(Mutex::new(()));

    let l1 = Arc::clone(&lock1);
    let l2 = Arc::clone(&lock2);

    let handle1 = thread::spawn(move || {
        let _guard1 = l1.lock().unwrap();
        println!("Thread 1 acquired lock 1");
        thread::sleep(std::time::Duration::from_millis(10));
        let _guard2 = l2.lock().unwrap();
        println!("Thread 1 acquired lock 2");
    });

    let l1 = Arc::clone(&lock1);
    let l2 = Arc::clone(&lock2);

    let handle2 = thread::spawn(move || {
        let _guard2 = l2.lock().unwrap();
        println!("Thread 2 acquired lock 2");
        thread::sleep(std::time::Duration::from_millis(10));
        let _guard1 = l1.lock().unwrap();
        println!("Thread 2 acquired lock 1");
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
