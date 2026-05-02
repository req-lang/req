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

pub(super) fn evaluate<N, E, M>(
    types: Arena<RefCell<Vec<Option<TypeResult>>>>,
    arena: Arena<Node<N, E, M>>,
) -> Arena<Node<N, TypeResult, M>> {
    arena.zip_map(types, |node, t| {
        let variant = node
            .variant
            .mape(|n| expression::evaluate(n, t.into_inner()));
        Node::new(node.payload, node.meta, variant)
    })
}

pub(super) fn init_types<N, E, M>(
    arena: &Arena<Node<N, E, M>>,
) -> Arena<RefCell<Vec<Option<TypeResult>>>> {
    let types = arena.as_ref().map(|node| {
        let init = match node.variant.expressions().next() {
            Some(expr) => std::iter::repeat(None).take(expr.len()).collect(),
            None => Vec::new(),
        };
        RefCell::new(init)
    });

    // TODO: Refactor this mess
    arena.idx_walk(|node, _, idx| {
        if let Some(expr) = node.variant.expressions().next() {
            expression::init_types(arena, &expr[..], idx.into(), &types, None);
        }
    });

    types
}
