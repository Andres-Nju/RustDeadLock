use std::sync::Mutex;

// 最底层的结构体，包含一个 Mutex 保护的值
struct Level3 {
    value: Mutex<i32>,
}

impl Level3 {
    fn new(value: i32) -> Self {
        Level3 {
            value: Mutex::new(value),
        }
    }
}

// 第二层结构体，包含一个 Level3 结构体
struct Level2 {
    level3: Level3,
}

impl Level2 {
    fn new(value: i32) -> Self {
        Level2 {
            level3: Level3::new(value),
        }
    }
}

// 顶层结构体，包含一个 Level2 结构体
struct Level1 {
    level2: Level2,
}

impl Level1 {
    fn new(value: i32) -> Self {
        Level1 {
            level2: Level2::new(value),
        }
    }
}

fn main() {
    let level1 = Level1::new(10);

    // 锁定最底层的 Level3 的 Mutex 并修改其值
    {
        let mut value = level1.level2.level3.value.lock().unwrap();
        *value = 42;
    }

    // 打印最终结果
    let final_value = level1.level2.level3.value.lock().unwrap();
    println!("Final value: {}", *final_value);
}
