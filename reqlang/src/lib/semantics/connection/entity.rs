use std::rc::Rc;

use crate::{
    node::{
        self, NodeRef,
        entity::{Label, VariableMut},
    },
    semantics::{
        connection::{expression, graph::Graph, issue::IssueKind, markup},
        link::{self, WeakNodeRef},
    },
    utils::{children::ChildrenIter, take::Take},
};
use reqlang_derive::{ChildrenIter, EntityVariableMut};

#[derive(Default, Debug, Clone, ChildrenIter, EntityVariableMut)]
pub struct Node {
    pub neighbors: Neighbors<Connection<WeakNodeRef<Node>>>,
    pub variant: node::entity::Variant<
        NodeRef<Node>,
        link::Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    >,
}

impl Label for Node {
    fn label(&self) -> String {
        self.variant.meta().label.clone()
    }
}

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

fn from_vec(x: Vec<WeakNodeRef<Node>>) -> Vec<Connection<WeakNodeRef<Node>>> {
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

pub(super) fn connect(node: NodeRef<Node>, graph: &mut Graph) {
    let ptr = Rc::as_ptr(&node);
    let mut node_ref = node.borrow_mut();

    node_ref.neighbors.parent = graph.parent.remove(&ptr).map(Connection::from);
    node_ref.neighbors.inbound = Inbound {
        restrictions: from_vec(graph.inbound.restrictions.take(&ptr)),
        allocations: from_vec(graph.inbound.allocations.take(&ptr)),
        usages: from_vec(graph.inbound.usages.take(&ptr)),
        traceability: Traceability {
            derivations: from_vec(graph.inbound.derivations.take(&ptr)),
            refinements: from_vec(graph.inbound.refinements.take(&ptr)),
            specializations: from_vec(graph.inbound.specializations.take(&ptr)),
            issues: Vec::new(),
        },
        issues: Vec::new(),
    };
    node_ref.neighbors.outbound = Outbound {
        restriction: graph
            .outbound
            .restriction
            .remove(&ptr)
            .map(Connection::from)
            .ok_or(IssueKind::MissingRequirementBinding),
        allocation: graph.outbound.allocation.remove(&ptr).map(Connection::from),
        usages: from_vec(graph.outbound.usages.take(&ptr)),
        traceability: Traceability {
            derivations: from_vec(graph.outbound.derivations.take(&ptr)),
            refinements: from_vec(graph.outbound.refinements.take(&ptr)),
            specializations: from_vec(graph.outbound.specializations.take(&ptr)),
            issues: Vec::new(),
        },
    };

    for child in node_ref.variant.children_iter() {
        connect(child, graph);
    }
}
