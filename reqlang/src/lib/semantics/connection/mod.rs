use std::{cell::RefCell, fmt::Debug};

use crate::{
    node::entity::{Arena, Idx},
    semantics::{
        connection::{
            entity::{Connection, Neighbors},
            graph::Graph,
        },
        link::{self, entity::Node},
    },
};

pub mod cycles;
pub mod entity;
pub mod graph;
pub mod issue;

pub trait Connect<E, M> {
    fn connect(self) -> Arena<Node<Neighbors<Connection<Idx>>, E, M>>;
}

impl<N, E: Debug, M: Debug> Connect<E, M> for Arena<link::entity::Node<N, E, M>> {
    fn connect(self) -> Arena<Node<Neighbors<Connection<Idx>>, E, M>> {
        let mut graph = Graph::default();

        graph::build_parents(&self, &mut graph);
        graph::build_transverse(&self, &mut graph);
        let arena = entity::connect(self, graph);

        let arena = arena.map_(RefCell::new);
        cycles::detect_traceability_cycles(&arena);
        arena.for_each(|_, idx| issue::collect_issues(&arena, idx));
        arena.map_(|n| n.into_inner())
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
