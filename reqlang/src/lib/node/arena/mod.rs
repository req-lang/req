use std::{collections::HashSet, iter::FusedIterator, marker::PhantomData};

use crate::{node::arena::idx::Idx, utils::children::ChildrenIter};

pub mod idx;
pub mod mutation;
pub mod selector;

/// Arena containing elements that can reference to each other
/// - Might have children that implements [ChildrenIter] with [Idx]
/// - Can have other references as [Idx]
/// - Must be constructed from DFS ordered tree
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Arena<T, M> {
    inner: Vec<T>,
    phantom: PhantomData<M>,
}

impl<T, M> Arena<T, M> {
    pub fn with_capacity<U: Default + Clone>(&self) -> Arena<U, M> {
        std::iter::repeat(Default::default())
            .take(self.inner.len())
            .collect::<Vec<_>>()
            .into()
    }

    pub fn descendants<'a>(&'a self, idx: Idx<M>) -> DescendantIter<'a, T, M> {
        DescendantIter {
            arena: self,
            stack: vec![idx],
        }
    }

    pub fn walk<'a, U>(
        &'a self,
        ctx: U,
        f: impl FnMut(&T, U, Idx<M>) -> U,
    ) -> WalkIter<'a, T, U, impl FnMut(&T, U, Idx<M>) -> U, M>
    where
        U: Clone,
        T: ChildrenIter<Idx<M>>,
    {
        WalkIter {
            remaining: self.inner.len(),
            arena: self,
            stack: vec![(Idx::root(), ctx)],
            f,
        }
    }

    pub fn walk_mut<'a, U>(
        &'a mut self,
        ctx: U,
        f: impl FnMut(&mut T, U, Idx<M>) -> U,
    ) -> WalkIterMut<'a, T, U, impl FnMut(&mut T, U, Idx<M>) -> U, M>
    where
        U: Clone,
        T: ChildrenIter<Idx<M>>,
    {
        WalkIterMut {
            remaining: self.inner.len(),
            arena: self,
            stack: vec![(Idx::root(), ctx)],
            f,
        }
    }

    pub fn from_walk<U, V>(self, ctx: U, mut f: impl FnMut(T, U, Idx<M>) -> (U, V)) -> Arena<V, M>
    where
        U: Clone,
        T: ChildrenIter<Idx<M>>,
    {
        let len = self.inner.len();
        let mut slots: Vec<Option<T>> = self.inner.into_iter().map(Some).collect();
        let mut stack: Vec<(Idx<M>, U)> = vec![(Idx::root(), ctx)];
        let mut ret: Vec<Option<V>> = (0..len).map(|_| None).collect();
        let mut buf: Vec<Idx<M>> = Vec::new();

        while let Some((idx, ctx)) = stack.pop() {
            let node = slots[usize::from(idx)].take().unwrap();
            buf.clear();
            buf.extend(node.children_iter());
            let (ctx, value) = f(node, ctx, idx);
            ret[usize::from(idx)] = Some(value);
            stack.extend(buf.iter().copied().map(|c| (c, ctx.clone())).rev());
        }

        ret.into_iter()
            .map(|v| v.unwrap())
            .collect::<Vec<_>>()
            .into()
    }

    pub fn cycles<I, F>(&self, mut f: F) -> Vec<Vec<Idx<M>>>
    where
        I: IntoIterator<Item = Idx<M>>,
        F: FnMut(&T) -> I,
    {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        for i in 0..self.inner.len() {
            let idx = Idx::from(i);
            if !visited.contains(&idx) {
                let ctx = CycleCtx::default();
                self.cycles_inner(idx, &mut f, ctx, &mut visited, &mut result);
            }
        }
        result
    }

    fn cycles_inner<I, F>(
        &self,
        idx: Idx<M>,
        f: &mut F,
        mut ctx: CycleCtx<M>,
        visited: &mut HashSet<Idx<M>>,
        cycles: &mut Vec<Vec<Idx<M>>>,
    ) where
        I: IntoIterator<Item = Idx<M>>,
        F: FnMut(&T) -> I,
    {
        if ctx.in_path.contains(&idx) {
            let pos = ctx.path.iter().position(|n| *n == idx).unwrap();
            cycles.push(ctx.path[pos..].to_vec());
            return;
        }

        if visited.contains(&idx) {
            return;
        }

        let neighbors = f(&self[idx]).into_iter();

        ctx.in_path.insert(idx);
        ctx.path.push(idx);

        for neighbor in neighbors {
            self.cycles_inner(neighbor, f, ctx.clone(), visited, cycles);
        }

        visited.insert(idx);
    }
}

pub struct DescendantIter<'a, T, M> {
    arena: &'a Arena<T, M>,
    stack: Vec<Idx<M>>,
}

impl<'a, T: ChildrenIter<Idx<M>>, M> Iterator for DescendantIter<'a, T, M> {
    type Item = Idx<M>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.stack.pop()?;
        let node = &self.arena[idx];
        let children = node.children_iter().rev();
        self.stack.extend(children);
        Some(idx)
    }
}

pub struct WalkIter<'a, T, U, F, M> {
    remaining: usize,
    arena: &'a Arena<T, M>,
    stack: Vec<(Idx<M>, U)>,
    f: F,
}

impl<'a, T, U, F, M> Iterator for WalkIter<'a, T, U, F, M>
where
    F: FnMut(&T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
    type Item = (&'a T, U, Idx<M>);

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, ctx) = self.stack.pop()?;
        self.remaining -= 1;
        let node = &self.arena[idx];
        let ctx = (self.f)(node, ctx, idx);
        let children = node.children_iter().map(|c| (c, ctx.clone())).rev();
        self.stack.extend(children);
        Some((node, ctx, idx))
    }
}

impl<'a, T, U, F, M> ExactSizeIterator for WalkIter<'a, T, U, F, M>
where
    F: FnMut(&T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<'a, T, U, F, M> FusedIterator for WalkIter<'a, T, U, F, M>
where
    F: FnMut(&T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
}

pub struct WalkIterMut<'a, T, U, F, M> {
    remaining: usize,
    arena: &'a mut Arena<T, M>,
    stack: Vec<(Idx<M>, U)>,
    f: F,
}

impl<'a, T, U, F, M> Iterator for WalkIterMut<'a, T, U, F, M>
where
    F: FnMut(&mut T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
    type Item = (&'a mut T, U, Idx<M>);

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, ctx) = self.stack.pop()?;
        self.remaining -= 1;
        let node = &mut self.arena[idx];
        let ret = std::ptr::from_mut(node);
        let ctx = (self.f)(node, ctx, idx);
        let node = unsafe { ret.as_mut().unwrap() };
        let children = node.children_iter().map(|c| (c, ctx.clone())).rev();
        self.stack.extend(children);
        Some((node, ctx, idx))
    }
}

impl<'a, T, U, F, M> ExactSizeIterator for WalkIterMut<'a, T, U, F, M>
where
    F: FnMut(&mut T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<'a, T, U, F, M> FusedIterator for WalkIterMut<'a, T, U, F, M>
where
    F: FnMut(&mut T, U, Idx<M>) -> U,
    T: ChildrenIter<Idx<M>>,
    U: Clone,
{
}

impl<T, M> IntoIterator for Arena<T, M> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T, M> IntoIterator for &'a Arena<T, M> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, T, M> IntoIterator for &'a mut Arena<T, M> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl<T, M> FromIterator<T> for Arena<T, M> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
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

pub struct Enumerate<I, M> {
    inner: I,
    count: usize,
    phantom: PhantomData<M>,
}

impl<I: Iterator, M> Iterator for Enumerate<I, M> {
    type Item = (Idx<M>, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.inner.next()?;
        let idx = Idx::from(self.count);
        self.count += 1;
        Some((idx, item))
    }
}

pub trait ArenaEnumerate<M>: Iterator + Sized {
    fn indices(self) -> Enumerate<Self, M> {
        Enumerate {
            inner: self,
            count: 0,
            phantom: PhantomData,
        }
    }
}

impl<T, M> ArenaEnumerate<M> for std::slice::Iter<'_, T> {}
impl<T, M> ArenaEnumerate<M> for std::vec::IntoIter<T> {}

struct CycleCtx<M> {
    in_path: HashSet<Idx<M>>,
    path: Vec<Idx<M>>,
}

impl<M> Clone for CycleCtx<M> {
    fn clone(&self) -> Self {
        Self {
            in_path: self.in_path.clone(),
            path: self.path.clone(),
        }
    }
}

impl<M> Default for CycleCtx<M> {
    fn default() -> Self {
        Self {
            in_path: Default::default(),
            path: Default::default(),
        }
    }
}

impl<M> From<Idx<M>> for CycleCtx<M> {
    fn from(value: Idx<M>) -> Self {
        Self {
            in_path: HashSet::from([value]),
            path: vec![value],
        }
    }
}
