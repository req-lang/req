use std::fmt::Debug;

use crate::{
    node::entity::{Arena, Idx},
    semantics::{
        connection::entity::{Connection, Neighbors},
        link::{self, entity::Node},
    },
};

pub mod entity;
pub mod graph;
pub mod issue;

pub trait Connect<E, M> {
    fn connect(self) -> Arena<Node<Neighbors<Connection<Idx>>, E, M>>;
}

impl<N, E: Debug, M: Debug> Connect<E, M> for Arena<link::entity::Node<N, E, M>> {
    fn connect(self) -> Arena<Node<Neighbors<Connection<Idx>>, E, M>> {
        entity::connect(self)
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
