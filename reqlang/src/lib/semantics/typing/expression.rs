use std::{cell::RefCell, fmt::Display, ops::Range, usize};

use crate::{
    issue::Severity,
    node::{
        self,
        expression::{Arena, Idx},
    },
    semantics::{
        link::{
            self, entity,
            expression::{Node, Target},
        },
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

pub(super) fn evaluate<E>(
    arena: Arena<link::expression::Node<E>>,
    types: Arena<Option<TypeResult>>,
) -> Arena<Node<TypeResult>> {
    arena
        .zip(types.map(|t| t.unwrap()))
        .map(|(node, typ)| node.map(|_| typ))
}

// TODO: Circular references between domains and requirements can loop indefinitely
pub(super) fn init_types<N, E, M>(
    entities: &node::entity::Arena<entity::Node<N, E, M>>,
    arena: &Arena<&Node<E>>,
    entity: node::entity::Idx,
    types: &node::entity::Arena<RefCell<Arena<Option<TypeResult>>>>,
    idx: Option<Range<usize>>,
) {
    let arena = match idx {
        Some(rng) => &Arena::from(Vec::from(&Vec::from(arena.clone())[rng])),
        None => arena,
    };

    for idx in 0..arena.as_vec().len() {
        ensure_typed(entities, arena, entity, types, idx.into());
    }
}

fn ensure_typed<N, E, M>(
    entities: &node::entity::Arena<entity::Node<N, E, M>>,
    arena: &Arena<&Node<E>>,
    entity: node::entity::Idx,
    types: &node::entity::Arena<RefCell<Arena<Option<TypeResult>>>>,
    idx: Idx,
) {
    use node::expression::Variant::*;

    if types[entity].borrow()[idx].is_some() {
        return;
    }

    match &arena[idx].variant {
        Identifier(ident) => match &ident.target.target() {
            Some(Target::Entity(tgt)) if types[*tgt].borrow().as_vec().is_empty() => {
                init_types(entities, arena, *tgt, types, None);
            }
            Some(Target::Expression(domain)) => {
                let is_untyped = types[entity].borrow()[*domain].is_none();
                if is_untyped {
                    let len = count(&arena, *domain);
                    let start = usize::from(*domain);
                    let end = start + len;
                    init_types(entities, arena, entity, types, Some(start..end));
                }
            }
            _ => {}
        },
        variant => {
            for child in variant.children_iter() {
                ensure_typed(entities, arena, entity, types, *child);
            }
        }
    }

    if types[entity].borrow()[idx].is_none() {
        let result = Some(evaluate_node(types, entity, arena, idx));
        types[entity].borrow_mut()[idx] = result;
    }
}

fn count<P>(arena: &Arena<&Node<P>>, idx: Idx) -> usize {
    let node = &arena[idx];
    1 + node
        .variant
        .children_iter()
        .map(|c| count(arena, *c))
        .sum::<usize>()
}
