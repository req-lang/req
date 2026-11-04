use std::collections::HashSet;

use crate::{
    issue::Severity,
    node::{
        self,
        entity::{Arena, Idx, Node, TraceabilityKind},
    },
    semantics::connection::entity::{Connection, Inbound, Neighbors, Traceability},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IssueKind {
    #[error("Requirement is missing a constraint")]
    MissingRequirementBinding,
    #[error("Part or attribute expected")]
    NotAPartOrAttribute,
    #[error("Requirement expected")]
    NotARequirement,
    #[error("The requirement is allocated but the inhibitor forbids derivation")]
    InhibitorForbidDerives,
    #[error("Allocation of an entity without a deriving requirement")]
    MissingDerivation,
    #[error("Unexpected traceability")]
    ForbiddenTrace,
    #[error("Missing traceability")]
    MissingTrace,
    #[error("Traceability link is duplicated")]
    DuplicatedTrace,
    #[error("Traceability link is in conflict")]
    ConflictingTrace,
    #[error("Specialization with different restricted subjects")]
    SpecializationMismatch,
    #[error("Refinement is not restricting a children of the refined restriction")]
    RefinementMismatch,
    #[error("Derived requirement is not allocated")]
    NoDerivedAllocation,
    #[error("Derived requirement is not allocated to the restriction of this requirement")]
    DerivationMismatch,
    #[error("Derived requirement is not using its parent restriction")]
    UnrelatedDerivation,
    #[error("Requirement does not uses its restriction in its expression")]
    UnrelatedRestriction,
    #[error("Traceability link is part of a cycle")]
    CyclicTrace,
    #[error("Allocation and restriction should be on different subjects")]
    SameAllocationAndRestriction,
    #[error("Requirements are expected to be contained in packages")]
    InvalidRequirementParent,
    #[error("Package are expected to be contained in a packages")]
    InvalidPackageParent,
    #[error("Attributes are expected to be contained in part")]
    InvalidAttributeParent,
    #[error("This entity is not the restricted by any requirement")]
    UnrestrictedSubject,
    #[error("This entity is not the used by any requirement")]
    UnusedSubject,
}

impl From<&IssueKind> for Severity {
    fn from(kind: &IssueKind) -> Self {
        use IssueKind::*;
        use Severity::*;

        match kind {
            InvalidRequirementParent => Critical,
            InvalidPackageParent => Critical,
            InvalidAttributeParent => Critical,
            NotARequirement => Critical,
            InhibitorForbidDerives => Critical,
            NotAPartOrAttribute => Critical,

            MissingRequirementBinding => Severe,
            ConflictingTrace => Severe,
            CyclicTrace => Severe,
            SpecializationMismatch => Severe,
            RefinementMismatch => Severe,
            DerivationMismatch => Severe,

            UnrelatedRestriction => Moderate,
            UnrelatedDerivation => Moderate,
            SameAllocationAndRestriction => Moderate,
            ForbiddenTrace => Moderate,
            DuplicatedTrace => Moderate,
            NoDerivedAllocation => Moderate,
            UnrestrictedSubject => Moderate,

            UnusedSubject => Light,
            MissingDerivation => Light,
            MissingTrace => Light,
        }
    }
}

pub(super) fn collect(
    entities: &Arena<Node>,
    neighbors: &mut Arena<Neighbors>,
    cycles: &HashSet<Idx>,
    idx: Idx,
) {
    let mut nd = std::mem::take(&mut neighbors[idx]);
    collect_inner(entities, neighbors, cycles, idx, &mut nd);
    neighbors[idx] = nd;
}

pub(super) fn collect_inner(
    entities: &Arena<Node>,
    graph: &Arena<Neighbors>,
    cycles: &HashSet<Idx>,
    idx: Idx,
    nd: &mut Neighbors,
) {
    use node::entity::Variant::*;

    match &entities[idx].variant {
        Part(_) => collect_subject_issues(&mut nd.inbound),
        Attribute(_) => {
            collect_parent_issues(
                entities,
                &mut nd.parent,
                |parent| matches!(parent, Part(_)),
                IssueKind::InvalidAttributeParent,
            );
            collect_subject_issues(&mut nd.inbound);
        }
        Package(_) => collect_parent_issues(
            entities,
            &mut nd.parent,
            |parent| matches!(parent, Package(_)),
            IssueKind::InvalidPackageParent,
        ),
        Type(_) => {}
        Requirement(req) => {
            // TODO: Flag inbound restrictions and allocations

            let is_bottom = req.inhibitor.bottom;
            let restriction = nd.outbound.restriction.as_ref().map(|c| c.target);
            let allocation = nd.outbound.allocation.as_ref().map(|c| c.target);
            let usages: Vec<_> = nd.outbound.usages.iter().map(|u| u.target).collect();

            collect_parent_issues(
                entities,
                &mut nd.parent,
                |parent| matches!(parent, Package(_)),
                IssueKind::InvalidRequirementParent,
            );

            if cycles.contains(&idx) {
                nd.inbound.traceability.issues.push(IssueKind::CyclicTrace);
            }
            if let Some(kind) = inhibition_issue(is_bottom, &nd.inbound.traceability) {
                nd.inbound.traceability.issues.push(kind);
            }
            if let Some(kind) = inhibition_issue(req.inhibitor.top, &nd.outbound.traceability) {
                nd.outbound.traceability.issues.push(kind);
            }

            let has_valid_derive = nd
                .inbound
                .traceability
                .derivations
                .iter()
                .any(|derivation| is_valid_derivation(graph, derivation, allocation));

            if let Some(alloc) = &mut nd.outbound.allocation {
                collect_allocation_issues(entities, is_bottom, has_valid_derive, alloc);
            }

            match &mut nd.outbound.restriction {
                Some(restr) => collect_restriction_issues(entities, &usages, restr),
                None => nd
                    .outbound
                    .issues
                    .push(IssueKind::MissingRequirementBinding),
            }

            if let (Some(restr), Some(alloc)) = (restriction, allocation)
                && restr == alloc
            {
                let kind = IssueKind::SameAllocationAndRestriction;
                for conn in nd
                    .outbound
                    .restriction
                    .iter_mut()
                    .chain(&mut nd.outbound.allocation)
                {
                    conn.issues.push(kind.clone());
                }
            }

            collect_traceability_issues(
                entities,
                graph,
                &mut nd.outbound.traceability,
                restriction,
                &usages,
            );
        }
    }
}

fn collect_parent_issues(
    entities: &Arena<Node>,
    parent: &mut Option<Connection>,
    is_valid: impl Fn(&node::entity::Variant) -> bool,
    kind: IssueKind,
) {
    if let Some(parent) = parent
        && !is_valid(&entities[parent.target].variant)
    {
        parent.issues.push(kind);
    }
}

fn collect_subject_issues(inbound: &mut Inbound) {
    if inbound.restrictions.is_empty() {
        inbound.issues.push(IssueKind::UnrestrictedSubject);
    }

    if inbound.usages.is_empty() {
        inbound.issues.push(IssueKind::UnusedSubject);
    }
}

fn collect_restriction_issues(entities: &Arena<Node>, usages: &[Idx], restr: &mut Connection) {
    use IssueKind::{NotAPartOrAttribute, UnrelatedRestriction};
    use node::entity::Variant::*;

    match entities[restr.target].variant {
        Part(_) | Attribute(_) => {}
        _ => restr.issues.push(NotAPartOrAttribute),
    }

    if !usages.contains(&restr.target) {
        restr.issues.push(UnrelatedRestriction);
    }
}

fn collect_allocation_issues(
    entities: &Arena<Node>,
    is_bottom: bool,
    has_valid_derive: bool,
    alloc: &mut Connection,
) {
    use IssueKind::{InhibitorForbidDerives, MissingDerivation, NotAPartOrAttribute};
    use node::entity::Variant::*;

    match entities[alloc.target].variant {
        Part(_) | Attribute(_) => {}
        _ => alloc.issues.push(NotAPartOrAttribute),
    }

    if is_bottom {
        alloc.issues.push(InhibitorForbidDerives);
    }

    if !has_valid_derive {
        alloc.issues.push(MissingDerivation);
    }
}

/// Flags the traceability links of a requirement, specializations first and derivations last
fn collect_traceability_issues(
    entities: &Arena<Node>,
    graph: &Arena<Neighbors>,
    trace: &mut Traceability,
    restriction: Option<Idx>,
    usages: &[Idx],
) {
    use IssueKind::*;
    use TraceabilityKind::*;
    use node::entity::Variant::Requirement;

    let kinds = [Specialization, Refinement, Derivation];
    let targets = kinds.map(|kind| trace[kind].iter().map(|c| c.target).collect::<Vec<_>>());

    for (pos, kind) in kinds.into_iter().enumerate() {
        let mut seen: HashSet<Idx> = HashSet::new();

        for conn in &mut trace[kind] {
            if !seen.insert(conn.target) {
                conn.issues.push(DuplicatedTrace);
            }

            let is_conflicting = targets
                .iter()
                .enumerate()
                .any(|(other, t)| other != pos && t.contains(&conn.target));
            if is_conflicting {
                conn.issues.push(ConflictingTrace);
            }

            if let Some(restr) = restriction {
                match kind {
                    Specialization if restr != conn.target => {
                        conn.issues.push(SpecializationMismatch);
                    }
                    Refinement if is_refinement_mismatch(graph, restr, conn.target) => {
                        conn.issues.push(RefinementMismatch);
                    }
                    Derivation => collect_derivation_issues(graph, usages, restr, conn),
                    _ => {}
                }
            }

            if !matches!(entities[conn.target].variant, Requirement(_)) {
                conn.issues.push(NotARequirement);
            }
        }
    }
}

/// The refined requirement does not restrict an ancestor of what this one restricts
fn is_refinement_mismatch(graph: &Arena<Neighbors>, restr: Idx, target: Idx) -> bool {
    let Some(target_restr) = &graph[target].outbound.restriction else {
        return false;
    };

    let mut candidate = Some(restr);
    while let Some(cnd) = candidate {
        if cnd == target_restr.target {
            return false;
        }

        candidate = graph[cnd].parent.as_ref().map(|parent| parent.target);
    }

    true
}

fn collect_derivation_issues(
    graph: &Arena<Neighbors>,
    usages: &[Idx],
    restr: Idx,
    conn: &mut Connection,
) {
    use IssueKind::{DerivationMismatch, NoDerivedAllocation, UnrelatedDerivation};

    match &graph[conn.target].outbound.allocation {
        Some(al) if restr != al.target => conn.issues.push(DerivationMismatch),
        None => conn.issues.push(NoDerivedAllocation),
        _ => {}
    }

    if !usages.contains(&restr) {
        conn.issues.push(UnrelatedDerivation);
    }
}

fn inhibition_issue(is_inhibited: bool, trace: &Traceability) -> Option<IssueKind> {
    use IssueKind::{ForbiddenTrace, MissingTrace};

    let has_derivation = !trace.derivations.is_empty();
    let has_refinement = !trace.refinements.is_empty();
    match is_inhibited {
        true if has_derivation || has_refinement => Some(ForbiddenTrace),
        false if !(has_derivation || has_refinement) => Some(MissingTrace),
        _ => None,
    }
}

fn is_valid_derivation(graph: &Arena<Neighbors>, conn: &Connection, alloc: Option<Idx>) -> bool {
    alloc.is_some_and(|alloc| {
        graph[conn.target]
            .outbound
            .restriction
            .as_ref()
            .is_some_and(|restr| restr.target == alloc)
    })
}

// TODO: Make this happen in the typing section
// fn collect_attr_type_issues(
//     attr: &mut Attribute<Connection<NodeRef<expression::Node>>, markup::Node>,
// ) {
//     use IssueKind::{UnexpectedUnit, UntypedAttribute};
//
//     match &attr.domain.target.borrow().typ {
//         Ok(TypeKind::Set(typ)) => {
//             let mut inner = *typ.clone();
//             loop {
//                 match inner {
//                     TypeKind::Set(typ) => inner = *typ,
//                     typ => {
//                         inner = typ;
//                         break;
//                     }
//                 }
//             }
//
//             match inner {
//                 TypeKind::Number => {}
//                 TypeKind::Undefined => attr.domain.issue.push(UntypedAttribute),
//                 _ => match &mut attr.unit {
//                     Some(unit) => unit.issue.push(UnexpectedUnit),
//                     _ => {}
//                 },
//             }
//         }
//         Ok(TypeKind::Undefined) => attr.domain.issue.push(UntypedAttribute),
//         _ => {}
//     }
// }
//
// TODO: Empty requirement issues
// TODO: Make this happen in the typing module
// fn collect_requirement_expr_issues(conn: &mut Connection<NodeRef<expression::Node>>) {
//     if let Ok(TypeKind::Boolean) = conn.target.borrow().typ {
//         return;
//     }
//
//     conn.issue.push(IssueKind::NonBooleanRequirement);
// }
//
