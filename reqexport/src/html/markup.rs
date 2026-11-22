use reqlang::{
    Specification,
    node::{
        entity,
        markup::{Idx, Item},
        reference,
        tree::Tree,
    },
};

use crate::html::entity::{anchor, has_anchor};

/// Informal text is Markdown with references in it, written as the author typed it: `{Name}` becomes a
/// link to the entity it names. There is no raw HTML: what looks like a tag is shown as text.
pub fn render<W: std::io::Write>(
    spec: &Specification,
    markup: Idx,
    out: &mut W,
) -> std::io::Result<()> {
    let tree = &spec.tree;
    let references = &spec.analysis.links.references;
    let text: String = tree.markups[markup]
        .unindented()
        .map(|item| stringify_item(tree, references, &item))
        .collect();

    // INVARIANT: Only MDX, which is off, makes the conversion fail.
    let html = markdown::to_html_with_options(&text, &options()).unwrap();
    writeln!(out, "{}", html.trim_end())
}

/// What the Markdown of an informal text may contain: no HTML, no image (a link written by someone else
/// must not make the page load a picture), no link that runs code.
fn options() -> markdown::Options {
    let mut options = markdown::Options::gfm();
    let constructs = &mut options.parse.constructs;
    constructs.html_flow = false;
    constructs.html_text = false;
    constructs.label_start_image = false;
    options.compile.allow_dangerous_html = false;
    options.compile.allow_dangerous_protocol = false;
    options
}

fn stringify_item(
    tree: &Tree,
    references: &reference::Arena<Option<entity::Idx>>,
    item: &Item,
) -> String {
    use Item::*;

    match item {
        Text(text) | Indent(text) => text.to_string(),
        Reference(r) => {
            let path = tree.references[*r].as_str().replace('_', "\\_");
            match references[*r] {
                Some(target) if has_anchor(tree, target) => {
                    format!("[{path}](#{})", anchor(target))
                }
                // Unresolved, or not in the export: shown as typed
                _ => format!("{{{path}}}"),
            }
        }
        Newline => "\n".to_string(),
    }
}
