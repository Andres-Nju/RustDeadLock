use std::sync::Mutex;
fn main(){
    let a = Mutex::new(123);
    t(&a);
}

fn t(a: &Mutex<i32>){
    a.lock();
}

// 这里会有一个小问题：形参是&Mutex
// 过程内分析时，找不到指向的Mutex，所以生成一个dummy node代替
// 过程间分析时，将该dummy node和实参合并进同一个集合
// 因此，最后建图的时候，会多出来一个dummy node对应的lock