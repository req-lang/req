use std::collections::HashSet;

use crate::{mapper::requirement::ForwardMap, syntax::NodeId};

pub struct Detector<'a> {
    forward: &'a ForwardMap,
    in_path: HashSet<NodeId>,
    path: Vec<NodeId>,
    visited: HashSet<NodeId>,
    pub cycles: Vec<Vec<NodeId>>,
}

impl<'a> Detector<'a> {
    pub fn new(forward: &'a ForwardMap) -> Self {
        Self {
            forward,
            path: Vec::new(),
            in_path: HashSet::new(),
            visited: HashSet::new(),
            cycles: Vec::new(),
        }
    }

    pub fn detect(&mut self) {
        let roots: Vec<NodeId> = self.forward.keys().copied().collect();
        for node in roots {
            if !self.visited.contains(&node) {
                self.dfs(node);
            }
        }
    }

    fn dfs(&mut self, node: NodeId) {
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
            .forward
            .get(&node)
            .map(|ns| ns.clone())
            .unwrap_or_default();

        for neighbor in neighbors {
            self.dfs(neighbor);
        }

        self.path.pop();
        self.in_path.remove(&node);
        self.visited.insert(node);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::syntax::NodeId;

    use super::Detector;

    fn id() -> NodeId {
        NodeId::new()
    }

    #[test]
    fn detects_no_cycle_in_empty_map() {
        let map = HashMap::new();
        let mut detector = Detector::new(&map);
        detector.detect();
        assert!(detector.cycles.is_empty());
    }

    #[test]
    fn detects_no_cycle_in_linear_chain() {
        let (a, b, c) = (id(), id(), id());
        let map = HashMap::from([(a, vec![b]), (b, vec![c])]);
        let mut detector = Detector::new(&map);
        detector.detect();
        assert!(detector.cycles.is_empty());
    }

    #[test]
    fn detects_direct_cycle() {
        let (a, b) = (id(), id());
        let map = HashMap::from([(a, vec![b]), (b, vec![a])]);
        let mut detector = Detector::new(&map);
        detector.detect();
        assert_eq!(detector.cycles.len(), 1);
    }

    #[test]
    fn detects_longer_cycle() {
        let (a, b, c) = (id(), id(), id());
        let map = HashMap::from([(a, vec![b]), (b, vec![c]), (c, vec![a])]);
        let mut detector = Detector::new(&map);
        detector.detect();
        assert_eq!(detector.cycles.len(), 1);
        assert_eq!(detector.cycles[0].len(), 3);
    }

    #[test]
    fn detects_multiple_independent_cycles() {
        let (a, b, c, d) = (id(), id(), id(), id());
        let map = HashMap::from([(a, vec![b]), (b, vec![a]), (c, vec![d]), (d, vec![c])]);
        let mut detector = Detector::new(&map);
        detector.detect();
        assert_eq!(detector.cycles.len(), 2);
    }
}
