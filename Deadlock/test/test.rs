use std::sync::Mutex;
fn main(){
    let a = Mutex::new(123);
    // println!("{:?}", a);
    let g = a.lock().unwrap();
    let b  = String::from("123");
    tt(&a);
}

fn tt(b: &Mutex<i32>){
    tt2(b);
}

fn tt2(b: &Mutex<i32>){
    
}