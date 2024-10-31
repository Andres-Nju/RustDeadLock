use super::lock::{LockFact, LockSetFact, LockSummary};



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



impl SetFact<LockFact> for LockSetFact{
    fn update(&mut self, value: LockFact) {
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