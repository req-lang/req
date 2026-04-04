use std::cell::Ref;

use crate::syntax::entity::Node;

pub mod entity;
pub mod expression;
pub mod markup;
mod padding;

pub fn format_string(node: Ref<'_, Node>, indent: usize) -> String {
    let mut buf = Vec::new();
    entity::format(node, indent, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}
