/// copied from RAP: https://github.com/Artisan-Lab/RAP

use rustc_middle::ty::{self,TyCtxt,TyKind};
use rustc_hir::def_id::DefId;
use rustc_middle::mir::{Operand,Rvalue,Statement,StatementKind,TerminatorKind,BasicBlocks,
                        BasicBlockData,Body,LocalDecl,LocalDecls,Terminator};
use colorful::{Color,Colorful};

const NEXT_LINE:&str = "\n";
const PADDING:&str = "    ";
const EXPLAIN:&str = " @ ";

// This trait is a wrapper towards std::Display or std::Debug, and is to resolve orphan restrictions.
pub trait Display {
    fn display(&self) -> String;
}

impl<'tcx> Display for Terminator<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{:?}{}", PADDING, self.kind, self.kind.display());
        s
    }
}

impl<'tcx> Display for TerminatorKind<'tcx>{
    fn display(&self) -> String {
        let mut s = String::new();
        s += EXPLAIN;
        match &self {
            TerminatorKind::Goto { .. } =>
                s += "Goto",
            TerminatorKind::SwitchInt { .. } =>
                s += "SwitchInt",
            TerminatorKind::Return =>
                s += "Return",
            TerminatorKind::Unreachable =>
                s += "Unreachable",
            TerminatorKind::Drop { .. } =>
                s += "Drop",
            TerminatorKind::Assert { .. } =>
                s += "Assert",
            TerminatorKind::Yield { .. } =>
                s += "Yield",
            TerminatorKind::FalseEdge { .. } =>
                s += "FalseEdge",
            TerminatorKind::FalseUnwind { .. } =>
                s += "FalseUnwind",
            TerminatorKind::InlineAsm { .. } =>
                s += "InlineAsm",
            TerminatorKind::UnwindResume =>
                s += "UnwindResume",
            TerminatorKind::UnwindTerminate( .. ) =>
                s += "UnwindTerminate",
            TerminatorKind::Call { func, .. } => {
                match func {
                    Operand::Constant(constant) => {
                            match constant.ty().kind() {
                                ty::FnDef(id, args) =>
                                    {
                                        s += &format!("Call: FnDid: {}, args: {:?}", id.index.as_usize(), args).as_str();
                                        for arg in args.iter(){
                                            // s += &format!("\n arg is {:?}", arg.unpack());
                                        }
                                    }
                                _ => (),
                            }
                    },
                    _ => (),
                }
            }
            TerminatorKind::CoroutineDrop => todo!(),
        };
        s
    }
}

impl<'tcx> Display for Statement<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{:?}{}", PADDING, self.kind, self.kind.display());
        s
    }
}

impl<'tcx> Display for StatementKind<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += EXPLAIN;
        match &self {
            StatementKind::Assign(assign) => {
                s += &format!("{:?}={:?}{}", assign.0, assign.1, assign.1.display());
            }
            StatementKind::FakeRead( .. ) =>
                s += "FakeRead",
            StatementKind::SetDiscriminant { .. } =>
                s += "SetDiscriminant",
            StatementKind::Deinit( .. ) =>
                s += "Deinit",
            StatementKind::StorageLive( .. ) =>
                s += "StorageLive",
            StatementKind::StorageDead( .. ) =>
                s += "StorageDead",
            StatementKind::Retag( .. ) =>
                s += "Retag",
            StatementKind::AscribeUserType( .. ) =>
                s += "AscribeUserType",
            StatementKind::Coverage( .. ) =>
                s += "Coverage",
            StatementKind::Nop =>
                s += "Nop",
            StatementKind::PlaceMention( .. ) =>
                s += "PlaceMention",
            StatementKind::Intrinsic( .. ) =>
                s += "Intrinsic",
            StatementKind::ConstEvalCounter =>
                s += "ConstEvalCounter",
        }
        s
    }
}

impl<'tcx> Display for Rvalue<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += EXPLAIN;
        match self {
            Rvalue::Use( o ) =>{
                s += "Use";
                match o{
                    Operand::Copy(p) => {
                        s += " Copy";
                        s += format!("The type is {:?}", p.projection).as_str();
                    },
                    Operand::Move(p) => {
                        s += " Move";
                        s += format!("The type is {:?}", p.projection).as_str();
                    },
                    Operand::Constant(_) => (),
                }
            },
            Rvalue::Repeat( .. ) =>
                s += "Repeat",
            Rvalue::Ref(_, _, p ) =>{
                s += "Ref";
                s += format!("The type is {:?}", p.projection).as_str();
            }
                ,
            Rvalue::ThreadLocalRef( .. ) =>
                s += "ThreadLocalRef",
            Rvalue::AddressOf( .. ) =>
                s += "AddressOf",
            Rvalue::Len( .. ) =>
                s += "Len",
            Rvalue::Cast( .. ) =>
                s += "Cast",
            Rvalue::BinaryOp( .. ) =>
                s += "BinaryOp",
            Rvalue::CheckedBinaryOp( .. ) =>
                s += "CheckedBinaryOp",
            Rvalue::NullaryOp( .. ) =>
                s += "NullaryOp",
            Rvalue::UnaryOp( .. ) =>
                s += "UnaryOp",
            Rvalue::Discriminant( .. ) =>
                s += "Discriminant",
            Rvalue::Aggregate( .. ) =>
                s += "Aggregate",
            Rvalue::ShallowInitBox( .. ) =>
                s+= "ShallowInitBox",
            Rvalue::CopyForDeref( .. ) =>
                s+= "CopyForDeref",
        }
        s
    }
}

impl<'tcx> Display for BasicBlocks<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        for (index, bb) in self.iter().enumerate() {
            s += &format!("bb {} {{{}{}}}{}", index, NEXT_LINE, bb.display(), NEXT_LINE);
        }
        s
    }
}

impl<'tcx> Display for BasicBlockData<'tcx>  {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("CleanUp: {}{}", self.is_cleanup, NEXT_LINE);
        for stmt in self.statements.iter() {
            s += &format!("{}{}", stmt.display(), NEXT_LINE);
        }
        s += &format!("{}{}", self.terminator.clone().unwrap().display(), NEXT_LINE);
        s
    }
}

impl<'tcx> Display for LocalDecls<'tcx>  {
    fn display(&self) -> String {
        let mut s = String::new();
        for (index, ld) in self.iter().enumerate() {
            s += &format!("_{}: {} {}", index, ld.display(), NEXT_LINE);
        }
        s
    }
}

impl<'tcx> Display for LocalDecl<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{}{}", EXPLAIN, self.ty.kind().display());
        s
    }
}

impl<'tcx> Display for Body<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &self.local_decls.display();
        s += &self.basic_blocks.display();
        s
    }
}

impl<'tcx> Display for TyKind<'tcx> {
    fn display(&self) -> String {
        let mut s = String::new();
        s += &format!("{:?}", self);
        s
    }
}

impl Display for DefId {
    fn display(&self) -> String {
        format!("{:?}", self)
    }
}

pub struct ShowMir<'tcx> {
    pub tcx: TyCtxt<'tcx>,
}

#[inline(always)]
fn display_mir(did: DefId, body: &Body) {
    println!("{}", did.display());
    println!("{}", body.local_decls.display());
    println!("{:?}", body.var_debug_info);
    println!("{}", body.basic_blocks.display());
}

impl<'tcx> ShowMir<'tcx>{
    pub fn new(tcx: TyCtxt<'tcx>) -> Self{
        Self{
            tcx,
        }
    }

    pub fn start(&mut self) {
        println!("Show MIR");
       	let mir_keys = self.tcx.mir_keys(());
       	for each_mir in mir_keys {
            let def_id = each_mir.to_def_id();
            // let body = self.tcx.instance_mir(ty::InstanceDef::Item(def_id));
            // let body = &self.tcx.mir_built(def_id.as_local().unwrap()).steal();
            let body = self.tcx.optimized_mir(def_id);
            display_mir(def_id, body);
	    }
    }
}