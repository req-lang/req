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

    let pd = pad(indent);

    if let Some(comment) = &node.variant.meta().comment {
        markup::format(comment, indent, out)?;
    }

    for tag in &node.variant.meta().tags {
        match &tag.value {
            Some(v) => writeln!(out, "{}# {} {} #", pd, tag.id, v)?,
            None => writeln!(out, "{}# {} #", pd, tag.id)?,
        }
    }

    match &node.variant {
        Part(prt) => {
            writeln!(out, "{}part {}", pd, prt.meta.label)?;
            format_container(&prt.imports, &prt.children, indent, out)?;
            writeln!(out, "{}part", pd)
        }

        Package(pkg) => {
            writeln!(out, "{}package {}", pd, pkg.meta.label)?;
            format_container(&pkg.imports, &pkg.children, indent, out)?;
            writeln!(out, "{}package", pd)
        }

        Attribute(atr) => {
            write!(out, "{}let {} in ", pd, atr.meta.label)?;
            expression::format(atr.domain.borrow(), 0.into(), out)?;
            if let Some(unit) = &atr.unit {
                write!(out, " [ ")?;
                expression::format(unit.borrow(), 0.into(), out)?;
                write!(out, " ]")?;
            }
            writeln!(out)
        }

        Requirement(req) => {
            write!(out, "{}requirement {} ", pd, node.variant.meta().label)?;

            if req.inhibitor.top {
                write!(out, "top ")?;
            }

            if req.inhibitor.bottom {
                write!(out, "bottom ")?;
            }

            let pd1 = pad(indent + 1);
            if let Some(constrained) = &req.restriction {
                write!(out, "\n{}on {}", pd1, constrained)?;
            }

            if let Some(allocated) = &req.allocation {
                write!(out, "\n{}via {}", pd1, allocated)?;
            }

            for trace in &req.traceability {
                let kind = traceability_keyword(&trace.kind);
                write!(out, "\n{}{} {}", pd1, kind, trace.target)?;
            }

            writeln!(out, "\n{}is", pd)?;

            use crate::node::entity::RequirementVariant::*;
            match &req.variant {
                Formal(expr) => expression::format(expr.borrow(), (indent + 1).into(), out)?,
                Informal(text) => markup::format(text, (indent + 1).into(), out)?,
            }

            writeln!(out, "\n{}requirement", pd)
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
