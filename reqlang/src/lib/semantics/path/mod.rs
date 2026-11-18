use std::collections::HashMap;

use crate::node::entity::{Arena, Idx, Node};

pub mod entity;

pub use crate::node::path::Path;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("two entities have the same name at the same location")]
    Duplicated,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: Path,
    pub issue: Option<Issue>,
}

impl From<Path> for Location {
    fn from(value: Path) -> Self {
        Location {
            path: value,
            issue: None,
        }
    }
}

pub fn paths(entities: &Arena<Node>) -> Arena<Location> {
    let mut paths = entities.parallel();
    entity::paths(entities, &mut paths);
    paths.into_iter().map(Location::from).collect()
}

pub fn reverse(paths: &Arena<Location>) -> HashMap<&Path, Idx> {
    paths
        .enumerate()
        .map(|(idx, loc)| (&loc.path, idx))
        .collect()
}

pub fn duplicates(paths: &mut Arena<Location>) {
    entity::duplicates(paths);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::entity::{Meta, Package, Variant};

    fn package(label: &str, children: &[usize]) -> Node {
        let children = children.iter().map(|&c| Idx::from(c)).collect();
        Node::new(
            Meta::from(label.to_string()),
            Variant::Package(Package::new(Vec::new(), children)),
        )
    }

    // INVARIANT: the parser stores children before their parent, the root is the last entity
    //
    // root (3)
    // ├── a (1)
    // │   └── c (0)
    // └── b (2)
    fn sample() -> Arena<Node> {
        vec![
            package("c", &[]),
            package("a", &[0]),
            package("b", &[]),
            package("root", &[1, 2]),
        ]
        .into()
    }

    #[test]
    fn root_path_is_its_label() {
        let paths = paths(&vec![package("root", &[])].into());
        assert_eq!(paths[Idx::from(0)].path, Path::from("root"));
    }

    #[test]
    fn paths_join_labels_from_the_root() {
        let paths = paths(&sample());
        let root = Path::from("root");
        let a = root.join(&"a");
        let c = a.join(&"c");
        let b = root.join(&"b");

        assert_eq!(paths[Idx::from(0)].path, c);
        assert_eq!(paths[Idx::from(1)].path, a);
        assert_eq!(paths[Idx::from(2)].path, b);
        assert_eq!(paths[Idx::from(3)].path, root);
    }

    #[test]
    fn paths_are_normalized() {
        // "é" as `e` followed by a combining acute accent, against the single code point
        let paths = paths(&vec![package("re\u{301}sume\u{301}", &[])].into());
        assert_eq!(paths[Idx::from(0)].path, Path::from("r\u{e9}sum\u{e9}"));
    }

    #[test]
    fn reverse_finds_each_entity() {
        let paths = paths(&sample());
        let reverse = reverse(&paths);

        assert_eq!(reverse.len(), 4);
        for (idx, path) in paths.enumerate() {
            assert_eq!(reverse[&path.path], idx);
        }
    }

    #[test]
    fn reverse_keeps_the_last_of_a_shared_path() {
        let entities: Arena<Node> = vec![
            package("a", &[]),
            package("a", &[]),
            package("root", &[0, 1]),
        ]
        .into();
        let paths = paths(&entities);
        let reverse = reverse(&paths);

        assert_eq!(reverse.len(), 2);
        assert_eq!(reverse[&Path::from("root").join(&"a")], Idx::from(1));
    }

    #[test]
    fn no_duplicates_when_paths_differ() {
        let mut locs = paths(&sample());
        duplicates(&mut locs);
        assert!(locs.into_iter().all(|loc| loc.issue.is_none()));
    }

    #[test]
    fn same_label_under_different_parents_is_not_a_duplicate() {
        let entities: Arena<Node> = vec![
            package("x", &[]),
            package("a", &[0]),
            package("x", &[]),
            package("b", &[2]),
            package("root", &[1, 3]),
        ]
        .into();
        let mut locs = paths(&entities);
        duplicates(&mut locs);
        assert!(locs.into_iter().all(|loc| loc.issue.is_none()));
    }

    #[test]
    fn flags_every_entity_sharing_a_path() {
        let entities: Arena<Node> = vec![
            package("a", &[]),
            package("a", &[]),
            package("b", &[]),
            package("root", &[0, 1, 2]),
        ]
        .into();
        let mut locs = paths(&entities);
        duplicates(&mut locs);

        assert!(matches!(locs[Idx::from(0)].issue, Some(Issue::Duplicated)));
        assert!(matches!(locs[Idx::from(1)].issue, Some(Issue::Duplicated)));
        assert!(matches!(locs[Idx::from(2)].issue, None));
        assert!(matches!(locs[Idx::from(3)].issue, None));
    }
}
