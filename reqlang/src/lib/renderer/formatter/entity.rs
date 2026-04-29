use std::fmt::Display;

use crate::{
    node::entity::{Idx, Node, TraceabilityKind},
    renderer::formatter::{expression, markup, padding::pad},
};

pub fn format<W: std::io::Write, N, R: Display, T: Display, E, M>(
    arena: &Vec<Node<N, R, T, E, M>>,
    idx: Idx,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::entity::Variant::*;

    let node = &arena[idx];
    let pd = pad(indent);

    if let Some(comment) = &node.meta.comment {
        markup::format(comment, indent, out)?;
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
            format_container(&prt.imports, arena, &prt.children, indent, out)?;
            writeln!(out, "{}part", pd)
        }

        Package(pkg) => {
            writeln!(out, "{}package {}", pd, node.meta.label)?;
            format_container(&pkg.imports, arena, &pkg.children, indent, out)?;
            writeln!(out, "{}package", pd)
        }

        Attribute(atr) => {
            write!(out, "{}let {} in ", pd, node.meta.label)?;
            expression::format(&atr.domain, 0.into(), 0.into(), out)?;
            if let Some(unit) = &atr.unit {
                write!(out, " [ ")?;
                write!(out, "{}", unit)?;
                write!(out, " ]")?;
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
                Formal(expr) => expression::format(expr, 0.into(), (indent + 1).into(), out)?,
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

fn format_container<W: std::io::Write, N, R: Display, T: Display, E, M>(
    imports: &[R],
    arena: &Vec<Node<N, R, T, E, M>>,
    children: &Vec<Idx>,
    indent: usize,
    out: &mut W,
) -> std::io::Result<()> {
    for import in imports {
        writeln!(out, "{}import {}", pad(indent), import)?;
    }

    for child in children {
        format(arena, *child, indent + 1, out)?;
    }

    Ok(())
}
