use std::sync::{Mutex, Arc};

fn main(){
    let a = Arc::new(Mutex::new(123));
    test(&a);
    testing(a);

    let b = Arc::new(Mutex::new(123));
    test(&b);
    testing(b);

    let c = Arc::new(Mutex::new(123));
    test(&c);
    testing(c);
}

fn testing(a: Arc<Mutex<i32>>){
    a.lock();
}

fn test(a: &Arc<Mutex<i32>>){
    a.lock();
}