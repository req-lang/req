use crate::{
    node::{self, NodeRef},
    reference::Reference,
    semantics::{connection::entity, reference::expression::Target},
    utils::children::ChildrenIter,
};

pub type Variant =
    node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>;

#[derive(Debug, Clone)]
pub struct Node {
    pub variant: Variant,
}

impl From<Variant> for Node {
    fn from(variant: Variant) -> Self {
        Node { variant }
    }
}

impl ChildrenIter<NodeRef<Node>> for NodeRef<Node> {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.borrow()
            .variant
            .children_iter()
            .map(|n| n.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}
