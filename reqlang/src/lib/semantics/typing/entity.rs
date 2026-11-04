use crate::{
    node::{expression::Arena, tree::Tree},
    semantics::{
        link::Links,
        typing::{Result, expression},
    },
};

/// Fills the type of the expressions of the tree, every one of them is known once it returns
pub fn evaluate(tree: &Tree, links: &Links, types: &mut Arena<Option<Result>>) {
    for idx in tree.expressions.indices() {
        expression::collect_types(tree, links, idx, types);
    }
}
