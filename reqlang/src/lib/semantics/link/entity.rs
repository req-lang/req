use std::collections::HashMap;

use crate::node::entity::Arena;
use crate::semantics::link::{Annotation, FindCtx};
use crate::semantics::path::{Located, Path};
use crate::{
    node::{self, entity::Idx},
    semantics::link::{Link, expression, markup},
};

pub type Node<N, E, M> = node::entity::Node<N, Link, expression::Link, E, M>;

pub(super) fn link<N: Located, R: Into<Path> + Clone, T: Into<Path>, E, M>(
    paths: &HashMap<Path, Idx>,
    annotations: &Arena<Annotation>,
    arena: Arena<node::entity::Node<N, R, T, E, M>>,
) -> Arena<Node<N, E, M>> {
    arena.from_walk(Vec::new(), |node, imports, _| {
        let parent = node.payload.location().path().parent().unwrap_or_default();
        let ctx = FindCtx::new(&paths, &annotations, &parent, imports);
        let imports = match node.variant.imports() {
            Some(imports) => imports.iter().map(|i| i.clone().into()).collect(),
            None => Vec::new(),
        };
        let meta = node.meta.mapm(|r| markup::link(&ctx, r));
        let variant = node
            .variant
            .mapr(|r| {
                let r = r.into();
                match find(&ctx, &r) {
                    Some(idx) => Link::valid(r, idx),
                    None => Link::invalid(r),
                }
            })
            .mape(|e| expression::link(&ctx, e))
            .mapm(|m| markup::link(&ctx, m));

        (imports, Node::new(node.payload, meta, variant))
    })
}

pub(super) fn find(ctx: &FindCtx, rf: &Path) -> Option<Idx> {
    if let Some(direct) = ctx.nodes.get(rf) {
        return Some(*direct);
    }

    let child = ctx.current.join(rf);
    if let Some(child) = ctx.nodes.get(&child) {
        return Some(*child);
    }

    for import in &ctx.imports {
        let imported = import.join(rf);
        if let Some(imported) = ctx.nodes.get(&imported) {
            return Some(*imported);
        }
    }

    None
}
