use std::rc::Rc;

use rustc_hir::def_id::DefId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::lock::Lock;



#[derive(Debug)]
pub struct LockGraph {
    adjacency_list: FxHashMap<Lock, Vec<Lock>>,
    self_loops: FxHashSet<Lock>,
}

impl LockGraph {
    pub fn new() -> Self {
        Self {
            adjacency_list: FxHashMap::default(),
            self_loops: FxHashSet::default(),
        }
    }

    pub fn add_edge(&mut self, from: Lock, to: Lock) {
        if from == to {
            self.self_loops.insert(from.clone()); // 记录自环
        } else {
            self.adjacency_list
                .entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to);
        }
    }

    pub fn print_loops(&self){
        for lo in self.find_all_cycles(){
            println!("loop: {:?}",lo);
        }

        for se_lo in self.self_loops.iter(){
            println!("self loop: {:?}",se_lo);
        }
    }


    pub fn find_all_cycles(&self) -> Vec<Vec<Lock>> {
        let mut cycles: Vec<Vec<Lock>> = Vec::new();
        let mut stack: Vec<Lock> = Vec::new();
        let mut on_stack: FxHashSet<Lock> = FxHashSet::default();
        let mut visited: FxHashSet<Lock> = FxHashSet::default();

        for start_node in self.adjacency_list.keys() {
            if !visited.contains(start_node) {
                self.dfs(start_node.clone(), &mut stack, &mut on_stack, &mut visited, &mut cycles);
            }
        }

        cycles
    }

    fn dfs(
        &self,
        node: Lock,
        stack: &mut Vec<Lock>,
        on_stack: &mut FxHashSet<Lock>,
        visited: &mut FxHashSet<Lock>,
        cycles: &mut Vec<Vec<Lock>>,
    ) {
        visited.insert(node.clone());
        stack.push(node.clone());
        on_stack.insert(node.clone());

        if let Some(neighbors) = self.adjacency_list.get(&node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs(neighbor.clone(), stack, on_stack, visited, cycles);
                } else if on_stack.contains(neighbor) {
                    // 找到一个环
                    let mut cycle: Vec<Lock> = Vec::new();
                    let mut index = stack.iter().rposition(|x| *x == *neighbor).unwrap();
                    while index < stack.len() {
                        cycle.push(stack[index].clone());
                        index += 1;
                    }
                    cycles.push(cycle);
                }
            }
        }

        stack.pop();
        on_stack.remove(&node);
    }

    pub fn get_self_loops(&self) -> &FxHashSet<Lock> {
        &self.self_loops
    }
}


#[cfg(test)]
mod test{
    use rustc_hir::def_id::{CrateNum, DefIndex};

    use super::*;


    #[test]
    fn test_graph() {
        let mut graph = LockGraph::new();
        let lock0 = Lock::new(DefId{ index: DefIndex::from_u32(0), krate: CrateNum::from_u32(1) }, 0);
        let lock1 = Lock::new(DefId{ index: DefIndex::from_u32(1), krate: CrateNum::from_u32(2) }, 1);
        let lock2 = Lock::new(DefId{ index: DefIndex::from_u32(2), krate: CrateNum::from_u32(3) }, 2);
        let lock3 = Lock::new(DefId{ index: DefIndex::from_u32(3), krate: CrateNum::from_u32(4) }, 3);

        graph.add_edge(lock0.clone(), lock1.clone());
        graph.add_edge(lock1.clone(), lock2.clone());
        graph.add_edge(lock2.clone(), lock1.clone());
        graph.add_edge(lock2.clone(), lock3.clone());
        graph.add_edge(lock3.clone(), lock0.clone());
        graph.add_edge(lock3.clone(), lock1.clone());
        graph.add_edge(lock0.clone(), lock0.clone());
        graph.add_edge(lock1.clone(), lock1.clone());

        // loop1: 0 -> 1 -> 2 -> 3 -> 0
        // loop2: 1 -> 2 -> 1
        // loop3: 1 -> 2 -> 3 -> 1
        for lo in graph.find_all_cycles(){
            println!("loop: {:?}",lo);
        }
        for se_lo in graph.self_loops.iter(){
            println!("self loop: {:?}",se_lo);
        }
    }
}