use std::{fmt::Debug, hash::Hash};

use rustc_hir::def_id::DefId;
use rustc_middle::mir::Location;

/// a call is in this format:
/// ret = call fun_id(arg1, arg2, ...);
pub struct Call{
    /// index of ret
    ret: usize,
    /// args
    args: Vec<usize>,
}

impl Call{
    pub fn new(ret: usize, args: Vec<usize>) -> Self{
        Call{
            ret,
            args,
        }
    }
}

pub struct CallGraphNode{
    /// call site: (caller id, location in caller)
    /// entry's call site is None
    call_site: Option<(DefId, Location)>,
    /// callee
    def_id: DefId,
    /// calls in the callee
    calls: Vec<Call>,
}

impl Hash for CallGraphNode{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.call_site.hash(state);
    }
}

impl PartialEq for CallGraphNode{
    fn eq(&self, other: &Self) -> bool {
        self.call_site == other.call_site
    }
}

impl Eq for CallGraphNode{}

impl Debug for CallGraphNode{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallGraphNode").
        field("call_site", &self.call_site).
        field("def_id", &self.def_id).
        // field("calls", &self.calls)
        finish()
    }
}

impl CallGraphNode{
    pub fn new(call_site: Option<(DefId, Location)>, def_id: DefId, calls: Vec<Call>) -> Self{
        CallGraphNode{
            call_site,
            def_id,
            calls,
        }
    }

    pub fn set_call_site(&mut self, caller: DefId, location: Location){
        self.call_site = Some((caller, location));
    }

    pub fn add_call(&mut self, call: Call){
        self.calls.push(call);
    }
}