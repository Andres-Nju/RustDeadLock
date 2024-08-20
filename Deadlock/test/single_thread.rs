use std::sync::{Mutex, Arc};

fn main() {
    // 创建两个 Mutex 保护的整数
    let mutex1 = Arc::new(Mutex::new(0));
    let mutex2 = Arc::new(Mutex::new(0));

    // 锁定第一个 Mutex 并进行操作
    {
        let mut num1 = mutex1.lock().unwrap();
        *num1 += 10;
        println!("After incrementing, mutex1 holds: {}", *num1);
    } // 第一个 Mutex 在这里解锁

    // 锁定第二个 Mutex 并进行操作
    {
        let mut num2 = mutex2.lock().unwrap();
        *num2 += 20;
        println!("After incrementing, mutex2 holds: {}", *num2);
    } // 第二个 Mutex 在这里解锁

    // 再次锁定两个 Mutex，并交换它们的值
    {
        let mut num1 = mutex1.lock().unwrap();
        let mut num2 = mutex2.lock().unwrap();
        
        std::mem::swap(&mut *num1, &mut *num2);
        
        println!("After swapping, mutex1 holds: {}", *num1);
        println!("After swapping, mutex2 holds: {}", *num2);
    } // 两个 Mutex 在这里解锁
}
