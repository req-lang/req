use crate::{renderer::formatter::padding::pad, syntax::parser::markup::Node};

pub fn format<W: std::io::Write>(node: &Node, indent: usize, out: &mut W) -> std::io::Result<()> {
    use crate::node::markup::Item::*;

    writeln!(out, "{}@@", pad(indent))?;

    write!(out, "{}", pad(indent))?;
    for item in &node.items {
        match item {
            Text(text) => write!(out, "{}", text)?,
            Reference(reference) => write!(out, "{{{}}}", reference)?,
            Newline => write!(out, "\n{}", pad(indent))?,
        }
    }

    writeln!(out, "{}@@", pad(indent))
}
