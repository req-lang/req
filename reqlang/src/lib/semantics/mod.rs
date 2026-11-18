use crate::{
    node::{
        entity::{self},
        expression,
        tree::Tree,
    },
    semantics::{
        connection::{entity::Neighbors, issue::Issues},
        link::Links,
    },
};

pub mod connection;
pub mod link;
pub mod path;
pub mod typing;

#[derive(Debug, Clone)]
pub struct Analysis {
    pub paths: entity::Arena<path::Location>,
    pub links: Links,
    pub types: expression::Arena<typing::Result>,
    pub neighbors: entity::Arena<Neighbors>,
    /// The issues of the connections, see [Analysis::diagnose]
    pub connection: Issues,
}

impl From<&Tree> for Analysis {
    fn from(tree: &Tree) -> Self {
        let paths = path::paths(&tree.entities);
        let links = link::link(&tree, &paths);
        let types = typing::evaluate(&tree, &links);
        let neighbors = connection::neighbors(&tree, &links);

        Self {
            paths,
            links,
            types,
            neighbors,
            connection: Issues::new(tree),
        }
    }
}

impl Analysis {
    pub(super) fn diagnose(&mut self, tree: &Tree) {
        path::duplicates(&mut self.paths);
        self.connection = Issues::diagnose(tree, &self.links, &self.neighbors);
    }
}
