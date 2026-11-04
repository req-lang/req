use std::fmt::Display;

use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::syntax::tokenizer::entity::REFERENCE_SEPARATOR;

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path(String);

impl Path {
    // INVARIANT: the inner string is NFC, so that visually identical paths compare equal
    pub fn new<P: Into<String>>(value: P) -> Self {
        let value = value.into();
        let norm = match is_nfc(&value) {
            true => value,
            false => value.nfc().collect(),
        };
        Self(norm)
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
        let rhs = match is_nfc(&rhs) {
            true => rhs,
            false => rhs.nfc().collect(),
        };

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
        Self::new(value)
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
