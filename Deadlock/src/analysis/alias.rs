use rustc_middle::ty::Ty;

#[derive(Debug, Clone)]
pub struct Owned<'tcx>{
    pub index: usize,
    pub ty: Ty<'tcx>,
}

#[derive(Debug, Clone)]
pub struct Ref{
    pub index: usize,
    pub point_to: usize,
}

#[derive(Debug, Clone)]
pub enum Node<'tcx>{
    Owned(Owned<'tcx>),
    Ref(Ref),
}

impl<'tcx> Owned<'tcx> {
    pub fn new(index: usize, ty: Ty<'tcx>) -> Self {
        Owned { index, ty }
    }
}

impl Ref {
    pub fn new(index: usize, point_to: usize) -> Self {
        Ref { index, point_to }
    }
}

impl<'tcx> Node<'tcx> {
    pub fn new_owned(index: usize, ty: Ty<'tcx>) -> Self {
        Node::Owned(Owned::new(index, ty))
    }

    pub fn new_ref(index: usize, point_to: usize) -> Self {
        Node::Ref(Ref::new(index, point_to))
    }

    pub fn set_index(&mut self, index: usize){
        match self{
            Node::Owned(o) => o.index = index,
            Node::Ref(r) => r.index = index,
        }
    }
}
