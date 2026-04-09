use std::rc::Rc;

use crate::{
    node::{
        self, NodeRef,
        entity::{Label, VariableMut},
    },
    reference::Reference,
    semantics::{
        connection::{expression, graph::Graph, issue::IssueKind, markup},
        reference::WeakNodeRef,
    },
    utils::{children::ChildrenIter, take::Take},
};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    Result<WeakNodeRef<Node>, Reference>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Debug, Clone)]
pub struct Node {
    pub neighbors: Neighbors<WeakNodeRef<Node>>,
    pub variant: Variant,
}

impl Label for Node {
    fn label(&self) -> String {
        self.variant.meta().label.clone()
    }
}

impl ChildrenIter<NodeRef<Node>> for Node {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.variant
            .children_iter()
            .map(|n| n.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl
    VariableMut<
        NodeRef<Node>,
        Result<WeakNodeRef<Node>, Reference>,
        NodeRef<expression::Node>,
        markup::Node,
    > for Node
{
    fn variant_mut(
        &mut self,
    ) -> &mut node::entity::Variant<
        NodeRef<Node>,
        Result<WeakNodeRef<Node>, Reference>,
        NodeRef<expression::Node>,
        markup::Node,
    > {
        &mut self.variant
    }
}

#[derive(Default, Debug, Clone)]
pub struct Neighbors<R> {
    pub inbound: Inbound<Connection<R>>,
    pub outbound: Outbound<Connection<R>>,
    pub parent: Option<Connection<R>>,
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

#[derive(Default, Debug, Clone)]
pub struct Traceability<R> {
    pub derivations: Vec<R>,
    pub refinements: Vec<R>,
    pub specializations: Vec<R>,
    pub issues: Vec<IssueKind>,
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

// Explicitely defined in the syntax
#[derive(Default, Debug, Clone)]
pub struct Outbound<R> {
    // Requirement to subject
    pub usages: Vec<R>,
    pub restriction: R,
    pub allocation: Option<R>,
    // Upward/Backward requirements
    pub traceability: Traceability<R>,
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
        restriction: Connection::from(graph.outbound.restriction.take(&ptr)),
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
