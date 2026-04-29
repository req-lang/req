use std::collections::HashMap;

use crate::{
    node::{self, entity::Idx},
    reference::Reference,
    semantics::link::{Result, expression, markup},
    utils::children::ChildrenIter,
};

pub type Node<P, E, M> = node::entity::Node<P, Result<Idx>, Result<expression::Target>, E, M>;

pub(super) fn link<N, R, E, M>(
    paths: &HashMap<Reference, Idx>,
    iv: Vec<Vec<&Reference>>,
    parents: Vec<Vec<R>>,
    arena: Vec<node::entity::Node<N, R, R, E, M>>,
) -> Vec<Node<N, E, M>>
where
    R: Into<Reference> + AsRef<str> + Clone,
{
    arena
        .into_iter()
        .zip(iv.into_iter())
        .zip(parents.into_iter())
        .map(|((node, current), imports)| {
            let parent = Reference::new(current[0].parent());
            let ctx = FindCtx::new(&paths, &parent, imports);
            let meta = node.meta.mapm(|r| markup::link(&ctx, r));
            let variant = node
                .variant
                .mapr(|r| find(&ctx, r))
                .mape(|e| expression::link(&ctx, e))
                .mapm(|m| markup::link(&ctx, m));
            Node::new(node.payload, meta, variant)
        })
        .collect()
}

pub struct FindCtx<'a, R> {
    nodes: &'a HashMap<Reference, Idx>,
    current: &'a Reference,
    imports: Vec<R>,
}

impl<'a, R> FindCtx<'a, R> {
    fn new(nodes: &'a HashMap<Reference, Idx>, current: &'a Reference, imports: Vec<R>) -> Self {
        Self {
            nodes,
            current,
            imports,
        }
    }
}

pub(super) fn find<R: Into<Reference> + Clone>(ctx: &FindCtx<'_, R>, rf: R) -> Result<Idx> {
    let rf = rf.into();
    if let Some(direct) = ctx.nodes.get(&rf) {
        return Ok(*direct);
    }

    let child = ctx.current + &rf;
    if let Some(child) = ctx.nodes.get(&child) {
        return Ok(*child);
    }

    for import in &ctx.imports {
        let imported = &(*import).clone().into() + rf.as_str();
        if let Some(imported) = ctx.nodes.get(&imported) {
            return Ok(imported.clone());
        }
    }

    Err(rf)
}

pub(super) fn init_imports<'n, P, R: Clone, T, E, M>(
    arena: &'n Vec<node::entity::Node<P, R, T, E, M>>,
    idx: Idx,
    prev: Vec<R>,
    parents: &mut Vec<Vec<R>>,
) {
    let node = &arena[idx];
    parents.push(prev);

    let imports = match node.variant.imports() {
        Some(imports) => imports.iter().cloned().collect(),
        None => Vec::new(),
    };
    for child in node.variant.children_iter() {
        init_imports(arena, child, imports.clone(), parents);
    }
}

pub(super) fn init_paths<'n, P, R, T, E, M>(
    arena: &Vec<node::entity::Node<P, R, T, E, M>>,
    idx: Idx,
    parent: &Reference,
    paths: &mut HashMap<Reference, Idx>,
) {
    let node = &arena[idx];
    let current = parent + &node.meta.label;

    for child in node.variant.children_iter() {
        init_paths(arena, child, &current, paths);
    }

    paths.insert(current, idx);
}

pub(super) fn iv_paths<'p, N>(
    arena: &Vec<N>,
    paths: &'p HashMap<Reference, Idx>,
) -> Vec<Vec<&'p Reference>> {
    let mut iv = Vec::with_capacity(paths.len());

    for _ in arena {
        iv.push(Vec::new());
    }

    for (path, idx) in paths {
        iv[usize::from(*idx)].push(path);
    }

    iv
}
