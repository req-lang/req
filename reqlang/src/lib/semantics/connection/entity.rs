use std::ops::{Index, IndexMut};

use crate::{
    node::{
        entity::{Arena, Idx, TraceabilityKind},
        tree::Tree,
    },
    semantics::{connection::graph, link::Links},
};

/// The entities an entity is connected to
#[derive(Default, Debug, Clone)]
pub struct Neighbors {
    pub inbound: Inbound,
    pub outbound: Outbound,
    pub parent: Option<Idx>,
}

#[derive(Default, Debug, Clone)]
pub struct Traceability {
    pub derivations: Vec<Idx>,
    pub refinements: Vec<Idx>,
    pub specializations: Vec<Idx>,
}

impl Traceability {
    pub fn iter(&self) -> impl Iterator<Item = (Idx, TraceabilityKind)> {
        use TraceabilityKind::*;

        let derivations = self.derivations.iter().map(|c| (*c, Derivation));
        let refinements = self.refinements.iter().map(|c| (*c, Refinement));
        let specializations = self.specializations.iter().map(|c| (*c, Specialization));

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
    type Output = Vec<Idx>;

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
    pub usages: Vec<Idx>,
    pub restrictions: Vec<Idx>,
    pub allocations: Vec<Idx>,
    pub traceability: Traceability,
}

// Explicitely defined in the syntax
#[derive(Default, Debug, Clone)]
pub struct Outbound {
    pub usages: Vec<Idx>,
    pub restriction: Option<Idx>,
    pub allocation: Option<Idx>,
    pub traceability: Traceability,
}

pub fn neighbors(tree: &Tree, links: &Links, neighbors: &mut Arena<Neighbors>) {
    graph::collect_neighbors(tree, links, neighbors);
}
