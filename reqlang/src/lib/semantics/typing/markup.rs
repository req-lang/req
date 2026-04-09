use reqlang_derive::IntoItems;

use crate::{
    node::{
        self,
        markup::{Container, IntoItems},
    },
    semantics::{
        link::{self},
        typing::entity,
    },
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
