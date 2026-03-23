use std::{collections::HashMap, fmt::Display, ops::ControlFlow};

use serde::{Deserialize, Serialize};

use crate::syntax::entity::{Entity, EntityKey, REFERENCE_SEPARATOR, Reference};
use crate::{
    children::Children,
    visitor::{Visitor, Walk, WalkCustom},
};

#[derive(Default, Debug)]
struct Mapper<'a> {
    current: Vec<String>,
    map: PathMap<'a>,
}

impl<'a> Walk<&'a Entity> for Mapper<'a> {
    fn walk(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.current.push(node.meta.label.clone());
        self.visit(node)?;
        if let Some(children) = node.children() {
            for n in children {
                self.walk(n)?;
            }
        }
        self.current.pop();
        ControlFlow::Continue(())
    }
}

impl<'a> Visitor<&'a Entity> for Mapper<'a> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'a Entity) -> ControlFlow<()> {
        self.map.insert(EntityKey(node), (&self.current).into());
        ControlFlow::Continue(())
    }
}

impl<'a> From<&'a Entity> for PathMap<'a> {
    fn from(node: &'a Entity) -> Self {
        let mut mapper = Mapper::default();
        let _ = mapper.walk(node);
        mapper.map
    }
}

pub type PathMap<'a> = HashMap<EntityKey<'a>, Path>;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Path(pub String);

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(raw) = self;
        write!(f, "{}", raw)
    }
}

impl PartialEq<str> for Path {
    fn eq(&self, other: &str) -> bool {
        let Self(self_raw) = self;
        other == self_raw
    }
}

impl From<&Vec<String>> for Path {
    fn from(labels: &Vec<String>) -> Self {
        Path(labels.join(REFERENCE_SEPARATOR))
    }
}

impl From<String> for Path {
    fn from(label: String) -> Self {
        Path(label)
    }
}

impl From<Reference> for Path {
    fn from(reference: Reference) -> Self {
        Path(reference.value)
    }
}

impl Path {
    pub fn is_prefix(&self, other: &Path) -> bool {
        let Self(s) = self;
        let Self(o) = other;
        o.starts_with(s)
    }

    pub fn intersects(&self, other: &Path) -> bool {
        let Self(s) = self;
        let Self(o) = other;
        s.starts_with(o) || o.starts_with(s)
    }

    pub fn depth(&self) -> usize {
        let Self(raw) = self;
        raw.split("::").count()
    }

    pub fn raw(&self) -> &str {
        let Self(raw) = self;
        &raw[..]
    }

    pub fn last(&self) -> &str {
        let Self(raw) = self;
        raw.rsplit("::").next().unwrap()
    }

    pub fn first(&self) -> &str {
        let Self(raw) = self;
        raw.split("::").next().unwrap()
    }

    pub fn parent(&self) -> Option<&str> {
        let Self(raw) = self;
        let slice_idx = raw.rfind("::")?;
        Some(&raw[..slice_idx])
    }

    pub fn merged(&self, reference: &str) -> Self {
        let Self(raw) = self;
        let slice_idx = raw.rfind("::").unwrap_or(raw.len());
        let parent = &raw[..slice_idx];
        Path(format!("{}::{}", parent, reference))
    }

    pub fn appended(&self, reference: &str) -> Self {
        let Self(raw) = self;
        Path(format!("{}::{}", raw, reference))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_path_with_reference() {
        let path = Path("spec::sys".to_string());
        let reference = Reference::new("sys::mass".to_string());
        let result = path.merged(&reference.value);
        assert_eq!(result.raw(), "spec::sys::mass");
    }

    #[test]
    fn appends_path_with_reference() {
        let path = Path("spec::sys".to_string());
        let reference = Reference::new("inner::mass".to_string());
        let result = path.appended(&reference.value);
        assert_eq!(result.raw(), "spec::sys::inner::mass");
    }
}
