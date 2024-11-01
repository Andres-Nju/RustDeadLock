use std::sync::{Mutex, Arc};

fn main() {
    // 创建一个Arc包装的Mutex
    let lock = Arc::new(Mutex::new(0));

    // 第一次锁定 Mutex
    let result = match lock.lock() {
        Ok(_) => {
            // *num += 1;
            // println!("First lock acquired, value: {}", *num);

            // 第二次尝试锁定同一个 Mutex，导致死锁
            match lock.lock() {
                Ok(_) => {
                    // *num_again += 1;
                    // println!("Second lock acquired, value: {}", *num_again);
                }
                Err(_) => {
                    // println!("Failed to acquire second lock"),
                }
            }

            Ok(())
        }
        Err(_) => Err(()),
    };

    // 由于死锁，程序不会运行到此处
    if result.is_ok() {
        println!("Program completed successfully");
    } else {
        println!("Error occurred");
    }
}