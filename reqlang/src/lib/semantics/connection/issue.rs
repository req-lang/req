use std::{cell::RefCell, collections::HashSet};

use crate::{
    issue::Severity,
    node::{
        self,
        entity::{Arena, Idx},
    },
    semantics::{
        connection::entity::{Connection, Inbound, Neighbors, Outbound, Traceability},
        link::entity::Node,
    },
};

#[derive(Debug, Clone, thiserror::Error)]
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

type RcArena<E, M> = Arena<RefCell<Node<Neighbors<Connection<Idx>>, E, M>>>;

pub(super) fn collect_issues<E, M>(arena: &RcArena<E, M>, idx: Idx) {
    use node::entity::Variant::*;

    let nd = &mut arena[idx].borrow_mut();

    match &mut nd.variant {
        Part(_) => {
            collect_subject_inbound_issues(&mut nd.payload.inbound);
        }
        Attribute(_) => {
            collect_attr_parent_issues(arena, &mut nd.payload.parent);
            collect_subject_inbound_issues(&mut nd.payload.inbound);
        }
        Package(_) => {
            collect_pkg_parent_issues(arena, &mut nd.payload.parent);
        }
        Requirement(req) => {
            // TODO: Flag inbound restrictions and allocations

            let is_bottom = req.inhibitor.bottom;
            let is_top = req.inhibitor.top;
            let has_valid_derive = nd
                .payload
                .inbound
                .traceability
                .derivations
                .iter()
                .find(|x| is_valid_derivation(arena, *x, &nd.payload.outbound.allocation))
                .is_some();
            let usages: Vec<_> = nd
                .payload
                .outbound
                .usages
                .iter()
                .map(|p| p.target)
                .collect();

            collect_req_parent_issues(&arena, &mut nd.payload.parent);

            if let Ok(restr) = &mut nd.payload.outbound.restriction {
                collect_restriction_issues(arena, &usages, restr);
            }

            if let Some(alloc) = &mut nd.payload.outbound.allocation {
                collect_allocation_issues(arena, is_bottom, has_valid_derive, alloc);
            }

            collect_alloc_restr_compatibility_issues(&mut nd.payload.outbound);

            collect_inhibition_issues(is_top, &mut nd.payload.outbound.traceability);
            collect_inhibition_issues(is_bottom, &mut nd.payload.inbound.traceability);

            let mut specializations: HashSet<Idx> = HashSet::new();
            let mut refinements: HashSet<Idx> = HashSet::new();
            let mut derivations: HashSet<Idx> = HashSet::new();

            let trace = &mut nd.payload.outbound.traceability;

            collect_duplicated_connection(&mut specializations, &mut trace.specializations);
            collect_duplicated_connection(&mut refinements, &mut trace.refinements);
            collect_duplicated_connection(&mut derivations, &mut trace.derivations);

            let other = &[&specializations, &refinements];
            collect_conflicting_connection(other, &mut trace.derivations);

            let other = &[&derivations, &refinements];
            collect_conflicting_connection(other, &mut trace.specializations);

            let other = &[&derivations, &specializations];
            collect_conflicting_connection(other, &mut trace.refinements);

            let restr = nd.payload.outbound.restriction.clone();
            let trace = &mut nd.payload.outbound.traceability;

            for conn in &mut trace.specializations {
                if let Ok(restr) = &restr {
                    collect_specialization_issues(&restr.target, conn);
                }
                collect_traceability_target_variant_issues(arena, conn);
            }

            for conn in &mut trace.refinements {
                if let Ok(restr) = &restr {
                    collect_refinement_issues(arena, &restr.target, conn);
                }
                collect_traceability_target_variant_issues(arena, conn);
            }

            for conn in &mut trace.derivations {
                if let Ok(restr) = &restr {
                    collect_derivation_issues(arena, &usages, &restr.target, conn);
                }
                collect_traceability_target_variant_issues(arena, conn);
            }
        }
    }
}

fn collect_subject_inbound_issues(inbound: &mut Inbound<Connection<Idx>>) {
    if inbound.restrictions.is_empty() {
        inbound.issues.push(IssueKind::UnrestrictedSubject);
    }

    if inbound.usages.is_empty() {
        inbound.issues.push(IssueKind::UnusedSubject);
    }
}

fn collect_pkg_parent_issues<E, M>(arena: &RcArena<E, M>, parent: &mut Option<Connection<Idx>>) {
    use node::entity::Variant::*;

    match parent {
        Some(p) if let Package(_) = arena[p.target].borrow().variant => {}
        Some(p) => p.issue.push(IssueKind::InvalidPackageParent),
        None => {}
    }
}

fn collect_attr_parent_issues<E, M>(arena: &RcArena<E, M>, parent: &mut Option<Connection<Idx>>) {
    use node::entity::Variant::*;

    match parent {
        Some(p) if let Part(_) = arena[p.target].borrow().variant => {}
        Some(p) => p.issue.push(IssueKind::InvalidAttributeParent),
        None => {}
    }
}
//
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
fn collect_req_parent_issues<E, M>(arena: &RcArena<E, M>, parent: &mut Option<Connection<Idx>>) {
    use node::entity::Variant::*;

    match parent {
        Some(p) if let Package(_) = arena[p.target].borrow().variant => {}
        Some(p) => p.issue.push(IssueKind::InvalidRequirementParent),
        None => {}
    }
}

fn collect_alloc_restr_compatibility_issues(outbound: &mut Outbound<Connection<Idx>>) {
    let restr = &mut outbound.restriction;
    let alloc = &mut outbound.allocation;

    if let Some(alloc) = alloc
        && let Ok(restr) = restr
        && restr.target == alloc.target
    {
        alloc.issue.push(IssueKind::SameAllocationAndRestriction);
        restr.issue.push(IssueKind::SameAllocationAndRestriction);
    }
}

// TODO: Empty requirement issues
// TODO: Make this happen in the typing module
// fn collect_requirement_expr_issues<E, M>(conn: &mut Connection<NodeRef<expression::Node>>) {
//     if let Ok(TypeKind::Boolean) = conn.target.borrow().typ {
//         return;
//     }
//
//     conn.issue.push(IssueKind::NonBooleanRequirement);
// }
//
fn collect_traceability_target_variant_issues<E, M>(
    arena: &RcArena<E, M>,
    conn: &mut Connection<Idx>,
) {
    use node::entity::Variant::*;

    if let Requirement(_) = arena[conn.target].borrow().variant {
        return;
    }

    conn.issue.push(IssueKind::NotARequirement);
}

fn collect_refinement_issues<E, M>(arena: &RcArena<E, M>, restr: &Idx, conn: &mut Connection<Idx>) {
    use IssueKind::RefinementMismatch;

    let target = arena[conn.target].borrow();
    let target_restr = &target.payload.outbound.restriction;

    let mut candidate = Some(*restr);
    if let Ok(target_restr) = target_restr {
        while let Some(cnd) = candidate {
            if cnd == target_restr.target {
                return;
            }

            let cnd = arena[cnd].borrow();
            candidate = cnd.payload.parent.as_ref().map(|c| c.target);
        }

        conn.issue.push(RefinementMismatch);
    }
}

fn collect_derivation_issues<E, M>(
    arena: &RcArena<E, M>,
    usages: &Vec<Idx>,
    restr: &Idx,
    conn: &mut Connection<Idx>,
) {
    use IssueKind::{DerivationMismatch, NoDerivedAllocation, UnrelatedDerivation};

    let target = arena[conn.target].borrow();
    let target_alloc = &target.payload.outbound.allocation;
    match target_alloc {
        Some(al) if *restr != al.target => conn.issue.push(DerivationMismatch),
        None => conn.issue.push(NoDerivedAllocation),
        _ => {}
    }

    let out_restr_usage = usages.iter().find(|u| **u == *restr);
    if out_restr_usage.is_none() {
        conn.issue.push(UnrelatedDerivation);
    }
}

fn collect_specialization_issues(restr: &Idx, conn: &mut Connection<Idx>) {
    use IssueKind::SpecializationMismatch;

    if *restr != conn.target {
        conn.issue.push(SpecializationMismatch);
    }
}

fn collect_restriction_issues<E, M>(
    arena: &RcArena<E, M>,
    usages: &Vec<Idx>,
    restr: &mut Connection<Idx>,
) {
    use IssueKind::{NotAPartOrAttribute, UnrelatedRestriction};
    use node::entity::Variant::*;

    match arena[restr.target].borrow().variant {
        Part(_) | Attribute(_) => {}
        _ => restr.issue.push(NotAPartOrAttribute),
    }

    let restr_usage = usages.iter().find(|u| restr.target == **u);
    if restr_usage.is_none() {
        restr.issue.push(UnrelatedRestriction);
    }
}

fn collect_allocation_issues<E, M>(
    arena: &RcArena<E, M>,
    is_bottom: bool,
    has_valid_derive: bool,
    alloc: &mut Connection<Idx>,
) {
    use IssueKind::{InhibitorForbidDerives, MissingDerivation, NotAPartOrAttribute};
    use node::entity::Variant::*;

    match arena[alloc.target].borrow().variant {
        Part(_) | Attribute(_) => {}
        _ => alloc.issue.push(NotAPartOrAttribute),
    }

    if is_bottom {
        alloc.issue.push(InhibitorForbidDerives)
    }

    if !has_valid_derive {
        alloc.issue.push(MissingDerivation)
    }
}

fn collect_inhibition_issues(is_inhibited: bool, trace: &mut Traceability<Connection<Idx>>) {
    use IssueKind::{ForbiddenTrace, MissingTrace};

    let has_derivation = !trace.derivations.is_empty();
    let has_refinement = !trace.refinements.is_empty();
    let issue = match is_inhibited {
        true if has_derivation || has_refinement => Some(ForbiddenTrace),
        false if !(has_derivation || has_refinement) => Some(MissingTrace),
        _ => None,
    };
    trace.issues.extend(issue.into_iter());
}

fn collect_duplicated_connection(seen: &mut HashSet<Idx>, connections: &mut Vec<Connection<Idx>>) {
    use IssueKind::DuplicatedTrace;

    for conn in connections {
        let ptr = conn.target;
        if !seen.insert(ptr) {
            conn.issue.push(DuplicatedTrace);
        }
    }
}

fn collect_conflicting_connection(other: &[&HashSet<Idx>], connections: &mut Vec<Connection<Idx>>) {
    use IssueKind::ConflictingTrace;

    for conn in connections {
        let ptr = conn.target;
        if other.iter().find(|s| s.get(&ptr).is_some()).is_some() {
            conn.issue.push(ConflictingTrace);
        }
    }
}

fn is_valid_derivation<E, M>(
    arena: &RcArena<E, M>,
    conn: &Connection<Idx>,
    alloc: &Option<Connection<Idx>>,
) -> bool {
    if let Some(alloc) = alloc
        && let Ok(restr) = &arena[conn.target].borrow().payload.outbound.restriction
        && restr.target == alloc.target
    {
        return true;
    }

    return false;
}
