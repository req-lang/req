use std::{cell::RefCell, collections::HashSet};

use crate::{
    node::{
        self,
        entity::{Arena, Idx, TraceabilityKind},
    },
    semantics::{
        connection::{
            entity::{Connection, Neighbors},
            issue::IssueKind,
        },
        link::entity::Node,
    },
};

#[derive(Clone, Default)]
struct PathCtx {
    in_path: HashSet<Idx>,
    path: Vec<(Idx, TraceabilityKind)>,
}

pub(super) fn detect_traceability_cycles<E, M>(
    arena: &Arena<RefCell<Node<Neighbors<Connection<Idx>>, E, M>>>,
) {
    use node::entity::Variant::Requirement;

    let mut visited: HashSet<Idx> = HashSet::new();
    arena.for_each(|node, idx| {
        let idx = Idx::from(idx);
        let node = node.borrow();
        if let Requirement(_) = &node.variant
            && !visited.contains(&idx)
        {
            let neighbors = node.payload.inbound.traceability.iter();
            for (neighbor, kind) in neighbors {
                let ctx = PathCtx {
                    in_path: HashSet::from([idx]),
                    path: vec![(idx, kind)],
                };
                detect_cycle_from(arena, neighbor.target, ctx, &mut visited);
            }
            visited.insert(idx);
        }
    });
}

fn detect_cycle_from<E, M>(
    arena: &Arena<RefCell<Node<Neighbors<Connection<Idx>>, E, M>>>,
    idx: Idx,
    ctx: PathCtx,
    visited: &mut HashSet<Idx>,
) {
    let idx = idx;

    if ctx.in_path.contains(&idx) {
        let pos = ctx.path.iter().position(|(n, _)| *n == idx).unwrap();
        mark_cycle(arena, &ctx.path[pos..]);
        return;
    }

    if visited.contains(&idx) {
        return;
    }

    let node = &arena[idx].borrow();
    let neighbors = node.payload.inbound.traceability.iter();
    let mut ctx = ctx;
    ctx.in_path.insert(idx);

    for (neighbor, kind) in neighbors
        .map(|(t, k)| (t.target, k))
        .collect::<Vec<_>>()
        .into_iter()
    {
        let mut next_ctx = ctx.clone();
        next_ctx.path.push((idx, kind));
        detect_cycle_from(arena, neighbor, next_ctx, visited);
    }

    visited.insert(idx);
}

fn mark_cycle<E, M>(
    arena: &Arena<RefCell<Node<Neighbors<Connection<Idx>>, E, M>>>,
    path: &[(Idx, TraceabilityKind)],
) {
    use TraceabilityKind::*;

    for (i, kind) in path {
        let nd = &mut arena[*i].borrow_mut();
        let (idx, _) = path[(usize::from(*i) + 1) % path.len()];
        let conns = match kind {
            Derivation => &mut nd.payload.inbound.traceability.derivations,
            Refinement => &mut nd.payload.inbound.traceability.refinements,
            Specialization => &mut nd.payload.inbound.traceability.specializations,
        };
        if let Some(conn) = conns.iter_mut().find(|c| c.target == idx) {
            conn.issue.push(IssueKind::CyclicTrace);
        }
    }
}
