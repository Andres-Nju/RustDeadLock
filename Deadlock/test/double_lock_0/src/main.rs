use std::sync::{Arc, Mutex};

fn main() {
    // 创建一个Arc包装的Mutex
    let lock = Arc::new(Mutex::new(0));

    // 第一次锁定 Mutex
    let mut num = lock.lock().unwrap();
    *num += 1;
    println!("First lock acquired, value: {}", *num);

    // 第二次尝试锁定同一个 Mutex，导致死锁
    let mut num_again = lock.lock().unwrap();
    *num_again += 1;
    println!("Second lock acquired, value: {}", *num_again);

    // 理论上不会执行到这一步，因为死锁会在第二次锁定时发生
    println!("Program completed");
}
