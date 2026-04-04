use std::rc::Rc;

use crate::{
    node::{self, NodeRef, entity::VariableMut},
    semantics::{
        reference::{self, WeakNodeRef},
        typing::{expression, markup},
    },
    utils::children::ChildrenIter,
};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    reference::Result<WeakNodeRef<Node>>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Debug)]
pub struct Node {
    pub variant: Variant,
}

impl
    VariableMut<
        NodeRef<Node>,
        reference::Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    > for Node
{
    fn variable_mut(
        &mut self,
    ) -> &mut node::entity::Variant<
        NodeRef<Node>,
        reference::Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    > {
        &mut self.variant
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

pub(super) fn evaluate(node: NodeRef<Node>) {
    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    for expr in node.borrow().variant.expressions() {
        expression::evaluate(Rc::clone(expr));
    }
}
