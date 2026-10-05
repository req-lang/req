use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

mod entity;

use crate::{
    node::entity::{Arena, Idx, Node},
    semantics::path::entity::count_duplicates,
    syntax::tokenizer::entity::REFERENCE_SEPARATOR,
};

pub trait Located {
    fn location(&self) -> &Location;
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("Two entities have the same name at the same location")]
    Duplicated,
}

pub trait Locate<R, T, E, M> {
    fn locate(self) -> Arena<Node<Location, R, T, E, M>>;
}

impl<N, R, T, E, M> Locate<R, T, E, M> for Arena<Node<N, R, T, E, M>> {
    fn locate(self) -> Arena<Node<Location, R, T, E, M>> {
        let paths = count_duplicates(collect(&self));
        entity::discover(paths, self)
    }
}

pub fn collect<P, R, T, E, M>(arena: &Arena<Node<P, R, T, E, M>>) -> HashMap<Path, Vec<Idx>> {
    let mut paths = HashMap::new();
    entity::collect(&arena, &mut paths);
    paths
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: Path,
    pub issue: Option<Issue>,
}

impl Location {
    fn valid(path: Path) -> Self {
        Self { path, issue: None }
    }

    fn invalid(path: Path, issue: Issue) -> Self {
        Self {
            path,
            issue: Some(issue),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsRef<Path> for Location {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Located for Location {
    fn location(&self) -> &Self {
        &self
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(String);

impl Path {
    pub fn new<P: Into<String>>(value: P) -> Self {
        Self(value.into())
    }

    pub fn parent(&self) -> Option<Self> {
        let v = self.as_str();
        let idx = v.rfind(REFERENCE_SEPARATOR)?;
        Some(Self::new(&v[..idx]))
    }

    pub fn label(&self) -> &str {
        let v = self.as_str();
        match v.rfind(REFERENCE_SEPARATOR) {
            None => v,
            Some(idx) => &v[idx + REFERENCE_SEPARATOR.len()..],
        }
    }

    pub fn join<P: ToString>(&self, rhs: &P) -> Self {
        let rhs = rhs.to_string();

        if self.as_str().is_empty() {
            return Self(rhs);
        }

        let Self(inner) = self;

        let sep = REFERENCE_SEPARATOR;
        let mut buf = String::with_capacity(inner.len() + sep.len() + rhs.len());
        buf.push_str(inner);
        buf.push_str(sep);
        buf.push_str(&rhs);

        Self(buf)
    }

    pub fn as_str(&self) -> &str {
        let Self(inner) = self;
        inner.as_str()
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for Path {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
impl From<String> for Path {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn creates_from_str() {
        let s = "this::is::a::path";
        let path = Path::new(s);
        assert_eq!(path.as_str(), s);
    }

    #[test]
    fn joins() {
        let s: Path = "this::is::a".into();
        let t: Path = "path".into();
        let expected: Path = "this::is::a::path".into();
        assert_eq!(expected, s.join(&t));
    }

    #[test]
    fn gets_parent() {
        let s: Path = "this::is::a::path".into();
        let expected: Path = "this::is::a".into();
        assert_eq!(expected, s.parent().unwrap());
    }

    #[test]
    fn gets_label() {
        let s: Path = "this::is::a::path".into();
        let expected = "path";
        assert_eq!(expected, s.label());
    }
}
