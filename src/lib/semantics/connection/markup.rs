use crate::{
    node::{self},
    reference::Reference,
    semantics::{
        connection::entity::{self},
        reference::WeakNodeRef,
    },
};

pub type Node = node::markup::Node<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;
