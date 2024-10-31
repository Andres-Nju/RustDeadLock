use std::rc::Rc;

use rustc_hir::def_id::DefId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::lock::Lock;



#[derive(Debug)]
pub struct LockGraph {
    adjacency_list: FxHashMap<Lock, Vec<Lock>>,
}

impl LockGraph {
    pub fn new() -> Self {
        Self {
            adjacency_list: FxHashMap::default(),
        }
    }

    pub fn add_edge(&mut self, from: Lock, to: Lock) {
        self.adjacency_list.entry(from).or_default().push(to);
    }


    // pub fn find_all_cycles(&self) -> Vec<Vec<Lock>> {
    //     let mut stack: Vec<Lock> = Vec::new();
    //     let mut low_link: FxHashMap<Lock, usize> = FxHashMap::default();
    //     let mut index: FxHashMap<Lock, usize> = FxHashMap::default();
    //     let mut on_stack: FxHashSet<Lock> = FxHashSet::default();
    //     let mut cycles: Vec<Vec<Lock>> = Vec::new();
    //     let mut idx = 0;

    //     for lock in self.adjacency_list.keys() {
    //         if !index.contains_key(lock) {
    //             self.strongconnect(lock, &mut idx, &mut stack, &mut low_link, &mut index, &mut on_stack, &mut cycles);
    //         }
    //     }

    //     cycles
    // }

    // fn strongconnect(
    //     &self,
    //     lock: &Lock,
    //     idx: &mut usize,
    //     stack: &mut Vec<Lock>,
    //     low_link: &mut FxHashMap<Lock, usize>,
    //     index: &mut FxHashMap<Lock, usize>,
    //     on_stack: &mut FxHashSet<Lock>,
    //     cycles: &mut Vec<Vec<Lock>>,
    // ) {
    //     index.insert(lock.clone(), *idx);
    //     low_link.insert(lock.clone(), *idx);
    //     *idx += 1;
    //     stack.push(lock.clone());
    //     on_stack.insert(lock.clone());

    //     // 遍历邻接节点
    //     if let Some(neighbors) = self.adjacency_list.get(lock) {
    //         for neighbor in neighbors {
    //             if !index.contains_key(neighbor) {
    //                 self.strongconnect(neighbor, idx, stack, low_link, index, on_stack, cycles);
    //                 let low = low_link.get(neighbor).unwrap();
    //                 let current_low = low_link.get(lock).unwrap();
    //                 low_link.insert(lock.clone(), *low_link.get(lock).unwrap().min(low));
    //             } else if on_stack.contains(neighbor) {
    //                 // 发现环
    //                 let low = low_link.get(lock).unwrap();
    //                 low_link.insert(lock.clone(), *low_link.get(lock).unwrap().min(index.get(neighbor).unwrap()));
    //             }
    //         }
    //     }

    //     // 回溯
    //     if low_link.get(lock) == Some(&index[lock]) {
    //         let mut cycle: Vec<Lock> = Vec::new();
    //         while let Some(node) = stack.pop() {
    //             on_stack.remove(&node);
    //             cycle.push(node.clone());
    //             if node == *lock {
    //                 break;
    //             }
    //         }
    //         if cycle.len() > 1 {
    //             cycles.push(cycle);
    //         }
    //     }
    // }
}


#[cfg(test)]
mod test{
    use rustc_hir::def_id::DefIndex;

    use super::*;


    #[test]
    fn test_graph() {
        let mut graph = LockGraph::new();
        
    }
}