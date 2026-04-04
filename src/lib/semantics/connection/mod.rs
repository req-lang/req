use crate::{
    node::NodeRef,
    semantics::{
        connection::graph::Graph,
        reference::{self, convert::convert},
    },
    utils::iter::NodeIter,
};

pub mod cycles;
pub mod entity;
pub mod expression;
pub mod graph;
pub mod issue;
pub mod markup;

pub fn connect(node: NodeRef<reference::entity::Node>) -> NodeRef<entity::Node> {
    let node = convert(node);
    let mut graph = Graph::default();

    graph::build_graph(node.clone(), &mut graph, None);
    entity::connect(node.clone(), &mut graph);

    cycles::detect_traceability_cycles(node.clone());
    for nd in NodeIter::new(node.clone()) {
        issue::collect_issues(nd);
    }
    node
}
