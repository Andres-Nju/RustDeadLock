use std::sync::{Mutex, Arc};

fn main() {
    // 创建两个被 Mutex 保护的整数
    let counter1 = Arc::new(Mutex::new(0));
    let counter2 = Arc::new(Mutex::new(0));

    // 定义循环的次数
    let iterations = 5;

    for i in 0..iterations {
        // 锁定第一个 Mutex 并进行操作
        {
            let mut num1 = counter1.lock().unwrap();
            *num1 += i;
            println!("Iteration {}: counter1 is now {}", i, *num1);
        } // 第一个 Mutex 在这里解锁

        // 锁定第二个 Mutex 并进行操作
        {
            let mut num2 = counter2.lock().unwrap();
            *num2 += 2 * i;
            println!("Iteration {}: counter2 is now {}", i, *num2);
        } // 第二个 Mutex 在这里解锁
    }

    // 打印最终结果
    let final_count1 = *counter1.lock().unwrap();
    let final_count2 = *counter2.lock().unwrap();
    println!("Final count1: {}", final_count1);
    println!("Final count2: {}", final_count2);
}
