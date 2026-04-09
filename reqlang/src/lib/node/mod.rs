use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::utils::children::ChildrenIter;

pub mod entity;
pub mod expression;
pub mod markup;

pub type NodeRef<N> = Rc<RefCell<N>>;
pub type WeakNodeRef<N> = Weak<RefCell<N>>;

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

pub type ConversionMap<N, O> = HashMap<*const RefCell<N>, NodeRef<O>>;

pub fn build_stub_map<N, O>(node: &NodeRef<N>, map: &mut ConversionMap<N, O>)
where
    O: Default,
    N: ChildrenIter<NodeRef<N>>,
{
    let key = Rc::as_ptr(node);
    let stub = new(O::default());
    map.insert(key, stub);
    for child in node.borrow().children_iter() {
        build_stub_map(&child, map);
    }
}
