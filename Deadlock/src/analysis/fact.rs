use std::rc::Rc;

use super::{alias::{AliasFact, AliasSet, VariableNode}, lock::{LockFact, LockGuard, LockSetFact, LockSummary}};



pub trait MapFact<K, V>{
    fn update(&mut self, key: K, value: V);
    fn meet(&mut self, other: Self);
}

pub trait SetFact<V>{
    fn update(&mut self, value: V);
    fn meet(&mut self, other: &Self);
}

pub trait VecFact<V>{
    fn update(&mut self, value: V);
    fn meet(&mut self, other: &Self);
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

impl SetFact<Rc<LockFact>> for LockSetFact{
    fn update(&mut self, value: Rc<LockFact>) {
        self.insert(value);
    }

    fn meet(&mut self, other: &Self) {
        for item in other.clone(){
            self.insert(item);
        }
    }
}

impl VecFact<LockSetFact> for LockSummary{
    fn update(&mut self, value: LockSetFact) {
        self.push(value);
    }

    fn meet(&mut self, other: &Self) {
        let min_len = std::cmp::min(self.len(), other.len());
        for i in 0..min_len{
            self[i].extend(other[i].iter().cloned());
        }
        if other.len() > min_len{
            self.extend(other[min_len..].iter().cloned());
        }
    }
}