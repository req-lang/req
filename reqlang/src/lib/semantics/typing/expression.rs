use std::fmt::Display;

use crate::{
    issue::{
        Severity,
        code::{IssueCode, codes},
    },
    node::{
        self,
        expression::{Arena, Idx, Variant},
        tree::Tree,
    },
    semantics::{
        link::{Links, Target},
        typing::{Result, evaluate::evaluate_node},
    },
    utils::children::ChildrenIter,
};

// TODO: Avoid boxed set, limit the number of nested sets
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TypeKind {
    Number,
    Boolean,
    Undefined,
    Custom(node::entity::Idx),
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

#[derive(Eq, PartialEq, Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("previously caught error")]
    Poison,
    #[error("expected a set type got {0}")]
    NotASet(TypeKind),
    #[error("expected type {0} got {1}")]
    UnexpectedType(TypeKind, TypeKind),
    #[error("this expression points to an entity that has no type")]
    NotEvaluable,
}

impl Issue {
    pub fn code(&self) -> IssueCode {
        match self {
            Issue::Poison => codes::PROPAGATED_TYPE_ERROR,
            Issue::NotASet(_) => codes::NOT_A_SET,
            Issue::UnexpectedType(..) => codes::UNEXPECTED_TYPE,
            Issue::NotEvaluable => codes::NOT_EVALUABLE,
        }
    }
}

impl From<&Issue> for Severity {
    fn from(_: &Issue) -> Self {
        use Severity::*;

        Critical
    }
}
//
// TODO: Circular references between domains and requirements can loop indefinitely
pub(super) fn collect_types(
    tree: &Tree,
    links: &Links,
    idx: Idx,
    types: &mut Arena<Option<Result>>,
) {
    if types[idx].is_some() {
        return;
    }

    if let Variant::Identifier(ident) = &tree.expressions[idx].variant
        && let Some(target) = &links.identifiers[ident.target]
    {
        match target {
            Target::Entity(tgt) => {
                if let Some(root) = tree.entities[*tgt].variant.expressions().next() {
                    collect_types(tree, links, root, types);
                }
            }
            Target::Local(set) => {
                let domain = tree.expressions[*set].variant.set_element().unwrap().domain;
                collect_types(tree, links, domain, types);
            }
            _ => {}
        }
    }

    for child in tree.expressions[idx].children_iter() {
        collect_types(tree, links, child, types);
    }

    if types[idx].is_none() {
        let result = evaluate_node(tree, links, types, idx);
        types[idx] = Some(result);
    }
}
