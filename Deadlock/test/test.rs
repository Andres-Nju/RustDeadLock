use std::sync::Mutex;
fn main(){
    let a = Mutex::new(123);
    // println!("{:?}", a);
    let g = a.lock().unwrap();
}

fn tt(b: &Mutex<i32>){
    
}