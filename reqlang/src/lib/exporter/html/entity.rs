use crate::{
    Specification,
    exporter::html::{escape, expression, markup},
    node::{
        entity::{Idx, Node, RequirementVariant, Variant},
        tree::Tree,
    },
};

/// `level` is 1 for the root, the package of the file. A package or a part is a heading, followed by its
/// description and by the requirements and attributes it contains, one level deeper.
pub fn render<W: std::io::Write>(
    spec: &Specification,
    idx: Idx,
    level: usize,
    out: &mut W,
) -> std::io::Result<()> {
    let tree = &spec.tree;
    let node = &tree.entities[idx];

    let children = match &node.variant {
        Variant::Package(package) => &package.children,
        Variant::Part(part) => &part.children,
        _ => return Ok(()),
    };

    render_heading(node, idx, level, out)?;
    render_description(spec, node, out)?;

    for child in children {
        let node = &tree.entities[*child];
        match &node.variant {
            Variant::Requirement(requirement) => {
                render_heading(node, *child, level + 1, out)?;
                match &requirement.variant {
                    RequirementVariant::Formal(expression) => {
                        expression::render(tree, *expression, out)?
                    }
                    RequirementVariant::Informal(text) => markup::render(spec, *text, out)?,
                }
                if let Some(comment) = node.meta.comment {
                    writeln!(out, "<aside class=\"comment\">")?;
                    markup::render(spec, comment, out)?;
                    writeln!(out, "</aside>")?;
                }
            }
            Variant::Attribute(_) => {
                render_heading(node, *child, level + 1, out)?;
                render_description(spec, node, out)?;
            }
            _ => {}
        }
    }

    Ok(())
}

/// The id a link to the entity points at. The prefix keeps it apart from the ids of the page it is put in.
pub(super) fn anchor(idx: Idx) -> String {
    format!("pv-{}", usize::from(idx))
}

/// Whether the export has a heading, so an anchor, for the entity (the types have none)
pub(super) fn has_anchor(tree: &Tree, idx: Idx) -> bool {
    !matches!(tree.entities[idx].variant, Variant::Type(_))
}

fn render_heading<W: std::io::Write>(
    node: &Node,
    idx: Idx,
    level: usize,
    out: &mut W,
) -> std::io::Result<()> {
    // The page has its own title: the headings start at h2
    let tag = format!("h{}", (level + 1).min(6));
    let label = escape(&node.meta.label.replace('_', " "));
    writeln!(out, "<{tag} id=\"{}\">{label}</{tag}>", anchor(idx))
}

fn render_description<W: std::io::Write>(
    spec: &Specification,
    node: &Node,
    out: &mut W,
) -> std::io::Result<()> {
    match node.meta.comment {
        Some(comment) => markup::render(spec, comment, out),
        None => Ok(()),
    }
}
