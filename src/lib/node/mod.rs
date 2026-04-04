use std::{cell::RefCell, rc::Rc};

pub mod entity;
pub mod expression;
pub mod markup;

pub type NodeRef<N> = Rc<RefCell<N>>;

pub fn new<N>(node: N) -> NodeRef<N> {
    Rc::new(RefCell::new(node))
}

pub fn map<N: std::fmt::Debug, O, F>(node: NodeRef<N>, f: F) -> NodeRef<O>
where
    F: FnOnce(N) -> O,
{
    new(f(Rc::try_unwrap(node).unwrap().into_inner()))
}

pub fn unwrap<N: std::fmt::Debug>(node: NodeRef<N>) -> N {
    Rc::try_unwrap(node).unwrap().into_inner()
}
