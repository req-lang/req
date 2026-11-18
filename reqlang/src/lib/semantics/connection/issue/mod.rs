//! The issues of the connections, gathered by what they are about. The diagnostics of each
//! module give an arena indexed by that: the entities or the references.

pub mod entity;
pub mod reference;

use crate::{
    node::{self, tree::Tree},
    semantics::{connection::entity::Neighbors, link::Links},
};

/// The issues of the connections
#[derive(Debug, Clone)]
pub struct Issues {
    /// Of each entity on its own
    pub entity: node::entity::Arena<Vec<entity::Issue>>,
    /// Of each reference, the connection it creates
    pub reference: node::reference::Arena<Vec<reference::Issue>>,
}

impl Issues {
    /// No issue yet
    pub fn new(tree: &Tree) -> Self {
        Self {
            entity: tree.entities.parallel(),
            reference: tree.references.parallel(),
        }
    }

    pub fn diagnose(
        tree: &Tree,
        links: &Links,
        neighbors: &node::entity::Arena<Neighbors>,
    ) -> Self {
        Self {
            entity: entity::diagnostics(tree, neighbors),
            reference: reference::diagnostics(tree, links, neighbors),
        }
    }
}
