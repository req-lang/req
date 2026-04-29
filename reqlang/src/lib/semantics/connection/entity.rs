use crate::{
    node::entity::{Idx, TraceabilityKind},
    semantics::{
        connection::{graph::Graph, issue::IssueKind},
        link::{self},
    },
    utils::take::Take,
};

pub(super) type Node<E, M> = link::entity::Node<Neighbors<Connection<Idx>>, E, M>;

#[derive(Debug, Clone)]
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

fn from_vec(x: Vec<Idx>) -> Vec<Connection<Idx>> {
    x.into_iter().map(|ptr| Connection::from(ptr)).collect()
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

pub(super) fn connect<N, E, M>(
    arena: Vec<link::entity::Node<N, E, M>>,
    graph: &mut Graph,
) -> Vec<Node<E, M>> {
    arena
        .into_iter()
        .enumerate()
        .map(|(idx, n)| n.map(|_| make_neighbors(idx.into(), graph)))
        .collect()
}

fn make_neighbors(nd: Idx, graph: &mut Graph) -> Neighbors<Connection<Idx>> {
    Neighbors {
        parent: graph.parent.remove(&nd).map(Connection::from),
        inbound: Inbound {
            restrictions: from_vec(graph.inbound.restrictions.take(&nd)),
            allocations: from_vec(graph.inbound.allocations.take(&nd)),
            usages: from_vec(graph.inbound.usages.take(&nd)),
            traceability: Traceability {
                derivations: from_vec(graph.inbound.derivations.take(&nd)),
                refinements: from_vec(graph.inbound.refinements.take(&nd)),
                specializations: from_vec(graph.inbound.specializations.take(&nd)),
                issues: Vec::new(),
            },
            issues: Vec::new(),
        },
        outbound: Outbound {
            restriction: graph
                .outbound
                .restriction
                .remove(&nd)
                .map(Connection::from)
                .ok_or(IssueKind::MissingRequirementBinding),
            allocation: graph.outbound.allocation.remove(&nd).map(Connection::from),
            usages: from_vec(graph.outbound.usages.take(&nd)),
            traceability: Traceability {
                derivations: from_vec(graph.outbound.derivations.take(&nd)),
                refinements: from_vec(graph.outbound.refinements.take(&nd)),
                specializations: from_vec(graph.outbound.specializations.take(&nd)),
                issues: Vec::new(),
            },
        },
    }
}
