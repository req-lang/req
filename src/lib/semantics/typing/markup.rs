use std::rc::Rc;

use crate::{
    node::{self, ConversionMap},
    reference::Reference,
    semantics::{
        reference::{self, WeakNodeRef},
        typing::entity,
    },
};

pub type Node = node::markup::Node<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;

pub(super) fn init_node(
    node: reference::markup::Node,
    entities: &ConversionMap<reference::entity::Node, entity::Node>,
) -> Node {
    node.mapr(|r| r.map(|weak| Rc::downgrade(&entities[&weak.as_ptr()])))
}
