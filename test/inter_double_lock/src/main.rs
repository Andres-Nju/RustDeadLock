use std::sync::{Arc, Mutex};

struct GasPricer {
    value: u32,
}

impl GasPricer {
    fn recalibrate(&mut self) {
        self.value += 10; // 模拟重新校准
    }
}

struct Miner {
    gas_pricer: Mutex<GasPricer>,
}

impl Miner {
    fn update_price(&self, new_price: u32) {
        let mut gas_pricer = self.gas_pricer.lock().unwrap();
        gas_pricer.value = new_price;
        // 锁在此作用域结束时未显式释放
    }

    fn recalibrate_wrapper(&self) {
        // 这里再次锁定，导致死锁
        let mut gas_pricer = self.gas_pricer.lock().unwrap();
        gas_pricer.recalibrate();
    }

    fn perform_update_and_recalibrate(&self, new_price: u32) {
        self.update_price(new_price); // 第一次锁定
        self.recalibrate_wrapper();  // 第二次锁定，可能导致死锁
    }
}

fn main() {
    let miner = Miner {
        gas_pricer: Mutex::new(GasPricer { value: 100 }),
    };

    // 调用更新和重新校准的方法
    miner.perform_update_and_recalibrate(200);
    println!("Update and recalibrate completed successfully");
}
