use std::collections::HashMap;

use crate::{
    node::{NodeRef, build_stub_map},
    semantics::{
        connection::{entity::init_node, graph::Graph},
        typing,
    },
    utils::iter::NodeIter,
};

pub mod cycles;
pub mod entity;
pub mod expression;
pub mod graph;
pub mod issue;
pub mod markup;

pub fn connect(node: NodeRef<typing::entity::Node>) -> NodeRef<entity::Node> {
    let mut entities = HashMap::new();
    let mut expressions = HashMap::new();
    build_stub_map(&node, &mut entities);

    let node = init_node(node, &entities, &mut expressions);
    let mut graph = Graph::default();

    graph::build_graph(node.clone(), &mut graph, None);
    entity::connect(node.clone(), &mut graph);

    cycles::detect_traceability_cycles(node.clone());
    for nd in NodeIter::new(node.clone()) {
        issue::collect_issues(nd);
    }
    node
}
