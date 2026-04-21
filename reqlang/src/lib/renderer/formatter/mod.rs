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

pub fn format_to_file(node: Ref<'_, Node>, path: &std::path::Path) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    entity::format(node, 0, &mut file)
}
