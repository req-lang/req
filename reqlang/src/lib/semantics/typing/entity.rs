use std::cell::RefCell;

use crate::semantics::{
    link::{self},
    typing::{
        TypeResult,
        expression::{self},
    },
};

pub type Node<N, M> = link::entity::Node<N, TypeResult, M>;

pub(super) fn evaluate<N, E, M>(
    types: Vec<RefCell<Vec<Option<TypeResult>>>>,
    arena: Vec<link::entity::Node<N, E, M>>,
) -> Vec<Node<N, M>> {
    arena
        .into_iter()
        .zip(types)
        .map(|(node, t)| {
            let variant = node
                .variant
                .mape(|n| expression::evaluate(n, t.into_inner()));
            Node::new(node.payload, node.meta, variant)
        })
        .collect()
}

pub(super) fn init_types<N, E, M>(
    arena: &Vec<link::entity::Node<N, E, M>>,
) -> Vec<RefCell<Vec<Option<TypeResult>>>> {
    let mut types = Vec::with_capacity(arena.len());
    for expr in arena.iter().map(|n| n.variant.expressions().next()) {
        let init = match expr {
            Some(expr) => std::iter::repeat(None).take(expr.len()).collect(),
            None => Vec::new(),
        };
        types.push(RefCell::new(init));
    }

    for (idx, node) in arena.iter().enumerate() {
        if let Some(expr) = node.variant.expressions().next() {
            expression::init_types(&arena, &expr[..], idx.into(), &types, None);
        }
    }

    types
}
