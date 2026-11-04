use crate::{
    node::{markup::Markup, tree::Tree},
    renderer::formatter::padding::pad,
};

pub fn format<W: std::io::Write>(
    tree: &Tree,
    markup: &Markup,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::markup::Item::*;

    writeln!(out, "{}@@", pad(indent))?;

    write!(out, "{}", pad(indent))?;
    for item in &markup.items {
        match item {
            Text(text) => write!(out, "{}", text)?,
            Reference(reference) => write!(out, "{{{}}}", tree.references[*reference])?,
            Newline => write!(out, "\n{}", pad(indent))?,
        }
    }

    writeln!(out, "\n{}@@", pad(indent))
}
