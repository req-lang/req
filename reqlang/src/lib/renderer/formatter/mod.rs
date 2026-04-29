use std::fmt::Display;

use crate::node;

pub mod entity;
pub mod expression;
pub mod markup;
mod padding;

pub fn format_string<N, R: Display, T: Display, E, M>(
    arena: &Vec<node::entity::Node<N, R, T, E, M>>,
    indent: usize,
) -> String {
    let mut buf = Vec::new();
    entity::format(arena, 0.into(), indent, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

pub fn format_to_file<N, R: Display, T: Display, E, M>(
    arena: &Vec<node::entity::Node<N, R, T, E, M>>,
    path: &std::path::Path,
) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    entity::format(arena, 0.into(), 0, &mut file)
}
