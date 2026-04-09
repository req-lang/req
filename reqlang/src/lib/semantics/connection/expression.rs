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
pub struct Node {
    pub variant: node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>,
}

impl From<Variant> for Node {
    fn from(variant: Variant) -> Self {
        Node { variant }
    }
}

