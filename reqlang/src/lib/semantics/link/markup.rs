use crate::{
    node::{self},
    semantics::{
        link::{FindCtx, Link},
        path::Path,
    },
};

pub type Node<P> = node::markup::Node<P, Link>;

pub(super) fn link<P, R: Into<Path>>(paths: &FindCtx, node: node::markup::Node<P, R>) -> Node<P> {
    node.mapr(|r| {
        let r = r.into();
        match super::entity::find(paths, &r) {
            Some(tgt) => Link::valid(r, tgt),
            None => Link::invalid(r),
        }
    })
}
