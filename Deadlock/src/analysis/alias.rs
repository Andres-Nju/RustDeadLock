use std::rc::Rc;

use graph::AliasGraph;
use node::{set_local_id, AliasGraphNode, EdgeLabel, GraphNodeId};
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_hir::{
    def_id::{DefId, LocalDefId},
    definitions::DefPathData,
};
use rustc_middle::{
    mir::{
        self, BasicBlock, Body, HasLocalDecls, Local, LocalDecls, Place, Rvalue, Statement,
        TerminatorKind,
    },
    ty::{Ty, TyCtxt},
};

use crate::context::MyTcx;

use super::{
    callgraph::{call_graph_node::Call, CallGraph},
    tools::{is_lock, is_mutex_method, is_smart_pointer},
};

pub mod graph;
pub mod node;

pub struct AliasAnalysis<'a, 'tcx> {
    my_tcx: &'a mut MyTcx<'tcx>,
    num_iteration: i32,
}

impl<'a, 'tcx> AliasAnalysis<'a, 'tcx> {
    pub fn new(my_tcx: &'a mut MyTcx<'tcx>) -> Self {
        Self {
            my_tcx,
            num_iteration: 1,
        }
    }

    pub fn run_analysis(&mut self) {
        self.before_run();

        self.intra_procedural_analysis();
        self.inter_procedural_analysis();

        self.after_run();
    }

    fn before_run(&mut self) {
        tracing::info!("Start alias analysis");
    }

    fn after_run(&self) {
        tracing::info!("Finish alias analysis");
    }

    fn init_func(&mut self, def_id: &DefId, body: &Body) {
        // compute the confrol flow graph in a reverse post-order
        let mut reverse_post_order = vec![];
        for bb in body.basic_blocks.reverse_postorder() {
            if !body.basic_blocks.get(*bb).unwrap().is_cleanup {
                reverse_post_order.push(bb.clone());
            }
        }
        self.my_tcx
            .control_flow_graph
            .entry(def_id.clone())
            .or_insert(reverse_post_order);
        set_local_id(body.local_decls.len());
        // create node for each parameter
        for index in 0..body.arg_count {
            self.my_tcx
                .alias_graph
                .get_or_insert_node(GraphNodeId::new(def_id.clone(), Some(index + 1)));
        }
    }

    fn intra_procedural_analysis(&mut self) {
        // traverse the functions in a reversed topo order
        for def_id in self.my_tcx.call_graph.topo.clone() {
            if self.my_tcx.tcx.is_mir_available(def_id) {
                if def_id.is_local() && self.my_tcx.control_flow_graph.get(&def_id) == None {
                    // each function is analyzed only once
                    // println!(
                    //     "Now analyze function {:?}'s alias information",
                    //     self.my_tcx.tcx.def_path_str(def_id)
                    // );
                    let body = self.my_tcx.tcx.optimized_mir(def_id);
                    // only analyze functions defined in current crate
                    // FIXME: closure?
                    self.visit_body(def_id, body);
                }
            }
        }
    }

    fn visit_body(&mut self, def_id: DefId, body: &Body<'tcx>) {
        self.init_func(&def_id, body);
        // FIXME: redundant clone
        for current_bb_index in self.my_tcx.control_flow_graph[&def_id].clone() {
            // println!("bb {:?} now under alias analysis ", current_bb_index);
            self.visit_bb(def_id, current_bb_index.as_usize(), body);
        }
    }

    fn visit_bb(&mut self, def_id: DefId, bb_index: usize, body: &Body<'tcx>) {
        let data = &body.basic_blocks[BasicBlock::from(bb_index)];

        // traverse the bb's statements
        data.statements.iter().for_each(|statement| {
            self.visit_statement(def_id, bb_index, statement, body.local_decls())
        });
        // process the terminator
        self.visit_terminator(&def_id, bb_index, &data.terminator().kind, body);
    }

    fn visit_statement(
        &mut self,
        def_id: DefId,
        bb_index: usize,
        statement: &Statement<'tcx>,
        decls: &LocalDecls,
    ) {
        match &statement.kind {
            rustc_middle::mir::StatementKind::Assign(ref assign) => {
                //if is_lock(&decls[Local::from_usize(left)].ty) {
                self.visit_assign(&def_id, bb_index, &assign.0, &assign.1);
                //}
            }
            rustc_middle::mir::StatementKind::FakeRead(_) => (),
            rustc_middle::mir::StatementKind::SetDiscriminant { .. } => (),
            rustc_middle::mir::StatementKind::Deinit(_) => (),
            rustc_middle::mir::StatementKind::StorageLive(_) => (),
            rustc_middle::mir::StatementKind::StorageDead(_) => (),
            rustc_middle::mir::StatementKind::Retag(_, _) => (),
            rustc_middle::mir::StatementKind::PlaceMention(_) => (),
            rustc_middle::mir::StatementKind::AscribeUserType(_, _) => (),
            rustc_middle::mir::StatementKind::Coverage(_) => (),
            rustc_middle::mir::StatementKind::Intrinsic(_) => (),
            rustc_middle::mir::StatementKind::ConstEvalCounter => (),
            rustc_middle::mir::StatementKind::Nop => (),
        }
    }

    fn visit_assign(&mut self, def_id: &DefId, bb_index: usize, lhs: &Place, rhs: &Rvalue<'tcx>) {
        // resolve rhs

        match rhs {
            Rvalue::Use(op) => match op {
                mir::Operand::Copy(p) | mir::Operand::Move(p) => {
                    self.visit_copy_or_move(def_id, lhs, p);
                }
                mir::Operand::Constant(_) => {
                    self.visit_constant(def_id, lhs);
                }
            },
            Rvalue::AddressOf(_, p) | Rvalue::Ref(_, _, p) => {
                self.visit_address_of_or_ref(def_id, lhs, p);
            }
            Rvalue::Repeat(_, _) => todo!(),
            Rvalue::ThreadLocalRef(_) => todo!(),
            Rvalue::Len(_) => todo!(),
            Rvalue::Cast(_, _, _) => (),
            Rvalue::Discriminant(p) => self.visit_copy_or_move(def_id, lhs, p),
            Rvalue::Aggregate(_, _) => (), // TODO: 直接创建struct时
            Rvalue::ShallowInitBox(_, _) => todo!(),
            Rvalue::CopyForDeref(p) => {
                self.visit_copy_or_move(def_id, lhs, p);
            }
            _ => (),
        }
    }

    fn visit_constant(&mut self, def_id: &DefId, lhs: &Place) {
        self.my_tcx.alias_graph.resolve_project(def_id, lhs);
    }

    fn visit_copy_or_move(&mut self, def_id: &DefId, lhs: &Place, rhs: &Place) {
        let node_x = self.my_tcx.alias_graph.resolve_project(def_id, lhs);
        let node_y = self.my_tcx.alias_graph.resolve_project(def_id, rhs);
        self.make_alias(node_x, node_y);
    }

    fn visit_address_of_or_ref(&mut self, def_id: &DefId, lhs: &Place, rhs: &Place) {
        let node_x = self.my_tcx.alias_graph.resolve_project(def_id, lhs);
        let node_y = self.my_tcx.alias_graph.resolve_project(def_id, rhs);
        unsafe {
            (*node_x).add_target(node_y, EdgeLabel::Deref);
        }
    }

    fn visit_terminator(
        &mut self,
        def_id: &DefId,
        bb_index: usize,
        terminator_kind: &TerminatorKind<'tcx>,
        body: &Body<'tcx>,
    ) {
        match terminator_kind {
            // TODO: if return a lock?
            rustc_middle::mir::TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                unwind,
                call_source,
                fn_span,
            } => {
                match func {
                    mir::Operand::Constant(constant) => {
                        match constant.ty().kind() {
                            rustc_type_ir::TyKind::FnDef(fn_id, _) => {
                                // _* = func(args) -> [return: bb*, unwind: bb*] @ Call: FnDid: *
                                let def_path = self.my_tcx.tcx.def_path(fn_id.clone());
                                let def_path_str = self.my_tcx.tcx.def_path_str(fn_id);
                                if let DefPathData::ValueNs(name) =
                                    &def_path.data[def_path.data.len() - 1].data
                                {
                                    if is_mutex_method(&def_path_str) {
                                        if name.as_str() == "new" {
                                            assert_eq!(1, args.len());
                                            match &args[0].node {
                                                mir::Operand::Copy(_) => {
                                                    panic!("should not go to this branch!");
                                                }
                                                mir::Operand::Constant(_)
                                                | mir::Operand::Move(_) => {
                                                    self.my_tcx
                                                        .alias_graph
                                                        .resolve_project(def_id, destination);
                                                }
                                            }
                                        } else if name.as_str() == "lock" {
                                            assert_eq!(1, args.len());
                                            match &args[0].node {
                                                // must be move _*
                                                mir::Operand::Constant(_) => todo!(),
                                                mir::Operand::Copy(p) | mir::Operand::Move(p) => {
                                                    // if the lock_ref is from the parameters, lock_ref may have no out_vertices
                                                    let guard = self
                                                        .my_tcx
                                                        .alias_graph
                                                        .resolve_project(def_id, destination);
                                                    let lock_ref = self
                                                        .my_tcx
                                                        .alias_graph
                                                        .resolve_project(def_id, p);
                                                    // guard = mutex::lock( lock_ref )
                                                    // lock_ref is &mutex, so need to get its deref target
                                                    unsafe {
                                                        if let Some(lock) = (*lock_ref)
                                                            .get_out_vertex(&EdgeLabel::Deref)
                                                        {
                                                            (*guard).add_target(
                                                                lock,
                                                                EdgeLabel::from("Guard"),
                                                            );
                                                        } else {
                                                            let lock = self
                                                                .my_tcx
                                                                .alias_graph
                                                                .get_or_insert_node(
                                                                    GraphNodeId::new(
                                                                        def_id.clone(),
                                                                        None,
                                                                    ),
                                                                );
                                                            (*lock_ref)
                                                                .add_target(lock, EdgeLabel::Deref);
                                                            (*guard).add_target(
                                                                lock,
                                                                EdgeLabel::from("Guard"),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else if is_smart_pointer(&def_path_str) {
                                        if name.as_str() == "new" {
                                            // the same as ref assign
                                            assert_eq!(1, args.len());
                                            match &args[0].node {
                                                mir::Operand::Constant(_)
                                                | mir::Operand::Copy(_) => {
                                                    panic!("should not go to this branch!");
                                                }
                                                mir::Operand::Move(p) => {
                                                    let smart_ptr = self
                                                        .my_tcx
                                                        .alias_graph
                                                        .resolve_project(def_id, destination);
                                                    let val = self
                                                        .my_tcx
                                                        .alias_graph
                                                        .resolve_project(def_id, p);
                                                    unsafe {
                                                        (*smart_ptr)
                                                            .add_target(val, EdgeLabel::Deref);
                                                    }
                                                }
                                            }
                                        }
                                    } else if name.as_str() == "unwrap" {
                                        assert_eq!(1, args.len());
                                        match &args[0].node {
                                            // must be move _*
                                            mir::Operand::Constant(_) => {
                                                self.visit_constant(def_id, destination)
                                            }
                                            mir::Operand::Move(p) | mir::Operand::Copy(p) => {
                                                let unwrap = self
                                                    .my_tcx
                                                    .alias_graph
                                                    .resolve_project(def_id, destination);
                                                let unwraped = self
                                                    .my_tcx
                                                    .alias_graph
                                                    .resolve_project(def_id, p);
                                                self.make_alias(unwraped, unwrap);
                                            }
                                        }
                                    }
                                    // else if name.as_str() == "deref"{
                                    // }
                                    // todo: maybe problematic here
                                    else if name.as_str() == "clone" || name.as_str() == "deref" {
                                        assert_eq!(1, args.len());
                                        match &args[0].node {
                                            // must be copy _*
                                            mir::Operand::Constant(_) => {
                                                self.visit_constant(def_id, destination)
                                            }
                                            mir::Operand::Move(p) | mir::Operand::Copy(p) => {
                                                // clone = mutex::lock( cloned_ref )
                                                // cloned_ref is &be_cloned, so need to get its deref target
                                                let clone = self
                                                    .my_tcx
                                                    .alias_graph
                                                    .resolve_project(def_id, destination);
                                                let cloned_ref = self
                                                    .my_tcx
                                                    .alias_graph
                                                    .resolve_project(def_id, p);
                                                unsafe {
                                                    // if the clone_ref is from the parameters, clone_ref may have no out_vertices
                                                    if let Some(cloned) = (*cloned_ref)
                                                        .get_out_vertex(&EdgeLabel::Deref)
                                                    {
                                                        self.make_alias(cloned, clone);
                                                    } else {
                                                        let cloned = self
                                                            .my_tcx
                                                            .alias_graph
                                                            .get_or_insert_node(GraphNodeId::new(
                                                                def_id.clone(),
                                                                None,
                                                            ));
                                                        (*cloned_ref)
                                                            .add_target(cloned, EdgeLabel::Deref);
                                                        self.make_alias(cloned, clone);
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        let call = Call::new(
                                            (
                                                def_id.clone(),
                                                body.terminator_loc(BasicBlock::from_usize(
                                                    bb_index,
                                                )),
                                            ),
                                            fn_id.clone(),
                                            destination.clone(),
                                            args.iter().map(|span| span.node.clone()).collect(),
                                        );
                                        // self.call_graph.add_call(def_id.clone(), call);
                                    }
                                }
                            }
                            // maybe problematic
                            rustc_type_ir::TyKind::FnPtr(_) => panic!("TODO: FnPtr"),
                            rustc_type_ir::TyKind::Closure(_, _) => panic!("TODO: closure"),
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }
            rustc_middle::mir::TerminatorKind::InlineAsm { .. } => {}
            _ => {}
        }
    }

    fn inter_procedural_analysis(&mut self) {
        let mut iteration_count = 0;
        loop {
            if iteration_count >= self.num_iteration {
                break;
            }
            // todo: redundant clone
            for (def_id, call_set) in self.my_tcx.call_graph.calls_map.clone().iter() {
                for call in call_set {
                    let callee = call.callee();
                    let ret = call.ret();
                    // 1. add ret's constrain: ret in caller = callee()
                    let ret_node = self.my_tcx.alias_graph.resolve_project(def_id, ret);
                    let callee_ret = self
                        .my_tcx
                        .alias_graph
                        .get_or_insert_node(GraphNodeId::new(callee.clone(), Some(0)));
                    self.make_alias(ret_node, callee_ret);
                    // 2. add args' constrain
                    // assert_eq!(call.args().len(), self.tcx.optimized_mir(callee).arg_count);
                    for (index, arg) in call.args().iter().enumerate() {
                        match arg {
                            mir::Operand::Copy(p) | mir::Operand::Move(p) => {
                                let arg = self.my_tcx.alias_graph.resolve_project(def_id, p);
                                let param = self.my_tcx.alias_graph.get_or_insert_node(
                                    GraphNodeId::new(callee.clone(), Some(index + 1)),
                                );
                                self.make_alias(param, arg);
                            }
                            mir::Operand::Constant(_) => (),
                        }
                    }
                }
            }
            self.my_tcx.alias_graph.qirun_algorithm();
            iteration_count += 1;
        }
    }

    fn make_alias(
        &mut self,
        node_x: *mut AliasGraphNode,
        node_y: *mut AliasGraphNode,
    ) -> *mut AliasGraphNode {
        self.my_tcx.alias_graph.combine(node_x, node_y)
    }
}
