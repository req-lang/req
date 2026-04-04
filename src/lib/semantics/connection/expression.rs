use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    node::{self, NodeRef},
    reference::Reference,
    semantics::{
        connection::entity,
        reference::expression::Target,
        typing::{
            self,
            expression::{Issue, TypeKind},
        },
    },
    utils::children::ChildrenIter,
};

pub type Variant =
    node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>;

#[derive(Debug)]
pub struct Node {
    pub variant: Variant,
    pub typ: Result<TypeKind, Issue>,
}

impl ChildrenIter<NodeRef<Node>> for NodeRef<Node> {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.borrow()
            .variant
            .children_iter()
            .map(|n| n.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

pub(super) type Map = HashMap<*const RefCell<typing::expression::Node>, NodeRef<Node>>;

pub(super) fn init_node(
    node: NodeRef<typing::expression::Node>,
    entities: &entity::Map,
    expressions: &mut Map,
) -> NodeRef<Node> {
    let typ = node.borrow().typ.clone();
    let key = Rc::as_ptr(&node);
    let inner = node::unwrap(node);
    let variant = inner
        .variant
        .mape(|child_ref| init_node(child_ref, entities, expressions))
        .mapr(|r| r.map(|t| convert_target(t, entities, expressions)));
    let node = node::new(Node { variant, typ });
    expressions.insert(key, node.clone());
    node
}

fn convert_target(
    target: Target<typing::entity::Node, typing::expression::Node>,
    entities: &entity::Map,
    expressions: &Map,
) -> Target<entity::Node, Node> {
    use Target::*;

    match target {
        Builtin(kind) => Builtin(kind),
        Entity(weak) => Entity(Rc::downgrade(&entities[&weak.as_ptr()])),
        Expression(weak) => Expression(Rc::downgrade(&expressions[&weak.as_ptr()])),
    }
}
