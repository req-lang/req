use crate::{
    node::{self, markup::IntoItems},
    reference::Reference,
    semantics::reference::{self, WeakNodeRef, entity},
    syntax::{self},
};

pub type Container = node::markup::Container<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;

#[derive(Default, Debug, Clone)]
pub struct Node {
    pub container: Container,
}

impl IntoItems<reference::Result<WeakNodeRef<entity::Node>>> for Node {
    fn into_items(self) -> Vec<node::markup::Item<reference::Result<WeakNodeRef<entity::Node>>>> {
        self.container.items
    }
}

impl From<Container> for Node {
    fn from(container: Container) -> Self {
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
