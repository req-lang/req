use std::{fmt::Display, ops::Add};

use crate::syntax::tokenizer::entity::REFERENCE_SEPARATOR;

#[derive(Default, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference {
    pub value: String,
}

impl Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Reference {
    pub fn new(value: String) -> Self {
        Reference { value }
    }

    pub fn len(&self) -> usize {
        self.value.split(REFERENCE_SEPARATOR).count()
    }

    pub fn replace_label(&mut self, label: &str) {
        match self.value.rfind(REFERENCE_SEPARATOR) {
            Some(slice_idx) => self.value.replace_range(slice_idx.., label),
            None => self.value = label.to_string(),
        }
    }

    pub fn label(&self) -> &str {
        match self.value.rfind(REFERENCE_SEPARATOR) {
            Some(slice_idx) => &self.value[slice_idx + 2..],
            None => &self.value,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Add<&str> for &Reference {
    type Output = Reference;

    fn add(self, rhs: &str) -> Self::Output {
        let value = &self.value[..];
        let ret = match value {
            "" => rhs.to_string(),
            prefix => format!("{}{}{}", prefix, REFERENCE_SEPARATOR, rhs),
        };

        Reference::new(ret)
    }
}

impl Add<&Reference> for &Reference {
    type Output = Reference;

    fn add(self, rhs: &Reference) -> Self::Output {
        let ret = match &self.value[..] {
            "" => rhs.to_string(),
            prefix => format!("{}{}{}", prefix, REFERENCE_SEPARATOR, rhs.value),
        };

        Reference::new(ret)
    }
}

impl PartialEq<str> for Reference {
    fn eq(&self, other: &str) -> bool {
        self.value == other
    }
}
