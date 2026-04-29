#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Node<P, R> {
    pub payload: P,
    pub container: Container<R>,
}

impl<P, R> Node<P, R> {
    pub fn empty(container: Container<R>) -> Self
    where
        P: Default,
    {
        Self {
            payload: Default::default(),
            container,
        }
    }

    pub fn new(payload: P, container: Container<R>) -> Self {
        Self { payload, container }
    }

    pub fn map<F, Q>(self, f: F) -> Node<Q, R>
    where
        F: FnOnce(P) -> Q,
    {
        Node {
            payload: f(self.payload),
            container: self.container,
        }
    }

    pub fn mapr<F, S>(self, f: F) -> Node<P, S>
    where
        F: FnMut(R) -> S,
    {
        Node {
            payload: self.payload,
            container: self.container.mapr(f),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item<R> {
    Text(String),
    Reference(R),
    Newline,
}

impl<R> Item<R> {
    pub fn mapr<F, S>(self, f: F) -> Item<S>
    where
        F: FnOnce(R) -> S,
    {
        match self {
            Item::Reference(r) => Item::Reference(f(r)),
            Item::Text(t) => Item::Text(t),
            Item::Newline => Item::Newline,
        }
    }
}

pub trait IntoItems<R> {
    fn into_items(self) -> Vec<Item<R>>;
}

/// This container owns the markdown text associated with markup tokens.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Container<R> {
    pub items: Vec<Item<R>>,
}

impl<R> Default for Container<R> {
    fn default() -> Self {
        Self {
            items: Default::default(),
        }
    }
}

impl<R> From<Vec<Item<R>>> for Container<R> {
    fn from(items: Vec<Item<R>>) -> Self {
        Self { items }
    }
}

impl<R> Container<R> {
    pub fn mapr<F, S>(self, mut f: F) -> Container<S>
    where
        F: FnMut(R) -> S,
    {
        Container {
            items: self.items.into_iter().map(|i| i.mapr(&mut f)).collect(),
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
