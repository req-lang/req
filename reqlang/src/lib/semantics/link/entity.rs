use std::collections::HashMap;

use crate::semantics::link::Issue;
use crate::semantics::path::{Located, Path};
use crate::{
    node::{self, entity::Idx},
    semantics::link::{Link, expression, markup},
    utils::children::ChildrenIter,
};

pub type Node<N, E, M> = node::entity::Node<N, Link, expression::Link, E, M>;

pub(super) fn link<N: Located, R: Into<Path>, T: Into<Path>, E, M>(
    paths: &HashMap<Path, Idx>,
    imports: Vec<Vec<Path>>,
    arena: Vec<node::entity::Node<N, R, T, E, M>>,
) -> Vec<Node<N, E, M>> {
    arena
        .into_iter()
        .zip(imports.into_iter())
        .map(|(node, imports)| {
            let parent = node.payload.location().path().parent().unwrap_or_default();
            let ctx = FindCtx::new(&paths, &parent, imports);
            let meta = node.meta.mapm(|r| markup::link(&ctx, r));
            let variant = node
                .variant
                .mapr(|r| find(&ctx, r.into()))
                .mape(|e| expression::link(&ctx, e))
                .mapm(|m| markup::link(&ctx, m));
            Node::new(node.payload, meta, variant)
        })
        .collect()
}

pub struct FindCtx<'a> {
    nodes: &'a HashMap<Path, Idx>,
    current: &'a Path,
    imports: Vec<Path>,
}

impl<'a> FindCtx<'a> {
    fn new(nodes: &'a HashMap<Path, Idx>, current: &'a Path, imports: Vec<Path>) -> Self {
        Self {
            nodes,
            current,
            imports,
        }
    }
}

pub(super) fn find(ctx: &FindCtx, rf: Path) -> Link {
    if let Some(direct) = ctx.nodes.get(&rf) {
        return Link::valid(rf, *direct);
    }

    let child = ctx.current.join(&rf);
    if let Some(child) = ctx.nodes.get(&child) {
        return Link::valid(rf, *child);
    }

    for import in &ctx.imports {
        let imported = import.join(&rf);
        if let Some(imported) = ctx.nodes.get(&imported) {
            return Link::valid(rf, *imported);
        }
    }

    Link::invalid(rf, Issue::NotFound)
}

pub(super) fn collect_imports<'n, P, R: Into<Path> + Clone, T, E, M>(
    arena: &'n Vec<node::entity::Node<P, R, T, E, M>>,
    idx: Idx,
    prev: Vec<Path>,
    parents: &mut Vec<Vec<Path>>,
) {
    let node = &arena[idx];
    parents.push(prev);

    let imports = match node.variant.imports() {
        Some(imports) => imports.iter().map(|i| i.clone().into()).collect(),
        None => Vec::new(),
    };
    for child in node.variant.children_iter() {
        collect_imports(arena, child, imports.clone(), parents);
    }
}
