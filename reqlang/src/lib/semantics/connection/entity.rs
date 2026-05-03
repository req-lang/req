use std::{
    cell::RefCell,
    ops::{Index, IndexMut},
};

use crate::{
    node::{
        arena::ArenaEnumerate,
        entity::{Arena, Idx, TraceabilityKind},
    },
    semantics::{
        connection::{
            graph,
            issue::{self, IssueKind},
        },
        link::entity::Node,
    },
};

#[derive(Default, Debug, Clone)]
pub struct Neighbors<R> {
    pub inbound: Inbound<R>,
    pub outbound: Outbound<R>,
    pub parent: Option<R>,
}

impl<R> Neighbors<R> {
    pub fn mapr<S>(self, mut f: impl Fn(R) -> S) -> Neighbors<S> {
        Neighbors {
            parent: self.parent.map(&mut f),
            inbound: self.inbound.mapr(|x| f(x)),
            outbound: self.outbound.mapr(|x| f(x)),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Connection<R> {
    pub target: R,
    pub issue: Vec<IssueKind>,
}

impl<R> From<R> for Connection<R> {
    fn from(target: R) -> Self {
        Connection {
            target,
            issue: Vec::new(),
        }
    }
}

impl<R> Connection<R> {
    pub fn mapr<S>(self, f: impl Fn(R) -> S) -> Connection<S> {
        Connection {
            target: f(self.target),
            issue: self.issue,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Traceability<R> {
    pub derivations: Vec<R>,
    pub refinements: Vec<R>,
    pub specializations: Vec<R>,
    pub issues: Vec<IssueKind>,
}

impl<R> Traceability<R> {
    pub fn mapr<S>(self, mut f: impl Fn(R) -> S) -> Traceability<S> {
        Traceability {
            derivations: self.derivations.into_iter().map(&mut f).collect(),
            refinements: self.refinements.into_iter().map(&mut f).collect(),
            specializations: self.specializations.into_iter().map(&mut f).collect(),
            issues: self.issues,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&R, TraceabilityKind)> {
        use TraceabilityKind::*;

        let derivations = self.derivations.iter().map(|c| (c, Derivation));
        let refinements = self.refinements.iter().map(|c| (c, Refinement));
        let specializations = self.specializations.iter().map(|c| (c, Specialization));

        derivations.chain(refinements).chain(specializations)
    }
}

impl<R> IndexMut<TraceabilityKind> for Traceability<R> {
    fn index_mut(&mut self, index: TraceabilityKind) -> &mut Self::Output {
        use TraceabilityKind::*;

        match index {
            Specialization => &mut self.specializations,
            Refinement => &mut self.refinements,
            Derivation => &mut self.derivations,
        }
    }
}

impl<R> Index<TraceabilityKind> for Traceability<R> {
    type Output = Vec<R>;

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
pub struct Inbound<R> {
    // Subject to requirement
    pub usages: Vec<R>,
    pub restrictions: Vec<R>,
    pub allocations: Vec<R>,
    // Downward/Forward requirements
    pub traceability: Traceability<R>,
    pub issues: Vec<IssueKind>,
}

impl<R> Inbound<R> {
    pub fn mapr<S>(self, mut f: impl Fn(R) -> S) -> Inbound<S> {
        Inbound {
            usages: self.usages.into_iter().map(&mut f).collect(),
            restrictions: self.restrictions.into_iter().map(&mut f).collect(),
            allocations: self.allocations.into_iter().map(&mut f).collect(),
            traceability: self.traceability.mapr(f),
            issues: self.issues,
        }
    }
}

// Explicitely defined in the syntax
#[derive(Debug, Clone)]
pub struct Outbound<R> {
    // Requirement to subject
    pub usages: Vec<R>,
    pub restriction: Result<R, IssueKind>,
    pub allocation: Option<R>,
    // Upward/Backward requirements
    pub traceability: Traceability<R>,
}

impl<R> Outbound<R> {
    pub fn mapr<S>(self, mut f: impl Fn(R) -> S) -> Outbound<S> {
        Outbound {
            usages: self.usages.into_iter().map(&mut f).collect(),
            restriction: self.restriction.map(&mut f),
            allocation: self.allocation.map(&mut f),
            traceability: self.traceability.mapr(f),
        }
    }
}

impl<R: Default> Default for Outbound<R> {
    fn default() -> Self {
        Self {
            usages: Default::default(),
            restriction: Err(IssueKind::MissingRequirementBinding),
            allocation: Default::default(),
            traceability: Default::default(),
        }
    }
}

pub(super) fn connect<N, E, M>(
    arena: Arena<Node<N, E, M>>,
) -> Arena<Node<Neighbors<Connection<Idx>>, E, M>> {
    let mut neighbors = arena.with_capacity();
    graph::collect_neighbors(&arena, &mut neighbors);
    let cycles = detect_traceability_cycles(&neighbors);
    let neighbors: Arena<_> = neighbors
        .into_iter()
        .map(|n| n.mapr(Connection::from))
        .collect();
    let mut arena = arena
        .into_iter()
        .zip(neighbors)
        .map(|(node, neighbors)| node.map(|_| neighbors))
        .collect::<Arena<_>>();

    collect_cycles_isses(cycles, &mut arena);
    // Using Rc for performance reasons, otherwise neighbors might need to be cloned
    let arena = arena.into_iter().map(RefCell::new).collect::<Arena<_>>();
    (&arena)
        .into_iter()
        .indices()
        .for_each(|(idx, _)| issue::collect_issues(&arena, idx));
    arena.into_iter().map(|r| r.into_inner()).collect()
}

fn detect_traceability_cycles(arena: &Arena<Neighbors<Idx>>) -> Vec<Vec<Idx>> {
    arena.cycles(|node| {
        node.inbound
            .traceability
            .iter()
            .map(|(t, _)| *t)
            .collect::<Vec<_>>()
    })
}

fn collect_cycles_isses<E, M>(
    cycles: Vec<Vec<Idx>>,
    arena: &mut Arena<Node<Neighbors<Connection<Idx>>, E, M>>,
) {
    cycles.into_iter().flatten().for_each(|idx| {
        arena[idx]
            .payload
            .inbound
            .traceability
            .issues
            .push(IssueKind::CyclicTrace);
    });
}
