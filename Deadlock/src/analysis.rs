use rustc_hash::{FxHashMap, FxHashSet};

use callgraph::CallGraph;
use rustc_hir::{def_id::DefId};
use rustc_middle::{mir::{BasicBlock, BasicBlockData, BasicBlocks, Successors}, ty::{self, TyCtxt}};
use lock::LockSetFact;
use rustc_middle::mir::{
    Location,
    Body,
    Statement,
    Terminator,
};

mod visitor;
pub mod callgraph;
pub mod lock;

pub struct LockSetAnalysis<'tcx>{
    tcx: TyCtxt<'tcx>, 
    // call_graph: CallGraph<'tcx>,
    // a DefId + bb index pair determines a bb
    lock_set_facts: FxHashMap<(DefId, usize), LockSetFact>,
}

impl<'tcx> LockSetAnalysis<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, call_graph: CallGraph<'tcx>) -> Self{
        Self{
            tcx,
            // call_graph,
            lock_set_facts: FxHashMap::default(),
        }
    }

    pub fn run_analysis(&mut self){
        for mir_key in self.tcx.mir_keys(()){
            let def_id = mir_key.to_def_id();
            // let body = self.tcx.instance_mir(ty::InstanceDef::Item(def_id));
            let body = &self.tcx.mir_built(def_id.as_local().unwrap()).steal();
            println!("{:?}, {:?}", body.span, self.tcx.def_path_str(def_id));
            self.intra_lock_set_analysis(def_id, body);
        }
    }
    pub fn intra_lock_set_analysis(&mut self, def_id: DefId, body: &Body){
        let mut work_list: Vec<usize> = (0..body.basic_blocks.len()).collect();
        while !work_list.is_empty(){
            let current_bb_index = work_list.pop().expect("Elements in non-empty work_list should always be valid!");
            let current_bb_data = &body.basic_blocks[BasicBlock::from(current_bb_index)];
            if self.visit_bb(def_id, current_bb_index, &body.basic_blocks){
                match &current_bb_data.terminator().kind{
                    rustc_middle::mir::TerminatorKind::Goto { target } => {
                        work_list.push(target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::SwitchInt { discr, targets } => {
                        for bb in targets.all_targets(){
                            work_list.push(bb.as_usize());
                        }
                    },
                    rustc_middle::mir::TerminatorKind::UnwindResume => (),
                    rustc_middle::mir::TerminatorKind::UnwindTerminate(_) => (),
                    rustc_middle::mir::TerminatorKind::Return => (),
                    rustc_middle::mir::TerminatorKind::Unreachable => (),
                    rustc_middle::mir::TerminatorKind::Drop { target, .. } => {
                        work_list.push(target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::Call { func, args, destination, target, unwind, call_source, fn_span } => {
                        // TODO: interprocedural
                    },
                    rustc_middle::mir::TerminatorKind::Assert { target, .. } => {
                        work_list.push(target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::Yield { .. } => (),
                    rustc_middle::mir::TerminatorKind::CoroutineDrop => (),
                    rustc_middle::mir::TerminatorKind::FalseEdge { real_target, .. } => {
                        work_list.push(real_target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::FalseUnwind { real_target, .. } => {
                        work_list.push(real_target.as_usize());
                    },
                    rustc_middle::mir::TerminatorKind::InlineAsm { destination, ..} => {
                        if let Some(bb) = destination{
                            work_list.push(bb.as_usize());
                        }
                    },
                }
            }
        }
        println!("bb number: {}", work_list.len());
    }

    pub fn visit_bb(&mut self, def_id: DefId, bb_index: usize, bbs: &BasicBlocks) -> bool{
        // TODO: maybe clean up bb?
        let mut flag = false;
        // if fact[bb] is none, initialize one
        self.lock_set_facts.entry((def_id, bb_index)).or_insert_with(|| {
            flag = true;
            FxHashSet::default()
        });
        // traverse the bb's statements
        let current_bb_data = &bbs[BasicBlock::from(bb_index)];
        current_bb_data.statements.iter().for_each(|statement| self.visit_statement(statement));
        flag
    }

    pub fn visit_statement(&mut self, statement: &Statement){
        match &statement.kind{
            rustc_middle::mir::StatementKind::Assign(_) => todo!(),
            rustc_middle::mir::StatementKind::FakeRead(_) => todo!(),
            rustc_middle::mir::StatementKind::SetDiscriminant { place, variant_index } => todo!(),
            rustc_middle::mir::StatementKind::Deinit(_) => todo!(),
            rustc_middle::mir::StatementKind::StorageLive(_) => todo!(),
            rustc_middle::mir::StatementKind::StorageDead(_) => todo!(),
            rustc_middle::mir::StatementKind::Retag(_, _) => todo!(),
            rustc_middle::mir::StatementKind::PlaceMention(_) => todo!(),
            rustc_middle::mir::StatementKind::AscribeUserType(_, _) => todo!(),
            rustc_middle::mir::StatementKind::Coverage(_) => todo!(),
            rustc_middle::mir::StatementKind::Intrinsic(_) => todo!(),
            rustc_middle::mir::StatementKind::ConstEvalCounter => todo!(),
            rustc_middle::mir::StatementKind::Nop => todo!(),
        }
    }
}

