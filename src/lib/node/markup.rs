// TODO: Detect newlines
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item<R> {
    Text(String),
    Reference(R),
    Newline,
}

impl<R> Item<R> {
    pub fn mapr<F, S>(self, f: F) -> Item<S>
    where
        F: Fn(R) -> S,
    {
        match self {
            Item::Reference(r) => Item::Reference(f(r)),
            Item::Text(t) => Item::Text(t),
            Item::Newline => Item::Newline,
        }
    }
}

/// This container owns the markdown text associated with markup tokens.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Node<R> {
    pub items: Vec<Item<R>>,
}

impl<R> Default for Node<R> {
    fn default() -> Self {
        Self {
            items: Default::default(),
        }
    }
}

impl<R> Node<R> {
    pub fn mapr<F, S>(self, f: F) -> Node<S>
    where
        F: Fn(R) -> S,
    {
        Node {
            items: self.items.into_iter().map(|i| i.mapr(|x| f(x))).collect(),
        }
    }

    pub fn references(&self) -> impl Iterator<Item = &R> {
        self.items.iter().filter_map(|item| match item {
            Item::Reference(r) => Some(r),
            _ => None,
        })
    }

    pub fn references_mut(&mut self) -> impl Iterator<Item = &mut R> {
        self.items.iter_mut().filter_map(|item| match item {
            Item::Reference(r) => Some(r),
            _ => None,
        })
    }
}
