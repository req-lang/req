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
use reqlang_derive::ChildrenIter;

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    reference::Result<WeakNodeRef<Node>>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Debug, Clone, ChildrenIter)]
#[children_iter(NodeRef<Node>)]
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

pub(super) fn evaluate(node: NodeRef<Node>) {
    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    for expr in node.borrow().variant.expressions() {
        expression::evaluate(Rc::clone(expr));
    }
}
