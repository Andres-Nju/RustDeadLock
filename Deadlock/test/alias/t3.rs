use std::sync::{Mutex, Arc};

fn main(){
    let a = Arc::new(Mutex::new(123));
    let b = a.clone();
    let c = Arc::clone(&b);

    testing(a);
    testing(b);
    testing(c);
}

fn testing(a: Arc<Mutex<i32>>){
    a.lock();
}