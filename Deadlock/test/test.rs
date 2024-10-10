struct A<'a>{
    a: &'a B
}

struct B{
    b: i32
}

fn main(){
    let a = Box::new(123);
    let b = *a;
}