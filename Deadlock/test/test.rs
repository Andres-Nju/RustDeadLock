use std::sync::Mutex;
fn main(){
    let a = Mutex::new(123);
    // println!("{:?}", a);
    let b = Mutex::new(24);
    let a1 = a.lock().unwrap();
    let b1 = b.lock().unwrap();
    let c;
    if true{
        c = a1;
    }
    else{
        c = b1;
    }
    println!("{:?}", c);
}

fn tt(b: &Mutex<i32>){
    tt2(b);
}

fn tt2(b: &Mutex<i32>){
    
}