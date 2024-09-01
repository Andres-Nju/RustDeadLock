fn main(){
    let a = String::from("123");
    let b = &a;
    tt(b);
}

fn tt(b: &String){
    let c = b.clone();
}