use itertools::Itertools;

use crate::{
    node::{
        self,
        arena::ArenaEnumerate,
        entity::{Arena, Variant},
    },
    semantics::{
        link::entity::Node,
        typing::{
            TypeResult,
            expression::{self, TypeKind},
        },
    },
};

pub(super) fn evaluate<N, E, M>(arena: Arena<Node<N, E, M>>) -> Arena<Node<N, TypeResult, M>> {
    let mut types = (&arena)
        .into_iter()
        .indices()
        .map(|(idx, node)| match node.variant {
            Variant::Type(_) => {
                Some(vec![Some(Ok(TypeKind::Set(Box::new(TypeKind::Custom(idx)))))].into())
            }
            _ => node
                .variant
                .expressions()
                .next()
                .map(|expr| expr.with_capacity::<Option<TypeResult>>()),
        })
        .collect::<Arena<_>>();

    (&arena).into_iter().indices().for_each(|(idx, node)| {
        if let Some(expr) = node.variant.expressions().next() {
            expression::collect_types((&arena, idx).into(), &expr, &mut types);
        }
    });

    arena
        .into_iter()
        .zip_eq(types)
        .map(|(node, t)| {
            let variant = node.variant.mape(|n| {
                let types: Arena<_> = t.unwrap().into_iter().map(|t| t.unwrap()).collect();
                n.into_iter()
                    .zip_eq(types)
                    .map(|(node, typ)| node.map(|_| typ))
                    .collect::<node::expression::Arena<_>>()
            });
            Node::new(node.payload, node.meta, variant)
        })
        .collect()
}
