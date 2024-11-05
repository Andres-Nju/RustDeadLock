use std::sync::{Arc, Mutex};

enum GasPricer {
    Fixed(u32),
    Calibrated(u32),
}

struct Miner {
    gas_pricer: Mutex<GasPricer>,
}

impl Miner {
    fn set_minimal_gas_price(&self, new_price: u32) -> Result<bool, &'static str> {
        // 第一次锁定 gas_pricer
        match *self.gas_pricer.lock().unwrap() {
            GasPricer::Fixed(_) => {
                // *val = new_price;
                // 第二次锁定 gas_pricer，造成死锁
                self.gas_pricer.lock().unwrap().recalibrate();
                Ok(true)
            },
            GasPricer::Calibrated(_) => {
                Err("Gas pricer already calibrated")
            },
        }
    }
}

// 为 GasPricer 定义一个 recalibrate 方法
impl GasPricer {
    fn recalibrate(&mut self) {
        match *self {
            GasPricer::Fixed(_) => {
                // *val += 10; // 模拟重新校准
            },
            GasPricer::Calibrated(_) => {}
        }
    }
}

fn main() {
    let miner = Miner {
        gas_pricer: Mutex::new(GasPricer::Fixed(100)),
    };

    // 尝试设置最低 gas 价格
    match miner.set_minimal_gas_price(200) {
        Ok(_) => println!("Set price successfully"),
        Err(_) => println!("Failed to set price"),
    };
}
