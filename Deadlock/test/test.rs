

struct A{
    a: i32
}

fn main(){
    let a = A{a: 10};
    let b = &a;
    let c = (*b).a;
}