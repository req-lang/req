use crate::{
    node::entity::{Arena, Selector},
    semantics::{
        link::entity::Node,
        typing::{
            TypeResult,
            expression::{self},
        },
    },
};

pub(super) fn evaluate<N, E, M>(arena: Arena<Node<N, E, M>>) -> Arena<Node<N, TypeResult, M>> {
    let mut types = arena.as_ref().map_(|node| {
        node.variant
            .expressions()
            .next()
            .map(|expr| expr.empty::<Option<TypeResult>>())
    });

    // TODO: Refactor this mess
    arena.for_each(|node, idx| {
        if let Some(expr) = node.variant.expressions().next() {
            let ents = arena.as_ref();
            let entity = Selector::new(&ents, idx);
            expression::collect_types(entity, &expr.as_ref(), &mut types);
        }
    });

    arena.zip_map(types, |node, t| {
        let variant = node.variant.mape(|n| {
            n.zip(t.unwrap().map(|t, _| t.unwrap()))
                .map_(|(node, typ)| node.map(|_| typ))
        });
        Node::new(node.payload, node.meta, variant)
    })
}
