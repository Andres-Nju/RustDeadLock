use std::sync::{Mutex, Arc};
struct A{
    a: i32,
    b: u32,
    s: String,
}
fn main(){
    // _1:  @ std::sync::Arc<std::sync::Mutex<std::boxed::Box<i32, std::alloc::Global>>, std::alloc::Global> 
    // _2:  @ std::sync::Mutex<std::boxed::Box<i32, std::alloc::Global>> 
    // _3:  @ std::boxed::Box<i32, std::alloc::Global> 
    let a = String::from("123");
    let b = A{
        a: 1,
        b:2,
        s: a
    };
    let n = 1;
    let c = Arc::new(Mutex::new(123));
    let e = Mutex::new(n);
    let f = c.lock().unwrap();
}