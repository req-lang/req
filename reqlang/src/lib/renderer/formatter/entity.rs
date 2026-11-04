use crate::{
    node::{
        entity::{Idx, TraceabilityKind},
        reference,
        tree::Tree,
    },
    renderer::formatter::{expression, markup, padding::pad},
};

pub fn format<W: std::io::Write>(
    tree: &Tree,
    idx: Idx,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::entity::Variant::*;

    let node = &tree.entities[idx];
    let pd = pad(indent);

    if let Some(comment) = &node.meta.comment {
        markup::format(tree, comment, indent, out)?;
    }

    for tag in &node.meta.tags {
        match &tag.value {
            Some(v) => writeln!(out, "{}# {} {} #", pd, tag.id, v)?,
            None => writeln!(out, "{}# {} #", pd, tag.id)?,
        }
    }

    match &node.variant {
        Part(prt) => {
            writeln!(out, "{}part {}", pd, node.meta.label)?;
            format_block(&prt.imports, tree, &prt.children, indent, out)?;
            writeln!(out, "{}part", pd)
        }

        Package(pkg) => {
            writeln!(out, "{}package {}", pd, node.meta.label)?;
            format_block(&pkg.imports, tree, &pkg.children, indent, out)?;
            writeln!(out, "{}package", pd)
        }

        Attribute(atr) => {
            write!(out, "{}let {} in ", pd, node.meta.label)?;
            expression::format(tree, atr.domain, 0.into(), out)?;
            if let Some(unit) = &atr.unit {
                write!(out, " \\ {} \\", unit)?;
            }
            writeln!(out)
        }

        Requirement(req) => {
            write!(out, "{}requirement {} ", pd, node.meta.label)?;

            if req.inhibitor.top {
                write!(out, "top ")?;
            }

            if req.inhibitor.bottom {
                write!(out, "bottom ")?;
            }

            let pd1 = pad(indent + 1);
            if let Some(constrained) = &req.restriction {
                write!(out, "\n{}on {}", pd1, tree.references[*constrained])?;
            }

            if let Some(allocated) = &req.allocation {
                write!(out, "\n{}via {}", pd1, tree.references[*allocated])?;
            }

            for trace in &req.traceability {
                let kind = traceability_keyword(&trace.kind);
                write!(out, "\n{}{} {}", pd1, kind, tree.references[trace.target])?;
            }

            writeln!(out, "\n{}is", pd)?;

            use crate::node::entity::RequirementVariant::*;
            match &req.variant {
                Formal(expr) => expression::format(tree, *expr, (indent + 1).into(), out)?,
                Informal(text) => markup::format(tree, text, (indent + 1).into(), out)?,
            }

            writeln!(out, "\n{}requirement", pd)
        }

        Type(typ) => {
            writeln!(out, "{}type {} has", pd, node.meta.label)?;
            let pd1 = pad(indent + 1);
            for (idx, member) in typ.members.iter().enumerate() {
                write!(out, "{}{}", pd1, member)?;
                if idx < typ.members.len() - 1 {
                    write!(out, " |")?;
                }
                writeln!(out, "")?;
            }
            writeln!(out, "{}type", pd)
        }
    }
}

fn traceability_keyword(kind: &TraceabilityKind) -> &'static str {
    use crate::node::entity::TraceabilityKind::*;

    match kind {
        Derivation => "derives",
        Specialization => "specializes",
        Refinement => "refines",
    }
}

fn format_block<W: std::io::Write>(
    imports: &[reference::Idx],
    tree: &Tree,
    children: &Vec<Idx>,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    for import in imports {
        writeln!(out, "{}import {}", pad(indent), tree.references[*import])?;
    }

    for child in children {
        format(tree, *child, indent + 1, out)?;
    }

    Ok(())
}
