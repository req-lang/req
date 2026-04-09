use std::rc::Rc;

use crate::{
    node::{
        self, NodeRef,
        entity::{IntoVariable, VariableMut},
    },
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

#[derive(Default, Debug, Clone)]
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
    fn variant_mut(
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

impl
    IntoVariable<
        NodeRef<Node>,
        reference::Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    > for Node
{
    fn into_variant(
        self,
    ) -> node::entity::Variant<
        NodeRef<Node>,
        reference::Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    > {
        self.variant
    }
}

impl ChildrenIter<NodeRef<Node>> for Node {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.variant
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
