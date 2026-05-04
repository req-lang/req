use std::collections::HashSet;

use crate::{
    node::arena::{Arena, idx::Idx},
    utils::children::{Children, ChildrenIter, ChildrenMut},
};

pub trait Replace<M> {
    fn idx_iter(&self) -> impl Iterator<Item = Idx<M>>;

    fn replace_idx(&mut self, f: impl FnMut(&mut Idx<M>));
}

pub trait Mutation<T, M> {
    fn insert(&mut self, elem: T, parent: Idx<M>, after: Option<Idx<M>>);

    fn remove(&mut self, idx: Idx<M>) -> T;

    fn extend(&mut self, elem: impl Iterator<Item = T>, parent: Idx<M>) {
        for el in elem {
            self.insert(el, parent, None);
        }
    }

    fn displace(&mut self, idx: Idx<M>, parent: Idx<M>, after: Option<Idx<M>>) {
        let elem = self.remove(idx);
        self.insert(elem, parent, after);
    }
}

impl<T: Replace<M> + ChildrenIter<Idx<M>> + ChildrenMut<Idx<M>> + Children<Idx<M>>, M>
    Mutation<T, M> for Arena<T, M>
{
    fn insert(&mut self, elem: T, parent: Idx<M>, after: Option<Idx<M>>) {
        let children = self[parent].children().expect("not a parent");
        let (idx, pos) = match after {
            Some(after) => {
                let pos = children.into_iter().position(|child| *child == after);
                let pos = pos.expect("before not found in parent") + 1;
                let idx = usize::from(after) + 1;
                (idx.into(), pos)
            }
            None => {
                let pos = children.len();
                let idx = usize::from(parent) + pos + 1;
                (idx.into(), pos)
            }
        };

        for node in self.inner.iter_mut() {
            node.replace_idx(|rf| {
                if *rf >= idx {
                    *rf = (usize::from(*rf) + 1).into();
                }
            });
        }
        self[parent].children_mut().unwrap().insert(pos, idx);
        self.inner.insert(idx.into(), elem);
    }

    fn remove(&mut self, idx: Idx<M>) -> T {
        let descendants: HashSet<_> = self.descendants(idx).collect();
        if self.inner.iter().enumerate().any(|(i, node)| {
            !descendants.contains(&Idx::from(i))
                && node.idx_iter().any(|rf| descendants.contains(&rf))
        }) {
            panic!("element or its children still referenced");
        }

        let (parent, pos) = self
            .into_iter()
            .enumerate()
            .filter_map(|(idx, n)| n.children().map(|children| (idx, children)))
            .find_map(|(parent, children)| {
                children
                    .iter()
                    .position(|child| *child == idx)
                    .map(|pos| (parent.into(), pos))
            })
            .expect("parent element not found");

        while let Some(&child) = self[idx].children().and_then(|c| c.first()) {
            self.remove(child);
        }

        for node in self.into_iter() {
            node.replace_idx(|rf| {
                if *rf > idx {
                    *rf = (usize::from(*rf) - 1).into();
                }
            });
        }

        self[parent].children_mut().unwrap().remove(pos);
        self.inner.remove(idx.into())
    }
}
