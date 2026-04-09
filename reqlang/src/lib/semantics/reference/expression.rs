use std::{collections::HashMap, rc::Rc};

use crate::{
    node::{self, NodeRef, expression::IntoVariable},
    reference::Reference,
    semantics::reference::{
        WeakNodeRef,
        entity::{self},
    },
    syntax,
    utils::children::ChildrenIter,
};
use reqlang_derive::IntoExpressionVariable;

#[derive(Debug, Clone, Copy)]
pub enum BuiltinKind {
    Real,
    Integer,
    Boolean,
}

impl TryFrom<&str> for BuiltinKind {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use BuiltinKind::*;

        match value {
            "real" => Ok(Real),
            "integer" => Ok(Integer),
            "boolean" => Ok(Boolean),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Target<N, E> {
    Entity(WeakNodeRef<N>),
    Expression(WeakNodeRef<E>),
    Builtin(BuiltinKind),
}

#[derive(Debug, Clone, IntoExpressionVariable)]
pub struct Node {
    pub variant: node::expression::Variant<NodeRef<Node>, super::Result<Target<entity::Node, Node>>>,
}

pub(super) fn link(
    paths: &entity::SubPaths,
    node: NodeRef<Node>,
    mut local: HashMap<String, WeakNodeRef<Node>>,
) {
    use node::expression::Variant::*;

    if let Some(set) = node.borrow().variant.set_element() {
        local.insert(set.variable.clone(), Rc::downgrade(&set.domain));
    }

    if let Identifier(ident) = &mut node.borrow_mut().variant {
        let rf = ident.target.as_ref().err().unwrap();
        if let Some(found) = find(paths, &local, rf) {
            ident.target = Ok(found);
        }
    }

    for child in node.borrow().variant.children_iter() {
        link(paths, child, local.clone());
    }
}

fn find(
    paths: &entity::SubPaths,
    local: &HashMap<String, WeakNodeRef<Node>>,
    rf: &Reference,
) -> Option<Target<entity::Node, Node>> {
    if let Ok(builtin) = BuiltinKind::try_from(&rf.value[..]) {
        return Some(Target::Builtin(builtin));
    }

    if let Some(expr) = local.get(&rf.value) {
        return Some(Target::Expression(expr.clone()));
    }

    if let Some(entity) = entity::find(paths.entity, rf, paths.ctx) {
        return Some(Target::Entity(entity.clone()));
    }

    None
}

pub(super) fn init_node(node: syntax::expression::Node) -> Node {
    let variant = node.variant.mapr(|r| Err::<Target<_, _>, Reference>(r));
    let variant = variant.mape(|n| node::map(n, init_node));
    Node { variant }
}
