use crate::{
    node::{self, markup::IntoItems},
    semantics::{
        reference::{self, WeakNodeRef},
        typing::entity,
    },
};

pub type Item = node::markup::Item<reference::Result<WeakNodeRef<entity::Node>>>;
pub type Container = node::markup::Container<reference::Result<WeakNodeRef<entity::Node>>>;

#[derive(Default, Debug, Clone)]
pub struct Node {
    container: Container,
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
