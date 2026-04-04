use crate::{
    node,
    reference::Reference,
    semantics::reference::{WeakNodeRef, entity},
    syntax::{self},
};

pub type Node = node::markup::Node<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;

pub(super) fn link(paths: &entity::SubPaths, node: &mut Node) {
    for rf in node.references_mut() {
        if let Some(found) = entity::find(paths.entity, rf.as_ref().err().unwrap(), &paths.ctx) {
            *rf = Ok(found.clone());
        }
    }
}

pub(super) fn init_nodes(node: syntax::parser::markup::Node) -> Node {
    node.mapr(|r| Err::<WeakNodeRef<entity::Node>, Reference>(r))
}
