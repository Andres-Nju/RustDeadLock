trait MyTrait<T> {
    fn foo<U>(&self, a: String);
}

struct MyA<T>{
    a: T
}

impl MyTrait<i32> for MyA<i32>{
    fn foo<u32> (&self, a: String){
        println!("wcnm");
    }
}
fn main(){
    let a = MyA{a: 10};
    a.foo::<u32>("10".to_string());
}