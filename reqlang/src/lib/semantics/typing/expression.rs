use std::{cell::RefCell, fmt::Display};

use crate::{
    issue::Severity,
    node::{
        self,
        expression::{Arena, Idx},
    },
    semantics::{
        link::{
            entity,
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
    #[error("this expression points to an entity that has no type")]
    NotEvaluable,
}

impl From<&Issue> for Severity {
    fn from(_: &Issue) -> Self {
        use Severity::*;

        Critical
    }
}
//
// TODO: Circular references between domains and requirements can loop indefinitely
pub(super) fn collect_types<N, E, M>(
    entities: &node::entity::Arena<entity::Node<N, E, M>>,
    arena: &Arena<&Node<E>>,
    entity: node::entity::Idx,
    types: &node::entity::Arena<Option<RefCell<Arena<Option<TypeResult>>>>>,
) {
    for idx in 0..arena.as_vec().len() {
        collect_in_order(entities, arena, entity, types, idx.into());
    }
}

fn collect_in_order<N, E, M>(
    entities: &node::entity::Arena<entity::Node<N, E, M>>,
    arena: &Arena<&Node<E>>,
    entity: node::entity::Idx,
    types: &node::entity::Arena<Option<RefCell<Arena<Option<TypeResult>>>>>,
    idx: Idx,
) {
    use node::expression::Variant::*;

    let typs = match &types[entity] {
        None => return,
        Some(typs) if typs.borrow()[idx].is_some() => return,
        Some(typs) => typs,
    };

    match &arena[idx].variant {
        Identifier(ident) => match &ident.target.target() {
            Some(Target::Entity(tgt))
                if let Some(typs) = &types[*tgt]
                    && typs.borrow()[Idx::root()].is_none() =>
            {
                collect_types(entities, arena, *tgt, types);
            }
            Some(Target::Expression(domain)) if typs.borrow()[*domain].is_none() => {
                let descendants = arena.descendants(*domain);
                let arena = arena.subrange(descendants).map(|n, _| *n);
                collect_types(entities, &arena, entity, types);
            }
            _ => {}
        },
        variant => {
            for child in variant.children_iter() {
                collect_in_order(entities, arena, entity, types, *child);
            }
        }
    }

    if typs.borrow()[idx].is_none() {
        let result = Some(evaluate_node(types, entity, arena, idx));
        typs.borrow_mut()[idx] = result;
    }
}
