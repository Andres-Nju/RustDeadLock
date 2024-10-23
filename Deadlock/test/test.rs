struct A<'a>{
    a: &'a B
}

#[derive(Copy)]
struct B{
    b: i32
}

struct C{
    c: Box<i32>,
}
fn main(){
    // // rvalue::deref_copy
    // // 当对一个projection >= 1的place进行deref时，会先将后面的place进行一个deref copy
    // // e.g. 1. *a.a, 其中a.a是一个reference
    // let b = B{b:1};
    // let a = A{a: &b};
    // let c = *a.a;
    // // e.g. *a.a, 其中a.a是一个Box
    // let d = C{c: Box::new(11)};
    // let e = *d.c;
    // // 对应的mir就是先做一个
    // // copy for deref: _1 = _0.0 
    // // 然后做一个
    // // use的deref: _2 = *_1
    
    
    // // 如果projection里出现了deref，一定是第一个，且一定是先做的
    // let b = B{b:1};
    // let a = A{a: &b};
    // let c = &a;
    // let d = *(*c).a;
    // let e = (*c).a;

    // let a = 1;
    // let b = &a;
    // let c = &b;
    // let d = **c;

    // 先deref后field
    let a = B{b: 1};
    let b = &a;
    let c = (*b).b;

    // *a = *b 就是正常的 *a = *b
    // let mut a = 10;
    // let b = &mut a;
    // let c = 1010;
    // let d = &c;
    // *b = *d;
}