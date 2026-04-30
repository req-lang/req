use crate::{
    node::{self},
    semantics::link::{Link, entity},
    semantics::path::Path,
};

pub type Node<P> = node::markup::Node<P, Link>;

pub(super) fn link<P, R: Into<Path>>(
    paths: &entity::FindCtx,
    node: node::markup::Node<P, R>,
) -> Node<P> {
    node.mapr(|r| entity::find(paths, r.into()))
}
