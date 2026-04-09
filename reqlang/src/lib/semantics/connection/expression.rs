use crate::{
    node::{self, NodeRef},
    reference::Reference,
    semantics::{connection::entity, reference::expression::Target},
    utils::children::ChildrenIter,
};
use reqlang_derive::ChildrenIter;

pub type Variant =
    node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>;

#[derive(Debug, Clone, ChildrenIter)]
#[children_iter(NodeRef<Node>)]
pub struct Node {
    pub variant: Variant,
}

impl From<Variant> for Node {
    fn from(variant: Variant) -> Self {
        Node { variant }
    }
}

