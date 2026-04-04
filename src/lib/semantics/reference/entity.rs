use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    node::{self, NodeRef, entity::IntoVariable},
    reference::Reference,
    semantics::reference::{Result, WeakNodeRef, expression, markup},
    syntax::{self},
    utils::children::ChildrenIter,
};

pub type Variant = node::entity::Variant<
    NodeRef<Node>,
    Result<WeakNodeRef<Node>>,
    NodeRef<expression::Node>,
    markup::Node,
>;

#[derive(Default, Clone)]
pub(super) struct Ctx {
    path: Reference,
    imports: Vec<Reference>,
}

#[derive(Default, Debug)]
pub struct Node {
    pub variant: Variant,
}

impl IntoVariable<NodeRef<Node>, Result<WeakNodeRef<Node>>, NodeRef<expression::Node>, markup::Node>
    for Node
{
    fn into_variant(
        self,
    ) -> node::entity::Variant<
        NodeRef<Node>,
        Result<WeakNodeRef<Node>>,
        NodeRef<expression::Node>,
        markup::Node,
    > {
        self.variant
    }
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

pub(super) struct SubPaths<'a> {
    pub entity: &'a Paths,
    pub ctx: &'a Ctx,
}

#[derive(Debug, Default)]
pub(super) struct Paths {
    nodes: HashMap<Reference, Weak<RefCell<Node>>>,
}

pub(super) fn link(paths: &Paths, node: NodeRef<Node>, ctx: Ctx) {
    let sub = SubPaths {
        entity: &paths,
        ctx: &ctx,
    };

    let imports = match node.borrow().variant.imports() {
        Some(i) => i.iter().map(|i| i.clone().err().unwrap()).collect(),
        None => Vec::new(),
    };

    {
        let mut node_ref = node.borrow_mut();

        for rf in node_ref.variant.references_mut() {
            if let Some(found) = find(paths, rf.as_ref().err().unwrap(), &ctx) {
                *rf = Ok(found.clone());
            }
        }

        for expr in node_ref.variant.expressions() {
            expression::link(&sub, expr.clone(), HashMap::new());
        }

        for markup in node_ref.variant.markups_mut() {
            markup::link(&sub, markup);
        }
    }

    let node_ref = node.borrow();
    let label = &node_ref.variant.meta().label[..];
    let path = &ctx.path + &label[..];

    let ctx = Ctx { path, imports };

    for child in node_ref.variant.children_iter() {
        link(paths, child, ctx.clone());
    }
}

pub(super) fn find(paths: &Paths, reference: &Reference, ctx: &Ctx) -> Option<WeakNodeRef<Node>> {
    if let Some(direct) = paths.nodes.get(reference) {
        return Some(direct.clone());
    }

    let child = &ctx.path + reference;
    if let Some(child) = paths.nodes.get(&child) {
        return Some(child.clone());
    }

    for import in &ctx.imports {
        let imported = import + reference;
        if let Some(imported) = paths.nodes.get(&imported) {
            return Some(imported.clone());
        }
    }

    None
}

pub(super) fn build_paths<'n>(node: NodeRef<Node>, parent: &Reference, out: &mut Paths) {
    let current = parent + &node.borrow().variant.meta().label[..];

    for child in node.borrow().variant.children_iter() {
        build_paths(child, &current, out);
    }

    out.nodes.insert(current, Rc::downgrade(&node));
}

pub(super) fn init_nodes(node: syntax::entity::Node) -> Node {
    let variant = node
        .variant
        .mapr(|r| Err::<Weak<RefCell<Node>>, Reference>(r));
    let variant = variant.mape(|n| node::map(n, expression::init_node));
    let variant = variant.mapn(|n| node::map(n, init_nodes));
    let variant = variant.mapm(markup::init_nodes);
    Node { variant }
}
