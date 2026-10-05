use std::collections::HashMap;

use crate::{
    node::{
        self,
        expression::{Arena, Idx},
    },
    semantics::{link::FindCtx, path::Path},
};

// INVARIANT: When an expression is referenced, J is always a valid set_element
#[derive(Debug, Clone)]
pub enum Target<I = node::entity::Idx, J = Idx> {
    Outer(super::Target<I>),
    Inner(J),
}

impl<I, J> Target<I, J> {
    pub fn outer(&self) -> Option<&super::Target<I>> {
        match self {
            Target::Outer(out) => Some(out),
            _ => None,
        }
    }

    pub fn outer_mut(&mut self) -> Option<&mut super::Target<I>>
    where
        I: Copy,
    {
        match self {
            Target::Outer(out) => Some(out),
            _ => None,
        }
    }
}

pub type Link<I = node::entity::Idx, J = Idx> = super::Link<Target<I, J>>;
pub type Node<P> = node::expression::Node<P, Link>;

pub(super) fn link<P, R: Into<Path>>(
    ctx: &FindCtx,
    arena: Arena<node::expression::Node<P, R>>,
) -> Arena<Node<P>> {
    arena.from_walk(HashMap::new(), |node, mut current, idx| {
        let node = node.mapr(|rf| {
            let r = rf.into();
            match find(ctx, &current, &r) {
                Some(tgt) => Link::valid(r, tgt),
                None => Link::invalid(r),
            }
        });
        if let Some(set) = node.variant.set_element() {
            current.insert(set.variable.clone(), idx);
        }
        (current, node)
    })
}

fn find(ctx: &FindCtx, local: &HashMap<String, Idx>, rf: &Path) -> Option<Target> {
    if let Some(tgt) = super::Target::find(ctx, rf) {
        return Some(Target::Outer(tgt));
    }

    if let Some(set) = local.get(rf.as_str()) {
        return Some(Target::Inner(*set));
    }

    return None;
}
