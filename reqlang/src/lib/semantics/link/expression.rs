use std::collections::HashMap;

use crate::{
    node::{
        self,
        expression::{Arena, Idx},
    },
    semantics::{
        link::entity::{self},
        path::Path,
    },
};

#[derive(Debug, Clone, Copy)]
pub enum BuiltinKind {
    Real,
    Integer,
    Boolean,
}

impl BuiltinKind {
    fn find<S: AsRef<str>>(value: S) -> Option<Self> {
        use BuiltinKind::*;

        match value.as_ref() {
            "real" => Some(Real),
            "integer" => Some(Integer),
            "boolean" => Some(Boolean),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BuiltinKind::Real => "real",
            BuiltinKind::Integer => "integer",
            BuiltinKind::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Target<I = node::entity::Idx, J = Idx> {
    Entity(I),
    Expression(J),
    Builtin(BuiltinKind),
}

pub type Link<I = node::entity::Idx, J = Idx> = super::Link<Target<I, J>>;
pub type Node<P> = node::expression::Node<P, Link>;

pub(super) fn link<P, R: Into<Path>>(
    ctx: &entity::FindCtx,
    arena: Arena<node::expression::Node<P, R>>,
) -> Arena<Node<P>> {
    arena.from_walk(HashMap::new(), |node, mut current, _| {
        let node = node.mapr(|rf| find(ctx, &current, rf.into()));
        if let Some(set) = node.variant.set_element() {
            current.insert(set.variable.clone(), set.domain);
        }
        (current, node)
    })
}

fn find(ctx: &entity::FindCtx, local: &HashMap<String, Idx>, rf: Path) -> Link {
    use Target::*;

    if let Some(builtin) = BuiltinKind::find(rf.as_str()) {
        return super::Link::valid(rf, Builtin(builtin));
    }

    if let Some(expr) = local.get(rf.as_str()) {
        return super::Link::valid(rf, Expression(*expr));
    }

    let entity = entity::find(ctx, rf);
    let rf = entity.original;
    match entity.resolved {
        Ok(idx) => super::Link::valid(rf, Entity(idx)),
        Err(err) => super::Link::invalid(rf, err),
    }
}
