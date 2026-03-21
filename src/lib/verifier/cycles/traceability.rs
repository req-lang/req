use std::collections::HashSet;

use crate::{
    diagnostic::{Diagnostic, DiagnosticKind},
    mapper::{path::PathMap, traceability::TraceabilityMap},
    syntax::NodeId,
};

pub struct Verifier<'a> {
    paths_by_id: &'a PathMap,
    traceability: &'a TraceabilityMap,
    in_path: HashSet<NodeId>,
    path: Vec<NodeId>,
    visited: HashSet<NodeId>,
    cycles: Vec<Vec<NodeId>>,
}

impl From<Verifier<'_>> for Vec<Diagnostic> {
    fn from(mut verifier: Verifier<'_>) -> Self {
        let roots: Vec<NodeId> = verifier.traceability.keys().copied().collect();
        for node in roots {
            if !verifier.visited.contains(&node) {
                verifier.find(node);
            }
        }

        let mut issues = Vec::new();
        for cycle in verifier.cycles {
            let paths: Vec<_> = cycle
                .iter()
                .map(|n| verifier.paths_by_id.get(n).unwrap().clone())
                .collect();
            issues.extend(cycle.into_iter().map(|n| {
                let kind = DiagnosticKind::CircularTrace(paths.clone());
                Diagnostic::new(n, kind)
            }));
        }

        issues
    }
}

impl<'a> Verifier<'a> {
    pub fn new(traceability: &'a TraceabilityMap, paths_by_id: &'a PathMap) -> Self {
        Self {
            traceability,
            paths_by_id,
            path: Vec::new(),
            in_path: HashSet::new(),
            visited: HashSet::new(),
            cycles: Vec::new(),
        }
    }

    fn find(&mut self, node: NodeId) {
        if self.in_path.contains(&node) {
            let pos = self.path.iter().position(|&n| n == node).unwrap();
            self.cycles.push(self.path[pos..].to_vec());
            return;
        }

        if self.visited.contains(&node) {
            return;
        }

        self.in_path.insert(node);
        self.path.push(node);

        let neighbors: Vec<NodeId> = self
            .traceability
            .get(&node)
            .map(|ns| ns.clone())
            .unwrap_or_default();

        for neighbor in neighbors {
            self.find(neighbor);
        }

        self.path.pop();
        self.in_path.remove(&node);
        self.visited.insert(node);
    }
}

#[cfg(test)]
mod tests {
    pub const INVALID_ROOT: &str = "tests/data/invalid/cycles/traceability";
    pub const VALID_ROOT: &str = "tests/data/valid/cycles/traceability";

    use crate::{Analysis, syntax::NodeParser};

    use super::Verifier;

    fn cycle_count(source: &str) -> usize {
        let mut parser = NodeParser::default();
        let entity = parser.parse(source).expect("parse failed");
        let analysis = Analysis::from(&entity);
        let verifier = Verifier::new(&analysis.traceabilities, &analysis.paths);
        Vec::from(verifier).len()
    }

    fn read_invalid(name: &str) -> String {
        let path = format!("{INVALID_ROOT}/{name}");
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("could not read {path}"))
    }

    fn read_valid(name: &str) -> String {
        let path = format!("{VALID_ROOT}/{name}");
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("could not read {path}"))
    }

    #[test]
    fn detects_no_cycle_when_no_traceability() {
        assert_eq!(cycle_count(&read_valid("no_traceability.req")), 0);
    }

    #[test]
    fn detects_no_cycle_in_linear_chain() {
        assert_eq!(cycle_count(&read_valid("linear_chain.req")), 0);
    }

    #[test]
    fn detects_direct_cycle() {
        assert!(cycle_count(&read_invalid("direct.req")) > 0);
    }

    #[test]
    fn detects_longer_cycle() {
        assert!(cycle_count(&read_invalid("longer.req")) > 0);
    }

    #[test]
    fn detects_multiple_independent_cycles() {
        assert!(cycle_count(&read_invalid("multiple.req")) > 0);
    }
}
