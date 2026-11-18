use std::slice;

use crate::node::{arena, reference};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
pub type Arena<T> = arena::Arena<T, IdxMarker>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item {
    Text(String),
    Reference(reference::Idx),
    Newline,
    Indent(String),
}

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Markup {
    pub items: Vec<Item>,
}

impl From<Vec<Item>> for Markup {
    fn from(items: Vec<Item>) -> Self {
        Self { items }
    }
}

pub struct Unindented<'a> {
    items: slice::Iter<'a, Item>,
    indentation: usize,
}

impl Iterator for Unindented<'_> {
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.items.next()? {
                Item::Indent(indent) => {
                    let kept = &indent[self.indentation.min(indent.len())..];
                    if !kept.is_empty() {
                        return Some(Item::Indent(kept.to_string()));
                    }
                }
                item => return Some(item.clone()),
            }
        }
    }
}

impl Markup {
    pub fn unindented(&self) -> Unindented<'_> {
        let skipped = match self.items.first() {
            Some(Item::Newline) => 1,
            _ => 0,
        };
        Unindented {
            items: self.items[skipped..].iter(),
            indentation: self.indentation(),
        }
    }

    fn indentation(&self) -> usize {
        self.items
            .split(|item| *item == Item::Newline)
            .skip(1)
            .filter(|line| !is_blank(line))
            .map(|line| match line.first() {
                Some(Item::Indent(indent)) => indent.len(),
                _ => 0,
            })
            .min()
            .unwrap_or(0)
    }

    pub fn references(&self) -> impl Iterator<Item = reference::Idx> {
        self.items.iter().filter_map(|item| match item {
            Item::Reference(r) => Some(*r),
            _ => None,
        })
    }
}

/// Whether the items of a line are nothing but indentation
fn is_blank(line: &[Item]) -> bool {
    line.iter().all(|item| matches!(item, Item::Indent(_)))
}
