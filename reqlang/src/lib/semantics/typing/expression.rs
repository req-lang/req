use std::{fmt::Display, rc::Rc};

use crate::{
    issue::Severity,
    node::{self, NodeRef, expression::IntoVariable},
    reference::Reference,
    semantics::{
        link::expression::Target,
        typing::{entity, evaluate::evaluate_node},
    },
    utils::children::ChildrenIter,
};
use reqlang_derive::{ChildrenIter, IntoExpressionVariable};

#[derive(Eq, PartialEq, Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("previously caught error")]
    Poison,
    #[error("expected a set got {0}")]
    NotASet(TypeKind),
    #[error("expected {0} got {1}")]
    UnexpectedType(TypeKind, TypeKind),
    #[error("cannot retrieve the domain for this identifier")]
    DomainNotFound,
}

impl From<&Issue> for Severity {
    fn from(_: &Issue) -> Self {
        use Severity::*;

        Critical
    }
}

pub type Variant =
    node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>;

#[derive(Debug, Clone, ChildrenIter, IntoExpressionVariable)]
pub struct Node {
    pub typ: Result<TypeKind, Issue>,
    pub variant:
        node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>,
}

impl From<Variant> for Node {
    fn from(variant: Variant) -> Self {
        Node {
            typ: Ok(TypeKind::Undefined),
            variant,
        }
    }
}

pub(super) fn evaluate(node: NodeRef<Node>) {
    use node::entity::Variant::*;
    use node::expression::Variant::*;

    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    if let Identifier(ident) = &node.borrow().variant {
        if let Ok(Target::Entity(n)) = &ident.target {
            if let Some(n) = n.upgrade() {
                if let Attribute(atr) = &n.borrow().variant
                    && atr.domain.borrow().typ == Ok(TypeKind::Undefined)
                {
                    evaluate(Rc::clone(&atr.domain));
                }
            }
        }

        if let Ok(Target::Expression(n)) = &ident.target {
            if let Some(n) = n.upgrade()
                && n.borrow().typ == Ok(TypeKind::Undefined)
            {
                evaluate(n);
            }
        }
    }

    node.borrow_mut().typ = evaluate_node(node.borrow());
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TypeKind {
    Number,
    Boolean,
    Undefined,
    Set(Box<TypeKind>),
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TypeKind {
    pub fn is_set(&self) -> bool {
        match self {
            TypeKind::Set(_) => true,
            _ => false,
        }
    }
}
