use std::rc::Rc;
use std::cell::RefCell;
fn main(){
    let a = Rc::new(RefCell::new(String::from("123")));
    let b = a.clone();
    println!("a = {:?}", a);
    println!("b = {:?}", b);
    {
        let mut c = b.borrow_mut();
        *c += "123";
       // println!("c = {:?}", c);
    }
    tt(a);
    println!("a = {:?}", a);
    println!("b = {:?}", b);
}

fn tt(a: Rc<RefCell<String>>){
    
}