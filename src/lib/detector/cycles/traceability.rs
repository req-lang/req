use std::collections::HashSet;

use crate::{mapper::requirement::TraceabilityMap, syntax::entity::EntityKey};

pub struct Detector<'a> {
    forward: &'a TraceabilityMap<'a>,
    in_path: HashSet<EntityKey<'a>>,
    path: Vec<EntityKey<'a>>,
    visited: HashSet<EntityKey<'a>>,
    pub cycles: Vec<Vec<EntityKey<'a>>>,
}

impl<'a> Detector<'a> {
    pub fn new(forward: &'a TraceabilityMap) -> Self {
        Self {
            forward,
            path: Vec::new(),
            in_path: HashSet::new(),
            visited: HashSet::new(),
            cycles: Vec::new(),
        }
    }

    pub fn detect(&mut self) {
        let roots: Vec<EntityKey<'a>> = self.forward.keys().cloned().collect();
        for node in roots {
            if !self.visited.contains(&node) {
                self.dfs(node);
            }
        }
    }

    fn dfs(&mut self, node: EntityKey<'a>) {
        if self.in_path.contains(&node) {
            let pos = self.path.iter().position(|n| *n == node).unwrap();
            self.cycles.push(self.path[pos..].to_vec());
            return;
        }

        if self.visited.contains(&node) {
            return;
        }

        self.in_path.insert(node.clone());
        self.path.push(node.clone());

        let neighbors: Vec<EntityKey<'a>> = self
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

// TODO: Readd tests
