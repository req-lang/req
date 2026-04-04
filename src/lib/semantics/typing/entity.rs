use std::rc::Rc;

use crate::{
    node::{self, ConversionMap, NodeRef},
    reference::Reference,
    semantics::{
        reference::{self, WeakNodeRef},
        typing::{expression, markup},
    },
    utils::children::ChildrenIter,
};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    Result<WeakNodeRef<Node>, Reference>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Debug)]
pub struct Node {
    pub variant: Variant,
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

pub(super) fn evaluate(node: NodeRef<Node>) {
    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    for expr in node.borrow().variant.expressions() {
        expression::evaluate(Rc::clone(expr));
    }
}

pub(super) fn init_node(
    node: NodeRef<reference::entity::Node>,
    entities: &ConversionMap<reference::entity::Node, Node>,
    expressions: &mut ConversionMap<reference::expression::Node, expression::Node>,
) -> NodeRef<Node> {
    let key = Rc::as_ptr(&node);

    let inner = node::unwrap(node);
    let variant = inner
        .variant
        .mape(|n| expression::init_node(n, entities, expressions))
        .mapn(|n| init_node(n, entities, expressions))
        .mapm(|m| markup::init_node(m, entities))
        .mapr(|r| r.map(|weak| Rc::downgrade(&entities[&weak.as_ptr()])));
    let node = Rc::clone(&entities[&key]);
    node.borrow_mut().variant = variant;
    node
}
