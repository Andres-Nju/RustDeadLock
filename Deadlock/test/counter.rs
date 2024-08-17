use std::sync::{Arc, Mutex};
use std::thread;

struct Counter {
    count: i32,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    fn increment(&mut self) {
        self.count += 1;
        println!("Count after increment: {}", self.count);
    }

    fn get_count(&self) -> i32 {
        self.count
    }
}

fn do_work(counter: &mut Counter) {
    for _ in 0..5 {
        counter.increment();
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn main() {
    let counter = Counter::new();
    let counter = Arc::new(Mutex::new(counter));

    let mut handles = vec![];

    for _ in 0..3 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut counter = counter.lock().unwrap();
            do_work(&mut counter);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_count = counter.lock().unwrap().get_count();
    println!("Final count: {}", final_count);
}
