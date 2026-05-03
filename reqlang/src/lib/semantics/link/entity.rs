use std::collections::HashMap;

use crate::node::entity::Arena;
use crate::semantics::link::Issue;
use crate::semantics::path::{Located, Path};
use crate::{
    node::{self, entity::Idx},
    semantics::link::{Link, expression, markup},
};

pub type Node<N, E, M> = node::entity::Node<N, Link, expression::Link, E, M>;

pub(super) fn link<N: Located, R: Into<Path> + Clone, T: Into<Path>, E, M>(
    paths: &HashMap<Path, Idx>,
    arena: Arena<node::entity::Node<N, R, T, E, M>>,
) -> Arena<Node<N, E, M>> {
    arena.from_walk(Vec::new(), |node, imports, _| {
        let parent = node.payload.location().path().parent().unwrap_or_default();
        let ctx = FindCtx::new(&paths, &parent, imports);
        let imports = match node.variant.imports() {
            Some(imports) => imports.iter().map(|i| i.clone().into()).collect(),
            None => Vec::new(),
        };
        let meta = node.meta.mapm(|r| markup::link(&ctx, r));
        let variant = node
            .variant
            .mapr(|r| find(&ctx, r.into()))
            .mape(|e| expression::link(&ctx, e))
            .mapm(|m| markup::link(&ctx, m));

        (imports, Node::new(node.payload, meta, variant))
    })
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
