use std::sync::{Mutex, Arc};

fn main() {
    // 创建两个由 Mutex 保护的资源
    let resource1 = Arc::new(Mutex::new(0));
    let resource2 = Arc::new(Mutex::new(0));

    // 条件变量，用于选择锁定哪一个资源
    let condition = true; // 可以改变此值以触发不同分支
    let mut res;
    if condition {
        // 如果条件为真，锁定 resource1
        res = resource1.lock().unwrap();
        *res += 10;
        println!("Condition is true, resource1 updated to: {}", *res);
    } else {
        // 如果条件为假，锁定 resource2
        res = resource2.lock().unwrap();
        *res += 20;
        println!("Condition is false, resource2 updated to: {}", *res);
    }
    let a = 10;
}
