use std::sync::{Mutex, Arc};


fn main(){
    let mutex1 = Arc::new(Mutex::new(0));
    let a = mutex1.lock().unwrap();
    println!("{}", a);
}

