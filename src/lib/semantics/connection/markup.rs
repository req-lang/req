use crate::{
    node::{self, markup::IntoItems},
    reference::Reference,
    semantics::{
        connection::entity::{self},
        reference::{self, WeakNodeRef},
    },
};

pub type Container = node::markup::Container<reference::Result<WeakNodeRef<entity::Node>>>;

#[derive(Default, Debug, Clone)]
pub struct Node {
    pub container: Container,
}

pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;

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
