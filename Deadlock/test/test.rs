use std::sync::{Mutex, Arc};
struct A{
    s: Mutex<Box<i32>>,
}
fn main(){
    // _1:  @ std::sync::Arc<std::sync::Mutex<std::boxed::Box<i32, std::alloc::Global>>, std::alloc::Global> 
    // _2:  @ std::sync::Mutex<std::boxed::Box<i32, std::alloc::Global>> 
    // _3:  @ std::boxed::Box<i32, std::alloc::Global> 
    let mutex1 = Arc::new(Mutex::new(Box::new(123)));
    let a = A{
        s: Mutex::new(Box::new(123)),
    };
}