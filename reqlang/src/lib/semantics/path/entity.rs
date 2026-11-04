use std::collections::HashMap;

use crate::{
    node::entity::{Arena, Node},
    semantics::path::{Issue, Location, Path},
};

pub fn paths(entities: &Arena<Node>, paths: &mut Arena<Path>) {
    let parents = entities.parents();
    for (idx, node) in entities.enumerate().rev() {
        paths[idx] = match parents[idx] {
            Some(parent) => paths[parent].join(&node.meta.label),
            None => Path::from(node.meta.label.as_str()),
        };
    }
}

pub fn duplicates(locs: &mut Arena<Location>) {
    let mut counts: HashMap<Path, usize> = HashMap::with_capacity(locs.len());
    for loc in &*locs {
        *counts.entry(loc.path.clone()).or_default() += 1;
    }

    for loc in locs {
        if counts[&loc.path] > 1 {
            loc.issue = Some(Issue::Duplicated);
        }
    }
}
