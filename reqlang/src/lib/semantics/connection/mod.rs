use std::{cell::RefCell, fmt::Debug};

use crate::{
    node::entity::Idx,
    semantics::{
        connection::{
            entity::{Connection, Neighbors},
            graph::Graph,
        },
        link::{self},
    },
};

pub mod cycles;
pub mod entity;
pub mod graph;
pub mod issue;

pub trait Connected {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>>;
}

impl Connected for Neighbors<Connection<Idx>> {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>> {
        &self
    }
}

pub fn connect<N, E: Debug, M: Debug>(
    arena: Vec<link::entity::Node<N, E, M>>,
) -> Vec<entity::Node<E, M>> {
    let mut graph = Graph::default();

    graph::build_graph(&arena, &mut graph, None);
    let arena = entity::connect(arena, &mut graph);

    let arena: Vec<_> = arena.into_iter().map(RefCell::new).collect();
    cycles::detect_traceability_cycles(&arena);
    for (idx, _) in arena.iter().enumerate() {
        issue::collect_issues(&arena, idx.into());
    }

    arena.into_iter().map(|n| n.into_inner()).collect()
}
