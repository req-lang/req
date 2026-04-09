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

#[derive(Default, Debug, Clone, ChildrenIter, IntoEntityVariable, EntityVariableMut)]
pub struct Node {
    pub variant: node::entity::Variant<NodeRef<Node>, reference::Result<WeakNodeRef<Node>>, NodeRef<expression::Node>, markup::Node>,
}

pub(super) fn evaluate(node: NodeRef<Node>) {
    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    for expr in node.borrow().variant.expressions() {
        expression::evaluate(Rc::clone(expr));
    }
}
