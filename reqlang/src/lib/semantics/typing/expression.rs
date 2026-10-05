use std::fmt::Display;

use crate::{
    issue::Severity,
    node::{
        self,
        arena::ArenaEnumerate,
        expression::{Arena, Idx, Selector},
    },
    semantics::{
        link::{entity, expression::Node},
        typing::{TypeResult, evaluate::evaluate_node},
    },
    utils::children::ChildrenIter,
};

// TODO: Avoid boxed set, limit the number of nested sets
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum TypeKind<I = node::entity::Idx> {
    Number,
    Boolean,
    Undefined,
    Custom(I),
    Set(Box<TypeKind<I>>),
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
    entity: node::entity::Selector<entity::Node<N, E, M>>,
    arena: &Arena<Node<E>>,
    types: &mut node::entity::Arena<Option<Arena<Option<TypeResult>>>>,
) {
    for (idx, _) in arena.into_iter().indices() {
        collect_in_order(entity, Selector::new(arena, idx), types);
    }
}

fn collect_in_order<N, E, M>(
    entity: node::entity::Selector<entity::Node<N, E, M>>,
    expression: Selector<Node<E>>,
    types: &mut node::entity::Arena<Option<Arena<Option<TypeResult>>>>,
) {
    use crate::semantics::link::Target::*;
    use crate::semantics::link::expression::Target::*;
    use node::expression::Variant::*;

    let (entities, i) = entity.into();
    let (expressions, j) = expression.into();

    match &types[i] {
        None => return,
        Some(typs) if typs[j].is_some() => return,
        Some(_) => {}
    };

    if let Identifier(ident) = &expressions[j].variant
        && let Some(target) = &ident.target.target()
    {
        match target {
            Outer(Entity(tgt)) => {
                if let Some(typs) = &types[*tgt]
                    && typs[Idx::root()].is_none()
                    && let Some(expr) = entities[*tgt].variant.expressions().next()
                {
                    collect_types((entities, *tgt).into(), expr, types);
                }
            }
            Inner(idx) => {
                let domain = expressions[*idx].variant.set_element().unwrap().domain;
                if types[i].as_ref().unwrap()[domain].is_none() {
                    collect_in_order((entities, i).into(), (expressions, domain).into(), types);
                }
            }
            _ => {}
        }
    }
    for child in expressions[j].children_iter() {
        collect_in_order((entities, i).into(), (expressions, child).into(), types);
    }
    if types[i].as_ref().unwrap()[j].is_none() {
        let result = Some(evaluate_node((&*types, i).into(), (expressions, j).into()));
        types[i].as_mut().unwrap()[j] = result;
    }
}
