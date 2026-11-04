use crate::node::reference;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item {
    Text(String),
    Reference(reference::Idx),
    Newline,
}

/// This container owns the markdown text associated with markup tokens.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Markup {
    pub items: Vec<Item>,
}

impl From<Vec<Item>> for Markup {
    fn from(items: Vec<Item>) -> Self {
        Self { items }
    }
}

impl Markup {
    pub fn references(&self) -> impl Iterator<Item = reference::Idx> {
        self.items.iter().filter_map(|item| match item {
            Item::Reference(r) => Some(*r),
            _ => None,
        })
    }
}
