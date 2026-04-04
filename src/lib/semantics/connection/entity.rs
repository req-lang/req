use std::rc::Rc;

use crate::{
    node::{self, ConversionMap, NodeRef, entity::Label},
    reference::Reference,
    semantics::{
        connection::{expression, graph::Graph, issue::IssueKind, markup},
        reference::WeakNodeRef,
        typing,
    },
    utils::{children::ChildrenIter, take::Take},
};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    Result<WeakNodeRef<Node>, Reference>,
    Connection<NodeRef<expression::Node>>,
    markup::Node,
>;

#[derive(Default, Debug)]
pub struct Node {
    pub inbound: Inbound<Connection<WeakNodeRef<Node>>>,
    pub outbound: Outbound<Connection<WeakNodeRef<Node>>>,
    pub parent: Option<Connection<WeakNodeRef<Node>>>,
    pub variant: Variant,
}

impl Label for NodeRef<Node> {
    fn label(&self) -> String {
        self.borrow().variant.meta().label.clone()
    }
}

impl ChildrenIter<NodeRef<Node>> for NodeRef<Node> {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.borrow()
            .variant
            .children_iter()
            .map(|n| n.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[derive(Default, Debug)]
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

#[derive(Default, Debug)]
pub struct Traceability<R> {
    pub derivations: Vec<R>,
    pub refinements: Vec<R>,
    pub specializations: Vec<R>,
    pub issues: Vec<IssueKind>,
}

// Derived from outbound connections
#[derive(Default, Debug)]
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
#[derive(Default, Debug)]
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

    node_ref.parent = graph.parent.remove(&ptr).map(Connection::from);
    node_ref.inbound = Inbound {
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
    node_ref.outbound = Outbound {
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

pub(super) fn init_node(
    node: NodeRef<typing::entity::Node>,
    entities: &ConversionMap<typing::entity::Node, Node>,
    expressions: &mut ConversionMap<typing::expression::Node, expression::Node>,
) -> NodeRef<Node> {
    let key = Rc::as_ptr(&node);
    let inner = node::unwrap(node);
    let variant = inner
        .variant
        .mape(|n| Connection::from(expression::init_node(n, entities, expressions)))
        .mapn(|n| init_node(n, entities, expressions))
        .mapm(|m| markup::init_node(m, entities))
        .mapr(|r| r.map(|weak| Rc::downgrade(&entities[&weak.as_ptr()])));
    let inbound = Inbound::default();
    let node = entities[&key].clone();
    node.borrow_mut().variant = variant;
    node.borrow_mut().inbound = inbound;
    node
}
