use std::{
    collections::HashSet,
    ops::{Index, IndexMut},
};

use crate::{
    node::{
        entity::{Arena, Idx, TraceabilityKind},
        tree::Tree,
    },
    semantics::{
        connection::{
            graph,
            issue::{self, IssueKind},
        },
        link::Links,
    },
};

#[derive(Default, Debug, Clone)]
pub struct Neighbors {
    pub inbound: Inbound,
    pub outbound: Outbound,
    pub parent: Option<Connection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub target: Idx,
    pub issues: Vec<IssueKind>,
}

impl Connection {
    pub fn new(target: Idx) -> Self {
        Connection {
            target,
            issues: Vec::new(),
        }
    }
}

impl From<Idx> for Connection {
    fn from(target: Idx) -> Self {
        Connection::new(target)
    }
}

#[derive(Default, Debug, Clone)]
pub struct Traceability {
    pub derivations: Vec<Connection>,
    pub refinements: Vec<Connection>,
    pub specializations: Vec<Connection>,
    pub issues: Vec<IssueKind>,
}

impl Traceability {
    pub fn iter(&self) -> impl Iterator<Item = (&Connection, TraceabilityKind)> {
        use TraceabilityKind::*;

        let derivations = self.derivations.iter().map(|c| (c, Derivation));
        let refinements = self.refinements.iter().map(|c| (c, Refinement));
        let specializations = self.specializations.iter().map(|c| (c, Specialization));

        derivations.chain(refinements).chain(specializations)
    }
}

impl IndexMut<TraceabilityKind> for Traceability {
    fn index_mut(&mut self, index: TraceabilityKind) -> &mut Self::Output {
        use TraceabilityKind::*;

        match index {
            Specialization => &mut self.specializations,
            Refinement => &mut self.refinements,
            Derivation => &mut self.derivations,
        }
    }
}

impl Index<TraceabilityKind> for Traceability {
    type Output = Vec<Connection>;

    fn index(&self, index: TraceabilityKind) -> &Self::Output {
        use TraceabilityKind::*;

        match index {
            Specialization => &self.specializations,
            Refinement => &self.refinements,
            Derivation => &self.derivations,
        }
    }
}

// Derived from outbound connections
#[derive(Default, Debug, Clone)]
pub struct Inbound {
    pub usages: Vec<Connection>,
    pub restrictions: Vec<Connection>,
    pub allocations: Vec<Connection>,
    pub traceability: Traceability,
    pub issues: Vec<IssueKind>,
}

// Explicitely defined in the syntax
#[derive(Default, Debug, Clone)]
pub struct Outbound {
    pub usages: Vec<Connection>,
    pub restriction: Option<Connection>,
    pub allocation: Option<Connection>,
    pub traceability: Traceability,
    pub issues: Vec<IssueKind>,
}

pub fn neighbors(tree: &Tree, links: &Links, neighbors: &mut Arena<Neighbors>) {
    graph::collect_neighbors(tree, links, neighbors);
}

pub fn diagnostics(tree: &Tree, neighbors: &mut Arena<Neighbors>) {
    let cycles: HashSet<Idx> = detect_traceability_cycles(neighbors)
        .into_iter()
        .flatten()
        .collect();

    for idx in neighbors.indices() {
        issue::collect(&tree.entities, neighbors, &cycles, idx);
    }
}

fn detect_traceability_cycles(arena: &Arena<Neighbors>) -> Vec<Vec<Idx>> {
    arena.cycles(|node| {
        node.inbound
            .traceability
            .iter()
            .map(|(t, _)| t.target)
            .collect::<Vec<_>>()
    })
}
