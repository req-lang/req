use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    node::entity::{Idx, Node},
    semantics::path::{Issue, Location, Path},
    utils::{children::ChildrenIter, upsert::Upsert},
};

pub fn discover<N, R: Into<Path>, T: Into<Path>, E, M>(
    paths: Vec<(Path, usize)>,
    arena: Vec<Node<N, R, T, E, M>>,
) -> Vec<Node<Location, Path, Path, E, M>> {
    arena
        .into_iter()
        .zip(paths.into_iter())
        .map(|(node, (path, count))| {
            node.map(|_| match count {
                1 => Location::valid(path),
                _ => Location::invalid(path, Issue::Duplicated),
            })
            .mapr(|r| r.into())
            .mapt(|t| t.into())
        })
        .collect()
}

pub(super) fn collect<'n, P, R, T, E, M>(
    arena: &Vec<Node<P, R, T, E, M>>,
    idx: Idx,
    parent: &Path,
    out: &mut HashMap<Path, Idx>,
) {
    let node = &arena[idx];
    let current = parent.join(&node.meta.label);

    for child in node.variant.children_iter() {
        collect(arena, child, &current, out);
    }

    out.insert(current, idx);
}

pub(super) fn count_duplicates(paths: HashMap<Path, Idx>) -> Vec<(Path, usize)> {
    let mut counts: HashMap<Idx, Vec<Path>> = HashMap::new();

    for (p, idx) in paths {
        counts.upsert(idx, p);
    }

    counts
        .into_iter()
        .sorted_by_key(|(idx, _)| *idx)
        .map(|(_, mut refs)| (refs.pop().unwrap(), refs.len() + 1))
        .collect()
}
