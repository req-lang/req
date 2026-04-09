use reqlang_derive::IntoItems;

use crate::{
    node::markup::Container,
    node::{self, markup::IntoItems},
    reference::Reference,
    semantics::link::{self, WeakNodeRef, entity},
    syntax::{self},
};

#[derive(Default, Debug, Clone, IntoItems)]
pub struct Node {
    pub container: Container<link::ResultRef<entity::Node>>,
}

impl From<Container<link::ResultRef<entity::Node>>> for Node {
    fn from(container: Container<link::ResultRef<entity::Node>>) -> Self {
        Self { container }
    }
}

pub(super) fn link(paths: &entity::SubPaths, node: &mut Node) {
    for rf in node.container.references_mut() {
        if let Some(found) = entity::find(paths.entity, rf.as_ref().err().unwrap(), &paths.ctx) {
            *rf = Ok(found.clone());
        }
    }
}

pub(super) fn init_nodes(node: syntax::parser::markup::Node) -> Node {
    Node {
        container: node
            .container
            .mapr(|r| Err::<WeakNodeRef<entity::Node>, Reference>(r)),
    }
}
