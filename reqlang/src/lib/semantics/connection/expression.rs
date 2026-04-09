use crate::{
    node::{NodeRef, expression::Variant},
    semantics::{
        connection::entity,
        link::{self},
    },
    utils::children::ChildrenIter,
};
use reqlang_derive::ChildrenIter;

#[derive(Debug, Clone, ChildrenIter)]
pub struct Node {
    pub variant: Variant<NodeRef<Node>, link::ResultTgt<entity::Node, Node>>,
}

impl From<Variant<NodeRef<Node>, link::ResultTgt<entity::Node, Node>>> for Node {
    fn from(variant: Variant<NodeRef<Node>, link::ResultTgt<entity::Node, Node>>) -> Self {
        Node { variant }
    }
}
