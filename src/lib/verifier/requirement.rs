use crate::{
    diagnostic::Diagnostic,
    mapper::{id::NodeMap, requirement::RequirementMap},
    syntax::NodeId,
};

pub struct Verifier<'a> {
    nodes_by_id: &'a NodeMap<'a>,
    map: &'a RequirementMap,
    cycles: &'a Vec<Vec<NodeId>>,
    types: &'a Vec<Vec<NodeId>>,
    pub issues: Vec<Diagnostic>,
}
