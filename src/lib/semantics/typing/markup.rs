use crate::{
    node::{self},
    reference::Reference,
    semantics::{reference::WeakNodeRef, typing::entity},
};

pub type Node = node::markup::Node<Result<WeakNodeRef<entity::Node>, Reference>>;
pub type Item = node::markup::Item<Result<WeakNodeRef<entity::Node>, Reference>>;
