use std::cell::RefCell;

use crate::{
    node::entity::Arena,
    semantics::{
        link::entity::Node,
        typing::{
            TypeResult,
            expression::{self},
        },
    },
};

pub(super) fn evaluate<N, E, M>(arena: Arena<Node<N, E, M>>) -> Arena<Node<N, TypeResult, M>> {
    let types = arena.as_ref().map_(|node| {
        let init = node
            .variant
            .expressions()
            .next()
            .map(|expr| expr.empty::<Option<TypeResult>>());
        init.map(RefCell::new)
    });

    // TODO: Refactor this mess
    arena.for_each(|node, idx| {
        if let Some(expr) = node.variant.expressions().next() {
            expression::collect_types(&arena, &expr.as_ref(), idx.into(), &types);
        }
    });

    arena.zip_map(types, |node, t| {
        let variant = node.variant.mape(|n| {
            n.zip(t.unwrap().into_inner().map(|t, _| t.unwrap()))
                .map_(|(node, typ)| node.map(|_| typ))
        });
        Node::new(node.payload, node.meta, variant)
    })
}
