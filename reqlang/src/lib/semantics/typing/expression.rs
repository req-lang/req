use std::fmt::Display;

use crate::{
    issue::Severity,
    node::{
        self,
        expression::{Arena, Idx, Selector},
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
    entity: node::entity::Selector<&entity::Node<N, E, M>>,
    arena: &Arena<&Node<E>>,
    types: &mut node::entity::Arena<Option<Arena<Option<TypeResult>>>>,
) {
    arena.for_each(|_, idx| {
        collect_in_order(entity, Selector::new(arena, idx), types);
    });
}

fn collect_in_order<N, E, M>(
    entity: node::entity::Selector<&entity::Node<N, E, M>>,
    expression: Selector<&Node<E>>,
    types: &mut node::entity::Arena<Option<Arena<Option<TypeResult>>>>,
) {
    use node::expression::Variant::*;

    match &types[entity.idx()] {
        None => return,
        Some(typs) if typs[expression.idx()].is_some() => return,
        Some(_) => {}
    };

    match &expression.get().variant {
        Identifier(ident) => match &ident.target.target() {
            Some(Target::Entity(tgt))
                if let Some(typs) = &types[*tgt]
                    && typs[Idx::root()].is_none() =>
            {
                collect_types(entity.set(*tgt), expression.arena(), types);
            }
            Some(Target::Expression(domain))
                if types[entity.idx()].as_ref().unwrap()[*domain].is_none() =>
            {
                collect_in_order(entity, expression.set(*domain), types);
            }
            _ => {}
        },
        variant => {
            for child in variant.children_iter() {
                collect_in_order(entity, expression.set(*child), types);
            }
        }
    }

    let sel = node::entity::Selector::new(types, entity.idx());
    let result = Some(evaluate_node(sel, expression));
    types[entity.idx()].as_mut().unwrap()[expression.idx()] = result;
}
