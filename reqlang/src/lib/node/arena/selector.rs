use crate::node::arena::{Arena, idx::Idx};

#[derive(Debug)]
pub struct Selector<'a, T, M> {
    arena: &'a Arena<T, M>,
    idx: Idx<M>,
}

impl<'a, T, M> Selector<'a, T, M> {
    pub fn new(arena: &'a Arena<T, M>, idx: Idx<M>) -> Self {
        Self { arena, idx }
    }
}

impl<'a, T, M> From<Selector<'a, T, M>> for (&'a Arena<T, M>, Idx<M>) {
    fn from(value: Selector<'a, T, M>) -> Self {
        (value.arena, value.idx)
    }
}

impl<'a, T, M> From<(&'a Arena<T, M>, Idx<M>)> for Selector<'a, T, M> {
    fn from((arena, idx): (&'a Arena<T, M>, Idx<M>)) -> Self {
        Selector::new(arena, idx)
    }
}

impl<'a, T, M> From<&'a Arena<T, M>> for Selector<'a, T, M> {
    fn from(value: &'a Arena<T, M>) -> Self {
        Self {
            arena: value,
            idx: Idx::root(),
        }
    }
}

impl<'a, T, M> Clone for Selector<'a, T, M> {
    fn clone(&self) -> Self {
        Self {
            arena: self.arena,
            idx: self.idx,
        }
    }
}

impl<'a, T, M> Copy for Selector<'a, T, M> {}
