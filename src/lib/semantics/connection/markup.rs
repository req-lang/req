use std::rc::Rc;

use crate::{
    node::{self, ConversionMap},
    reference::Reference,
    semantics::{
        connection::entity::{self},
        reference::WeakNodeRef,
        typing,
    },
};

pub type Node = node::markup::Node<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;

pub(super) fn init_node(
    node: typing::markup::Node,
    entities: &ConversionMap<typing::entity::Node, entity::Node>,
) -> Node {
    node.mapr(|r| r.map(|weak| Rc::downgrade(&entities[&weak.as_ptr()])))
}
