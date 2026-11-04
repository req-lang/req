use crate::node::tree::Tree;

pub mod entity;
pub mod expression;
pub mod markup;
mod padding;

pub fn format<W: std::io::Write>(tree: &Tree, out: &mut W) -> std::io::Result<()> {
    entity::format(tree, tree.root(), 0, out)
}

pub fn format_string(tree: &Tree, indent: usize) -> String {
    let mut buf = Vec::new();
    entity::format(tree, tree.root(), indent, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

pub fn format_to_file(tree: &Tree, path: &std::path::Path) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    entity::format(tree, tree.root(), 0, &mut file)
}
