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
use reqlang_derive::{ChildrenIter, EntityVariableMut, IntoEntityVariable};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    reference::Result<WeakNodeRef<Node>>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Debug, Clone, ChildrenIter, IntoEntityVariable, EntityVariableMut)]
#[children_iter(NodeRef<Node>)]
#[into_entity_variable(NodeRef<Node>, reference::Result<WeakNodeRef<Node>>, NodeRef<expression::Node>, markup::Node)]
#[entity_variable_mut(NodeRef<Node>, reference::Result<WeakNodeRef<Node>>, NodeRef<expression::Node>, markup::Node)]
pub struct Node {
    pub variant: Variant,
}

pub(super) fn evaluate(node: NodeRef<Node>) {
    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    for expr in node.borrow().variant.expressions() {
        expression::evaluate(Rc::clone(expr));
    }
}
