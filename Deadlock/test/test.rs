use std::sync::Arc;
struct A{
    a: i32,
}
fn main(){
    let mut aa = A{a: 10};
    aa.a = ttt(100);
}
fn ttt(_a: i32) -> i32{
    _a
}