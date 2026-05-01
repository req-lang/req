use std::collections::HashMap;

use crate::{
    node::{self, expression::Idx},
    semantics::link::entity::{self},
    semantics::path::Path,
    utils::children::ChildrenIter,
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
    arena: Vec<node::expression::Node<P, R>>,
) -> Vec<Node<P>> {
    let mut local: Vec<HashMap<String, Idx>> = Vec::with_capacity(arena.len());

    for _ in &arena {
        local.push(HashMap::new());
    }

    init_scope(&arena, 0, &mut local);
    arena
        .into_iter()
        .zip(local.into_iter())
        .map(|(node, scope)| node.mapr(|rf| find(ctx, &scope, rf.into())))
        .collect()
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

fn init_scope<P, R>(
    arena: &Vec<node::expression::Node<P, R>>,
    idx: usize,
    local: &mut Vec<HashMap<String, Idx>>,
) {
    let scope = &mut local[idx];
    let node = &arena[idx];
    if let Some(set) = node.variant.set_element() {
        scope.insert(set.variable.clone(), set.domain);
    }

    let ext = scope.clone();
    for child in node.variant.children_iter() {
        let idx = usize::from(child);
        local[idx].extend(ext.clone());
        init_scope(arena, idx, local);
    }
}
