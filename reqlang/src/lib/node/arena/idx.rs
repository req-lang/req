use std::{
    hash::Hash,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::node::arena::Arena;

#[derive(Debug)]
pub struct Idx<M> {
    inner: usize,
    phantom: PhantomData<M>,
}

impl<M> Default for Idx<M> {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            phantom: Default::default(),
        }
    }
}

impl<M> Clone for Idx<M> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}
impl<M> Copy for Idx<M> {}

impl<M> PartialEq for Idx<M> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<M> Eq for Idx<M> {}

impl<M> PartialOrd for Idx<M> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<M> Ord for Idx<M> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<M> Hash for Idx<M> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<M> Idx<M> {
    pub fn root() -> Self {
        0.into()
    }
}

impl<M> From<Idx<M>> for usize {
    fn from(value: Idx<M>) -> Self {
        value.inner
    }
}

impl<M> From<usize> for Idx<M> {
    fn from(value: usize) -> Self {
        Self {
            inner: value,
            phantom: PhantomData,
        }
    }
}

impl<T, M> IndexMut<Idx<M>> for Arena<T, M> {
    fn index_mut(&mut self, index: Idx<M>) -> &mut Self::Output {
        &mut self.inner[usize::from(index)]
    }
}

impl<T, M> Index<Idx<M>> for Arena<T, M> {
    type Output = T;

    fn index(&self, index: Idx<M>) -> &Self::Output {
        &self.inner[usize::from(index)]
    }
}
