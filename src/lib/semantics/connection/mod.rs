use std::collections::HashMap;

use crate::{
    node::NodeRef,
    semantics::{
        connection::{
            entity::{build_entity_map, init_node},
            graph::Graph,
        },
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
    let mut entities: entity::Map = HashMap::new();
    let mut expressions: expression::Map = HashMap::new();
    build_entity_map(&node, &mut entities);

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
