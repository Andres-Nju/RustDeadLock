use std::{fmt::Debug, hash::Hash};

use rustc_hir::def_id::DefId;
use rustc_middle::mir::{Location, Operand, Place};

// (caller, location) can define an unique call
type CallSite = (DefId, Location);
/// a call is in this format:
/// ret = call fun_id(arg1, arg2, ...);
#[derive(Debug)]
pub struct Call<'tcx>{
    call_site: CallSite,
    callee: DefId,
    ret: Place<'tcx>,
    /// args
    args: Vec<Operand<'tcx>>,
}


impl<'tcx> Call<'tcx>{
    pub fn new(call_site: CallSite, callee: DefId, ret: Place<'tcx>, args: Vec<Operand<'tcx>>) -> Self{
        Call{
            call_site,
            callee,
            ret,
            args,
        }
    }
}

impl<'tcx> PartialEq for Call<'tcx>{
    fn eq(&self, other: &Self) -> bool {
        self.call_site == other.call_site
    }
}

impl<'tcx> Eq for Call<'tcx>{}

impl<'tcx> Hash for Call<'tcx>{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.call_site.hash(state);
    }
}

pub struct CallGraphNode<'tcx>{
    /// call site: (caller id, location in caller)
    /// entry's call site is None
    call_site: Option<CallSite>,
    /// callee
    def_id: DefId,
    /// calls in the callee
    calls: Vec<Call<'tcx>>,
}

impl<'tcx> Hash for CallGraphNode<'tcx>{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.call_site.hash(state);
    }
}

impl<'tcx> PartialEq for CallGraphNode<'tcx>{
    fn eq(&self, other: &Self) -> bool {
        self.call_site == other.call_site
    }
}

impl<'tcx> Eq for CallGraphNode<'tcx>{}

impl<'tcx> Debug for CallGraphNode<'tcx>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallGraphNode").
        field("call_site", &self.call_site).
        field("def_id", &self.def_id).
        // field("calls", &self.calls)
        finish()
    }
}

impl<'tcx> CallGraphNode<'tcx>{
    pub fn new(call_site: Option<(DefId, Location)>, def_id: DefId, calls: Vec<Call<'tcx>>) -> Self{
        CallGraphNode{
            call_site,
            def_id,
            calls,
        }
    }

    pub fn set_call_site(&mut self, caller: DefId, location: Location){
        self.call_site = Some((caller, location));
    }

    pub fn add_call(&mut self, call: Call<'tcx>){
        self.calls.push(call);
    }
}