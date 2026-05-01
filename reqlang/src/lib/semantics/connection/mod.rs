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

pub trait Connect<E, M> {
    fn connect(self) -> Vec<entity::Node<E, M>>;
}

impl<N, E: Debug, M: Debug> Connect<E, M> for Vec<link::entity::Node<N, E, M>> {
    fn connect(self) -> Vec<entity::Node<E, M>> {
        let mut graph = Graph::default();

        graph::build_parents(&self, 0.into(), None, &mut graph);
        graph::build_transverse(&self, &mut graph);
        let arena = entity::connect(self, graph);

        let arena: Vec<_> = arena.into_iter().map(RefCell::new).collect();
        cycles::detect_traceability_cycles(&arena);
        for (idx, _) in arena.iter().enumerate() {
            issue::collect_issues(&arena, idx.into());
        }

        arena.into_iter().map(|n| n.into_inner()).collect()
    }
}

pub trait Connected {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>>;
}

impl Connected for Neighbors<Connection<Idx>> {
    fn neigbhors(&self) -> &Neighbors<Connection<Idx>> {
        &self
    }
}
