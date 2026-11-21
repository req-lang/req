use crate::{
    node::{markup::Idx, tree::Tree},
    renderer::formatter::padding::pad,
    syntax::tokenizer::markup::{REFERENCE_END, REFERENCE_START},
};

pub fn format<W: std::io::Write>(
    tree: &Tree,
    idx: Idx,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::markup::Item::*;

    let node = &tree.markups[idx];
    writeln!(out, "{}@@", pad(indent))?;

    write!(out, "{}", pad(indent))?;
    for item in node.unindented() {
        match item {
            Text(text) => write!(out, "{}", text)?,
            Reference(reference) => write!(
                out,
                "{REFERENCE_START}{}{REFERENCE_END}",
                tree.references[reference]
            )?,
            Newline => write!(out, "\n{}", pad(indent))?,
            Indent(indent) => write!(out, "{}", indent)?,
        }
    }

    write!(out, "\n{}@@", pad(indent))
}
