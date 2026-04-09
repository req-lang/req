use std::cell::Ref;

use crate::{
    node::{NodeRef, entity::TraceabilityKind},
    reference::Reference,
    renderer::formatter::{expression, markup, padding::pad},
    syntax::entity::Node,
};

pub fn format<W: std::io::Write>(
    node: Ref<Node>,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::entity::Variant::*;

    let pad = pad(indent);

    if let Some(comment) = &node.variant.meta().comment {
        markup::format(comment, indent, out)?;
    }

    for tag in &node.variant.meta().tags {
        match &tag.value {
            Some(v) => writeln!(out, "{}# {} {} #", pad, tag.id, v)?,
            None => writeln!(out, "{}# {} #", pad, tag.id)?,
        }
    }

    match &node.variant {
        Part(prt) => {
            writeln!(out, "{}part {}", pad, prt.meta.label)?;
            format_container(&prt.imports, &prt.children, indent, out)?;
            writeln!(out, "{}part", pad)
        }

        Package(pkg) => {
            writeln!(out, "{}package {}", pad, pkg.meta.label)?;
            format_container(&pkg.imports, &pkg.children, indent, out)?;
            writeln!(out, "{}package", pad)
        }

        Attribute(atr) => {
            write!(out, "{}let {} in ", pad, atr.meta.label)?;
            expression::format(atr.domain.borrow(), 0.into(), out)?;
            if let Some(unit) = &atr.unit {
                write!(out, " [ ")?;
                expression::format(unit.borrow(), 0.into(), out)?;
                write!(out, " ]")?;
            }
            writeln!(out)
        }

        Requirement(req) => {
            write!(out, "{}requirement {} ", pad, node.variant.meta().label)?;

            if req.inhibitor.top {
                write!(out, "top ")?;
            }

            if req.inhibitor.bottom {
                write!(out, "bottom ")?;
            }

            if let Some(constrained) = &req.restriction {
                writeln!(out, "{}on {}", pad, constrained)?;
            }

            if let Some(allocated) = &req.allocation {
                writeln!(out, "{}via {}", pad, allocated)?;
            }

            for trace in &req.traceability {
                let kind = traceability_keyword(&trace.kind);
                writeln!(out, "{}{} {}", pad, kind, trace.target)?;
            }

            writeln!(out, "{}is", pad)?;

            use crate::node::entity::RequirementVariant::*;
            match &req.variant {
                Formal(expr) => expression::format(expr.borrow(), (indent + 1).into(), out)?,
                Informal(text) => markup::format(text, (indent + 1).into(), out)?,
            }

            writeln!(out, "{}requirement", pad)
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

fn format_container<W: std::io::Write>(
    imports: &[Reference],
    children: &[NodeRef<Node>],
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    for import in imports {
        writeln!(out, "{}import {}", pad(indent), import.value)?;
    }

    for child in children {
        format(child.borrow(), indent + 1, out)?;
    }

    Ok(())
}
