use std::fmt::Display;

use crate::{node::markup::Node, renderer::formatter::padding::pad};

pub fn format<W: std::io::Write, M, R: Display>(
    node: &Node<M, R>,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::markup::Item::*;

    writeln!(out, "{}@@", pad(indent))?;

    write!(out, "{}", pad(indent))?;
    for item in &node.container.items {
        match item {
            Text(text) => write!(out, "{}", text)?,
            Reference(reference) => write!(out, "{{{}}}", reference)?,
            Newline => write!(out, "\n{}", pad(indent))?,
        }
    }

    writeln!(out, "\n{}@@", pad(indent))
}
