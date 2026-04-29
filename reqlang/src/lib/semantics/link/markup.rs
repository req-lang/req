use crate::{
    node::{self},
    reference::Reference,
    semantics::link::{Result, entity},
};

pub type Node<P> = node::markup::Node<P, Result<node::entity::Idx>>;

pub(super) fn link<P, R: Into<Reference> + Clone>(
    paths: &entity::FindCtx<R>,
    node: node::markup::Node<P, R>,
) -> Node<P> {
    node.mapr(|r| entity::find(paths, r))
}
