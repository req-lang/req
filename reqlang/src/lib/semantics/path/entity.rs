use std::collections::HashMap;

use crate::{
    node::{
        arena::ArenaEnumerate,
        entity::{Arena, Idx, Node},
    },
    semantics::path::{Issue, Location, Path},
    utils::upsert::Upsert,
};

pub fn discover<N, R, T, E, M>(
    mut paths: HashMap<Idx, (Path, bool)>,
    arena: Arena<Node<N, R, T, E, M>>,
) -> Arena<Node<Location, R, T, E, M>> {
    arena
        .into_iter()
        .indices()
        .map(|(idx, node)| {
            let (path, is_duplicate) = paths.remove(&idx).unwrap();
            node.map(|_| match is_duplicate {
                true => Location::valid(path),
                _ => Location::invalid(path, Issue::Duplicated),
            })
        })
        .collect()
}

pub(crate) fn collect<'n, P, R, T, E, M>(
    arena: &Arena<Node<P, R, T, E, M>>,
    out: &mut HashMap<Path, Vec<Idx>>,
) {
    arena
        .walk(Path::default(), |node, parent, idx| {
            let current = parent.join(&node.meta.label);
            out.upsert(current.clone(), idx);
            current
        })
        .for_each(drop);
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
