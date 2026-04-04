use std::rc::Rc;

use crate::{
    node::{self, NodeRef},
    reference::Reference,
    semantics::{
        connection::entity,
        reference::{
            convert::{ConversionMap, convert_target},
            expression::Target,
        },
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

pub(super) fn init_node(
    node: NodeRef<typing::expression::Node>,
    entities: &ConversionMap<typing::entity::Node, entity::Node>,
    expressions: &mut ConversionMap<typing::expression::Node, Node>,
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
