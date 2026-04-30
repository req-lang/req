use std::{cell::RefCell, fmt::Display, ops::Range, usize};

use crate::{
    issue::Severity,
    node::{self, expression::Idx},
    semantics::{
        link::{self, expression::Target},
        typing::{TypeResult, evaluate::evaluate_node},
    },
    utils::children::ChildrenIter,
};

// TODO: Avoid boxed set, limit the number of nested sets
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

type Node = link::expression::Node<super::TypeResult>;

pub(super) fn evaluate<E>(
    arena: Vec<link::expression::Node<E>>,
    types: Vec<Option<TypeResult>>,
) -> Vec<Node> {
    arena
        .into_iter()
        .zip(types.into_iter().map(|t| t.unwrap()))
        .map(|(node, typ)| node.map(|_| typ))
        .collect()
}

// TODO: Circular references between domains and requirements can loop indefinitely
pub(super) fn init_types<N, E, M>(
    entities: &[link::entity::Node<N, E, M>],
    arena: &[link::expression::Node<E>],
    entity: node::entity::Idx,
    types: &Vec<RefCell<Vec<Option<TypeResult>>>>,
    idx: Option<Range<usize>>,
) {
    let arena = match idx {
        Some(rng) => &arena[rng],
        None => arena,
    };

    for idx in 0..arena.len() {
        ensure_typed(entities, arena, entity, types, idx.into());
    }
}

fn ensure_typed<N, E, M>(
    entities: &[link::entity::Node<N, E, M>],
    arena: &[link::expression::Node<E>],
    entity: node::entity::Idx,
    types: &Vec<RefCell<Vec<Option<TypeResult>>>>,
    idx: Idx,
) {
    use node::expression::Variant::*;

    if types[usize::from(entity)].borrow()[usize::from(idx)].is_some() {
        return;
    }

    match &arena[usize::from(idx)].variant {
        Identifier(ident) => match &ident.target.target() {
            Some(Target::Entity(tgt)) if types[usize::from(*tgt)].borrow().is_empty() => {
                init_types(entities, arena, *tgt, types, None);
            }
            Some(Target::Expression(domain)) => {
                let is_untyped =
                    types[usize::from(entity)].borrow()[usize::from(*domain)].is_none();
                if is_untyped {
                    let len = count(arena, *domain);
                    let start = usize::from(*domain);
                    let end = start + len;
                    init_types(entities, arena, entity, types, Some(start..end));
                }
            }
            _ => {}
        },
        variant => {
            for child in variant.children_iter() {
                ensure_typed(entities, arena, entity, types, child);
            }
        }
    }

    if types[usize::from(entity)].borrow()[usize::from(idx)].is_none() {
        let result = Some(evaluate_node(types, entity, arena, idx));
        types[usize::from(entity)].borrow_mut()[usize::from(idx)] = result;
    }
}

fn count<P>(arena: &[link::expression::Node<P>], idx: Idx) -> usize {
    let node = &arena[usize::from(idx)];
    1 + node
        .variant
        .children_iter()
        .map(|c| count(arena, c))
        .sum::<usize>()
}
