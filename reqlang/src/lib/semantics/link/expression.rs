use std::collections::HashMap;

use crate::{
    node::{self, expression::Idx},
    reference::Reference,
    semantics::link::{
        Result,
        entity::{self},
    },
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
}

#[derive(Debug, Clone)]
pub enum Target {
    Entity(node::entity::Idx),
    Expression(String, Idx),
    Builtin(BuiltinKind),
}

pub type Node<P> = node::expression::Node<P, Result<Target>>;

pub(super) fn link<P, R: Into<Reference> + AsRef<str> + Clone>(
    ctx: &entity::FindCtx<R>,
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
        .map(|(node, scope)| node.mapr(|rf| find(ctx, &scope, rf)))
        .collect()
}

fn find<R: Into<Reference> + AsRef<str> + Clone>(
    ctx: &entity::FindCtx<R>,
    local: &HashMap<String, Idx>,
    target: R,
) -> Result<Target> {
    let rf = target.as_ref();
    if let Some(builtin) = BuiltinKind::find(rf) {
        return Ok(Target::Builtin(builtin));
    }

    if let Some(expr) = local.get(rf) {
        return Ok(Target::Expression(rf.to_string(), *expr));
    }

    let entity = entity::find(ctx, target)?;
    Ok(Target::Entity(entity))
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
