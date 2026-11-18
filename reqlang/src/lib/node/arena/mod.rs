use std::{collections::HashSet, marker::PhantomData};

use crate::{node::arena::idx::Idx, utils::children::ChildrenIter};

pub mod idx;

/// Arena containing elements that can reference to each other
/// - Might have children that implements [ChildrenIter] with [Idx]
/// - Can have other references as [Idx]
/// - Cannot be mutated directly, only elements can be mutated
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arena<T, M> {
    inner: Vec<T>,
    phantom: PhantomData<M>,
}

impl<T, M> Default for Arena<T, M> {
    fn default() -> Self {
        Vec::new().into()
    }
}

impl<T, M> Arena<T, M> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn last_idx(&self) -> Idx<M> {
        (self.inner.len() - 1).into()
    }

    pub fn indices(&self) -> impl Iterator<Item = Idx<M>> + use<T, M> {
        (0..self.inner.len()).map(Idx::from)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value)
    }

    pub fn enumerate(&self) -> impl DoubleEndedIterator<Item = (Idx<M>, &T)> {
        self.inner
            .iter()
            .enumerate()
            .map(|(i, t)| (Idx::from(i), t))
    }

    pub fn parallel<U: Default + Clone>(&self) -> Arena<U, M> {
        std::iter::repeat(Default::default())
            .take(self.inner.len())
            .collect::<Vec<_>>()
            .into()
    }

    pub fn subtree<'a>(&'a self, root: Idx<M>) -> SubtreeIter<'a, T, M>
    where
        T: ChildrenIter<Idx<M>>,
    {
        SubtreeIter {
            arena: self,
            stack: vec![root],
        }
    }

    pub fn parents(&self) -> Arena<Option<Idx<M>>, M>
    where
        T: ChildrenIter<Idx<M>>,
    {
        let mut parents = self.parallel();
        for (idx, node) in self.enumerate() {
            for child in node.children_iter() {
                parents[child] = Some(idx);
            }
        }
        parents
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

pub struct SubtreeIter<'a, T, M> {
    arena: &'a Arena<T, M>,
    stack: Vec<Idx<M>>,
}

impl<'a, T: ChildrenIter<Idx<M>>, M> Iterator for SubtreeIter<'a, T, M> {
    type Item = (Idx<M>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.stack.pop()?;
        let node = &self.arena[idx];
        self.stack.extend(node.children_iter().rev());
        Some((idx, node))
    }
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

#[cfg(test)]
mod tests {
    use crate::{
        node::arena::{Arena, idx::Idx},
        utils::children::ChildrenIter,
    };

    struct Marker;
    struct Node(Vec<usize>);

    impl ChildrenIter<Idx<Marker>> for Node {
        fn children_iter(&self) -> impl DoubleEndedIterator<Item = Idx<Marker>> {
            self.0.iter().map(|c| Idx::from(*c))
        }
    }

    /// 0 -> (1 -> (3, 4), 2)
    fn tree() -> Arena<Node, Marker> {
        vec![
            Node(vec![1, 2]),
            Node(vec![3, 4]),
            Node(vec![]),
            Node(vec![]),
            Node(vec![]),
        ]
        .into()
    }

    #[test]
    fn subtree_visits_a_node_before_its_children() {
        let arena = tree();

        let all: Vec<_> = arena
            .subtree(0.into())
            .map(|(idx, _)| usize::from(idx))
            .collect();
        let below: Vec<_> = arena
            .subtree(1.into())
            .map(|(idx, _)| usize::from(idx))
            .collect();

        assert_eq!(all, vec![0, 1, 3, 4, 2]);
        assert_eq!(below, vec![1, 3, 4]);
    }

    #[test]
    fn parents_are_the_nodes_having_it_as_a_child() {
        let parents: Vec<_> = tree()
            .parents()
            .into_iter()
            .map(|p| p.map(usize::from))
            .collect();

        assert_eq!(parents, vec![None, Some(0), Some(0), Some(1), Some(1)]);
    }
}
