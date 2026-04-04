use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use crate::{
    node::{
        self, NodeRef, WeakNodeRef, entity, expression,
        markup::{self, Container},
    },
    semantics::reference::{Result, expression::Target},
    utils::children::ChildrenIter,
};

pub(crate) type ConversionMap<N, O> = HashMap<*const RefCell<N>, NodeRef<O>>;

pub fn convert<N, O, E, G, M, H>(node: NodeRef<N>) -> NodeRef<O>
where
    N: entity::IntoVariable<NodeRef<N>, Result<WeakNodeRef<N>>, NodeRef<E>, M> + Debug + Clone,
    O: entity::VariableMut<NodeRef<O>, Result<WeakNodeRef<O>>, NodeRef<G>, H> + Default,
    E: expression::IntoVariable<NodeRef<E>, Result<Target<N, E>>> + Debug + Clone,
    G: From<expression::Variant<NodeRef<G>, Result<Target<O, G>>>>,
    M: Clone + markup::IntoItems<Result<WeakNodeRef<N>>>,
    H: From<markup::Container<Result<WeakNodeRef<O>>>>,
    NodeRef<N>: ChildrenIter<NodeRef<N>>,
{
    let mut entities = HashMap::new();
    let mut expressions = HashMap::new();

    build_stub_map(&node, &mut entities);
    convert_entity(node, &entities, &mut expressions)
}

fn convert_entity<N, O, E, G, M, H>(
    node: NodeRef<N>,
    entities: &ConversionMap<N, O>,
    expressions: &mut ConversionMap<E, G>,
) -> NodeRef<O>
where
    N: entity::IntoVariable<NodeRef<N>, Result<WeakNodeRef<N>>, NodeRef<E>, M> + Clone,
    O: entity::VariableMut<NodeRef<O>, Result<WeakNodeRef<O>>, NodeRef<G>, H>,
    E: expression::IntoVariable<NodeRef<E>, Result<Target<N, E>>> + Debug + Clone,
    G: From<expression::Variant<NodeRef<G>, Result<Target<O, G>>>>,
    M: Clone + markup::IntoItems<Result<WeakNodeRef<N>>>,
    H: From<markup::Container<Result<WeakNodeRef<O>>>>,
{
    let key = Rc::as_ptr(&node);
    let variant = (*node.borrow())
        .clone()
        .into_variant()
        .mape(|n| convert_expr(n, entities, expressions))
        .mapn(|n| convert_entity(n, entities, expressions))
        .mapm(|m| convert_markup(m, entities))
        .mapr(|r| r.map(|weak| Rc::downgrade(&entities[&weak.as_ptr()])));
    *entities[&key].borrow_mut().variant_mut() = variant;
    let node = Rc::clone(&entities[&key]);
    node
}

fn convert_expr<N, O, E, G>(
    node: NodeRef<E>,
    entities: &ConversionMap<N, O>,
    expressions: &mut ConversionMap<E, G>,
) -> NodeRef<G>
where
    E: expression::IntoVariable<NodeRef<E>, Result<Target<N, E>>> + Debug + Clone,
    G: From<expression::Variant<NodeRef<G>, Result<Target<O, G>>>>,
{
    let key = Rc::as_ptr(&node);
    let variant = (*node.borrow())
        .clone()
        .into_variant()
        .mape(|child_ref| convert_expr(child_ref, entities, expressions))
        .mapr(|r| r.map(|t| convert_target(t, entities, expressions)));
    let node = node::new(G::from(variant));
    expressions.insert(key, Rc::clone(&node));
    node
}

fn convert_markup<N, O, M, H>(node: M, ctx: &ConversionMap<N, O>) -> H
where
    M: Clone + markup::IntoItems<Result<WeakNodeRef<N>>>,
    H: From<markup::Container<Result<WeakNodeRef<O>>>>,
{
    Container {
        items: node
            .into_items()
            .into_iter()
            .map(|r| r.mapr(|r| r.map(|weak| Rc::downgrade(&ctx[&weak.as_ptr()]))))
            .collect(),
    }
    .into()
}

pub(crate) fn convert_target<N, O, E, G>(
    target: Target<N, E>,
    entities: &ConversionMap<N, O>,
    expressions: &ConversionMap<E, G>,
) -> Target<O, G> {
    use Target::*;

    match target {
        Builtin(kind) => Builtin(kind),
        Entity(weak) => Entity(Rc::downgrade(&entities[&weak.as_ptr()])),
        Expression(weak) => Expression(Rc::downgrade(&expressions[&weak.as_ptr()])),
    }
}

fn build_stub_map<N, O>(node: &NodeRef<N>, map: &mut ConversionMap<N, O>)
where
    O: Default,
    NodeRef<N>: ChildrenIter<NodeRef<N>>,
{
    let key = Rc::as_ptr(node);
    let stub = node::new(O::default());
    map.insert(key, stub);
    for child in node.children_iter() {
        build_stub_map(&child, map);
    }
}
