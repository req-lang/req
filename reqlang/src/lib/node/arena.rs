use std::{
    marker::PhantomData,
    ops::{Index, IndexMut, Range},
};

use crate::utils::children::ChildrenIter;

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Arena<T, M> {
    inner: Vec<T>,
    phantom: PhantomData<M>,
}

impl<T, M> Arena<T, M> {
    pub fn empty<U: Default + Clone>(&self) -> Arena<U, M> {
        std::iter::repeat(Default::default())
            .take(self.inner.len())
            .collect::<Vec<_>>()
            .into()
    }

    // INVARIANT: Assumes the arena is in DFS order
    pub fn descendants<'a>(&'a self, start: Idx<M>) -> Range<Idx<M>>
    where
        T: ChildrenIter<'a, Idx<M>> + 'a,
        M: 'a + Copy + PartialOrd + Ord,
    {
        let mut end = start;
        self.walk1(start, (), |node, _, _| match node.children_iter().last() {
            Some(idx) if *idx > end => end = *idx,
            _ => {}
        });

        Range { start, end }
    }

    pub fn as_ref(&self) -> Arena<&T, M> {
        self.inner.iter().collect::<Vec<_>>().into()
    }

    pub fn map_<U, F: FnMut(T) -> U>(self, f: F) -> Arena<U, M> {
        self.inner.into_iter().map(f).collect::<Vec<_>>().into()
    }

    pub fn for_each<F: FnMut(&T, Idx<M>)>(&self, mut f: F) {
        self.inner
            .iter()
            .enumerate()
            .for_each(|(idx, t)| f(t, idx.into()));
    }

    pub fn map<U, F: FnMut(T, Idx<M>) -> U>(self, mut f: F) -> Arena<U, M> {
        self.inner
            .into_iter()
            .enumerate()
            .map(|(idx, t)| f(t, idx.into()))
            .collect::<Vec<_>>()
            .into()
    }

    pub fn subrange(&self, range: Range<Idx<M>>) -> Arena<&T, M> {
        self.inner[usize::from(range.start)..usize::from(range.end)]
            .iter()
            .collect::<Vec<_>>()
            .into()
    }

    pub fn zip<U>(self, other: Arena<U, M>) -> Arena<(T, U), M> {
        if self.inner.len() != other.inner.len() {
            panic!("Arena of different sizes");
        }
        self.inner
            .into_iter()
            .zip(other.inner.into_iter())
            .collect::<Vec<_>>()
            .into()
    }

    pub fn zip_map<U, V, F: FnMut(T, U) -> V>(self, other: Arena<U, M>, mut f: F) -> Arena<V, M> {
        if self.inner.len() != other.inner.len() {
            panic!("Arena of different sizes");
        }
        self.inner
            .into_iter()
            .zip(other.inner)
            .map(|(t, u)| f(t, u))
            .collect::<Vec<_>>()
            .into()
    }

    pub fn from_walk<'a, U, V>(&'a self, f: impl FnMut(&'a T, U, Idx<M>) -> (U, V)) -> Arena<V, M>
    where
        M: 'a + Clone + Copy,
        U: Default + Clone,
        T: ChildrenIter<'a, Idx<M>>,
    {
        self.from_walk1(Default::default(), Default::default(), f)
    }

    pub fn from_walk1<'a, U, V, F>(&'a self, idx: Idx<M>, ctx: U, mut f: F) -> Arena<V, M>
    where
        M: 'a + Clone + Copy,
        F: FnMut(&'a T, U, Idx<M>) -> (U, V),
        T: ChildrenIter<'a, Idx<M>>,
        U: Clone,
    {
        let mut ret = Vec::with_capacity(self.inner.len());
        self.inner_from_walk(idx, ctx, &mut f, &mut ret);
        ret.into()
    }

    fn inner_from_walk<'a, U, V, F>(&'a self, idx: Idx<M>, ctx: U, f: &mut F, out: &mut Vec<V>)
    where
        M: 'a + Clone + Copy,
        U: Clone,
        F: FnMut(&'a T, U, Idx<M>) -> (U, V),
        T: ChildrenIter<'a, Idx<M>>,
    {
        let current = &self[idx];
        let (ctx, value) = f(current, ctx, idx);
        out.push(value);

        for child in current.children_iter() {
            self.inner_from_walk(child.clone(), ctx.clone(), f, out);
        }
    }

    pub fn walk<'a, U>(&'a self, f: impl FnMut(&'a T, U, Idx<M>) -> U)
    where
        M: 'a + Clone,
        U: Default + Clone,
        T: ChildrenIter<'a, Idx<M>>,
    {
        self.walk1(Default::default(), Default::default(), f);
    }

    pub fn walk1<'a, U, F>(&'a self, idx: Idx<M>, ctx: U, mut f: F)
    where
        M: 'a + Clone,
        U: Clone,
        F: FnMut(&'a T, U, Idx<M>) -> U,
        T: ChildrenIter<'a, Idx<M>>,
    {
        self.inner_walk1(idx, ctx, &mut f);
    }

    fn inner_walk1<'a, U, F>(&'a self, idx: Idx<M>, ctx: U, f: &mut F)
    where
        M: 'a + Clone,
        U: Clone,
        F: FnMut(&'a T, U, Idx<M>) -> U,
        T: ChildrenIter<'a, Idx<M>>,
    {
        let current = &self[idx.clone()];
        let ctx = f(current, ctx, idx);

        for child in current.children_iter() {
            self.inner_walk1(child.clone(), ctx.clone(), f);
        }
    }
}

impl<T, M> From<Vec<T>> for Arena<T, M> {
    fn from(value: Vec<T>) -> Self {
        Self {
            inner: value,
            phantom: PhantomData,
        }
    }
}

impl<T, M> From<Arena<T, M>> for Vec<T> {
    fn from(value: Arena<T, M>) -> Self {
        value.inner
    }
}

#[derive(Debug, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy)]
pub struct Selector<'a, T, M> {
    arena: &'a Arena<T, M>,
    idx: Idx<M>,
}

impl<'a, T, M> Selector<'a, T, M> {
    pub fn arena(&self) -> &'a Arena<T, M> {
        &self.arena
    }

    pub fn idx(&self) -> Idx<M> {
        self.idx.clone()
    }

    pub fn get(&self) -> &'a T {
        &self.arena[self.idx.clone()]
    }

    pub fn new(arena: &'a Arena<T, M>, idx: Idx<M>) -> Self {
        Self { arena, idx }
    }

    pub fn set(self, idx: Idx<M>) -> Self {
        Self {
            arena: self.arena,
            idx,
        }
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
