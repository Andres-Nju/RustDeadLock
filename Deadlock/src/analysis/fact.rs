use std::rc::Rc;

use super::{alias::{AliasFact, AliasSet, VariableNode}, lock::{LockGuard, LockSetFact}};



pub trait MapFact<K, V>{
    fn update(&mut self, key: K, value: V);
    fn meet(&mut self, other: Self);
}

impl MapFact<usize, (Rc<VariableNode>, Rc<AliasSet>)> for AliasFact{
    fn update(&mut self, key: usize, value: (Rc<VariableNode>, Rc<AliasSet>)){
        self.insert(key, value);
    }

    fn meet(&mut self, other: Self){
        for (index, (node, pre_set)) in other.into_iter(){
            let new_alias_set = AliasSet::new();
            for item in pre_set.as_ref().variables.borrow().iter(){
                new_alias_set.add_variable(item.clone());
            }
            if let Some((_, cur_set)) = self.get(&index){
                for item in cur_set.as_ref().variables.borrow().iter(){
                    new_alias_set.add_variable(item.clone());
                }
            }
            self.insert(index, (node.clone(), new_alias_set));
        }
    }
}

impl MapFact<usize, Rc<LockGuard>> for LockSetFact {
    fn update(&mut self, key: usize, value: Rc<LockGuard>) {
        self.insert(key, value);
    }

    fn meet(&mut self, other: Self) {
        for (index, guard) in other.into_iter(){
            self.update(index, guard);
        }
    }
}