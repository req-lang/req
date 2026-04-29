use std::{fmt::Display, ops::Add};

use crate::syntax::tokenizer::entity::REFERENCE_SEPARATOR;

// TODO: Rename with something more adequate, Reference collapses with Rust refs

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference(String);

impl Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(v) = self;
        write!(f, "{}", v)
    }
}

impl Reference {
    pub fn new<S: ToString>(value: S) -> Self {
        Reference(value.to_string())
    }

    pub fn len(&self) -> usize {
        self.as_str().split(REFERENCE_SEPARATOR).count()
    }

    pub fn label(&self) -> &str {
        let v = self.as_str();
        match v.rfind(REFERENCE_SEPARATOR) {
            Some(idx) => &v[idx + 2..],
            None => &v,
        }
    }

    pub fn parent(&self) -> &str {
        let v = self.as_str();
        match v.rfind(REFERENCE_SEPARATOR) {
            Some(idx) => &v[..idx],
            None => &v,
        }
    }

    pub fn as_str(&self) -> &str {
        let Self(v) = self;
        &v[..]
    }
}

impl<S: AsRef<str>> Add<S> for &Reference {
    type Output = Reference;

    fn add(self, rhs: S) -> Self::Output {
        let ret = match self.as_str() {
            "" => format!("{}", rhs.as_ref()),
            prefix => format!("{}{}{}", prefix, REFERENCE_SEPARATOR, rhs.as_ref()),
        };

        Reference::new(ret)
    }
}

impl AsRef<str> for Reference {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
