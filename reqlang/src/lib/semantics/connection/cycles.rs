use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::{
    node::entity::TraceabilityKind,
    node::{self, NodeRef},
    semantics::connection::{entity::Node, issue::IssueKind},
    utils::iter::NodeIter,
};

#[derive(Clone, Default)]
struct PathCtx {
    in_path: HashSet<*const RefCell<Node>>,
    path: Vec<(NodeRef<Node>, TraceabilityKind)>,
}

pub(super) fn detect_traceability_cycles(node: NodeRef<Node>) {
    use node::entity::Variant::Requirement;

    let mut visited: HashSet<*const RefCell<Node>> = HashSet::new();
    for child in NodeIter::new(node) {
        let ptr = Rc::as_ptr(&child);
        if let Requirement(_) = &child.borrow().variant
            && !visited.contains(&ptr)
        {
            let neighbors = traceability_neighbors(&child);
            for (neighbor, kind) in neighbors {
                let ctx = PathCtx {
                    in_path: HashSet::from([ptr]),
                    path: vec![(child.clone(), kind)],
                };
                detect_cycle_from(neighbor, ctx, &mut visited);
            }
            visited.insert(ptr);
        }
    }
}

fn detect_cycle_from(
    node: NodeRef<Node>,
    ctx: PathCtx,
    visited: &mut HashSet<*const RefCell<Node>>,
) {
    let ptr = Rc::as_ptr(&node);

    if ctx.in_path.contains(&ptr) {
        let pos = ctx
            .path
            .iter()
            .position(|(n, _)| Rc::as_ptr(n) == ptr)
            .unwrap();
        mark_cycle(&ctx.path[pos..]);
        return;
    }

    if visited.contains(&ptr) {
        return;
    }

    let neighbors = traceability_neighbors(&node);
    let mut ctx = ctx;
    ctx.in_path.insert(ptr);

    for (neighbor, kind) in neighbors {
        let mut next_ctx = ctx.clone();
        next_ctx.path.push((node.clone(), kind));
        detect_cycle_from(neighbor, next_ctx, visited);
    }

    visited.insert(ptr);
}

fn traceability_neighbors(node: &NodeRef<Node>) -> Vec<(NodeRef<Node>, TraceabilityKind)> {
    use TraceabilityKind::*;

    let nd = node.borrow();
    let ib = &nd.inbound.traceability;

    let derivations = ib
        .derivations
        .iter()
        .map(|c| (c.target.upgrade().unwrap(), Derivation));

    let refinements = ib
        .refinements
        .iter()
        .map(|c| (c.target.upgrade().unwrap(), Refinement));

    let specializations = ib
        .specializations
        .iter()
        .map(|c| (c.target.upgrade().unwrap(), Specialization));

    derivations
        .chain(refinements)
        .chain(specializations)
        .collect()
}

fn mark_cycle(path: &[(NodeRef<Node>, TraceabilityKind)]) {
    use TraceabilityKind::*;

    for i in 0..path.len() {
        let (node, kind) = &path[i];
        let next_ptr = Rc::as_ptr(&path[(i + 1) % path.len()].0);

        let mut nd = node.borrow_mut();
        let conns = match kind {
            Derivation => &mut nd.inbound.traceability.derivations,
            Refinement => &mut nd.inbound.traceability.refinements,
            Specialization => &mut nd.inbound.traceability.specializations,
        };
        if let Some(conn) = conns.iter_mut().find(|c| c.target.as_ptr() == next_ptr) {
            conn.issue.push(IssueKind::CyclicTrace);
        }
    }
}
