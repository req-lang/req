use std::collections::HashMap;

use crate::{
    node::entity::{Idx, Node},
    semantics::path::{Issue, Location, Path},
    utils::{children::ChildrenIter, upsert::Upsert},
};

pub fn discover<N, R, T, E, M>(
    mut paths: HashMap<Idx, (Path, bool)>,
    arena: Vec<Node<N, R, T, E, M>>,
) -> Vec<Node<Location, R, T, E, M>> {
    arena
        .into_iter()
        .enumerate()
        .map(|(idx, node)| {
            let (path, is_duplicate) = paths.remove(&idx.into()).unwrap();
            node.map(|_| match is_duplicate {
                true => Location::valid(path),
                _ => Location::invalid(path, Issue::Duplicated),
            })
        })
        .collect()
}

pub(super) fn collect<'n, P, R, T, E, M>(
    arena: &Vec<Node<P, R, T, E, M>>,
    idx: Idx,
    parent: &Path,
    out: &mut HashMap<Path, Vec<Idx>>,
) {
    let node = &arena[idx];
    let current = parent.join(&node.meta.label);

    for child in node.variant.children_iter() {
        collect(arena, child, &current, out);
    }

    out.upsert(current, idx);
}

pub(super) fn count_duplicates(paths: HashMap<Path, Vec<Idx>>) -> HashMap<Idx, (Path, bool)> {
    let mut counts: HashMap<Idx, (Path, bool)> = HashMap::new();

    for (p, idx) in paths {
        for idx in idx {
            match counts.remove(&idx) {
                Some((p, _)) => {
                    counts.insert(idx, (p, true));
                }
                None => {
                    counts.insert(idx, (p.clone(), false));
                }
            }
        }
    }
    counts
}
