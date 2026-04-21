use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::{
    issue::Severity,
    node::{self, NodeRef},
    semantics::{
        connection::entity::{Connection, Inbound, Node, Outbound, Traceability},
        link::WeakNodeRef,
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

pub(super) fn collect_issues(node: NodeRef<Node>) {
    use node::entity::Variant::*;

    let mut nd = node.borrow_mut();

    match &mut nd.variant {
        Part(_) => {
            collect_subject_inbound_issues(&mut nd.neighbors.inbound);
        }
        Attribute(_) => {
            collect_attr_parent_issues(&mut nd.neighbors.parent);
            collect_subject_inbound_issues(&mut nd.neighbors.inbound);
        }
        Package(_) => {
            collect_pkg_parent_issues(&mut nd.neighbors.parent);
        }
        Requirement(req) => {
            // TODO: Flag inbound restrictions and allocations

            let is_bottom = req.inhibitor.bottom;
            let is_top = req.inhibitor.top;
            let has_valid_derive = nd
                .neighbors
                .inbound
                .traceability
                .derivations
                .iter()
                .find(|x| is_valid_derivation(*x, &nd.neighbors.outbound.allocation))
                .is_some();
            let usages: Vec<_> = nd
                .neighbors
                .outbound
                .usages
                .iter()
                .map(|p| p.target.as_ptr())
                .collect();

            collect_req_parent_issues(&mut nd.neighbors.parent);

            if let Ok(restr) = &mut nd.neighbors.outbound.restriction {
                collect_restriction_issues(&usages, restr);
            }

            if let Some(alloc) = &mut nd.neighbors.outbound.allocation {
                collect_allocation_issues(is_bottom, has_valid_derive, alloc);
            }

            collect_alloc_restr_compatibility_issues(&mut nd.neighbors.outbound);

            collect_inhibition_issues(is_top, &mut nd.neighbors.outbound.traceability);
            collect_inhibition_issues(is_bottom, &mut nd.neighbors.inbound.traceability);

            let mut specializations: HashSet<*const RefCell<Node>> = HashSet::new();
            let mut refinements: HashSet<*const RefCell<Node>> = HashSet::new();
            let mut derivations: HashSet<*const RefCell<Node>> = HashSet::new();

            let trace = &mut nd.neighbors.outbound.traceability;

            collect_duplicated_connection(&mut specializations, &mut trace.specializations);
            collect_duplicated_connection(&mut refinements, &mut trace.refinements);
            collect_duplicated_connection(&mut derivations, &mut trace.derivations);

            let other = &[&specializations, &refinements];
            collect_conflicting_connection(other, &mut trace.derivations);

            let other = &[&derivations, &refinements];
            collect_conflicting_connection(other, &mut trace.specializations);

            let other = &[&derivations, &specializations];
            collect_conflicting_connection(other, &mut trace.refinements);

            let restr = nd.neighbors.outbound.restriction.clone();
            let trace = &mut nd.neighbors.outbound.traceability;

            for conn in &mut trace.specializations {
                if let Ok(restr) = &restr {
                    collect_specialization_issues(&restr.target, conn);
                }
                collect_traceability_target_variant_issues(conn);
            }

            for conn in &mut trace.refinements {
                if let Ok(restr) = &restr {
                    collect_refinement_issues(&restr.target, conn);
                }
                collect_traceability_target_variant_issues(conn);
            }

            for conn in &mut trace.derivations {
                if let Ok(restr) = &restr {
                    collect_derivation_issues(&usages, &restr.target, conn);
                }
                collect_traceability_target_variant_issues(conn);
            }
        }
    }
}

fn collect_subject_inbound_issues(inbound: &mut Inbound<Connection<WeakNodeRef<Node>>>) {
    if inbound.restrictions.is_empty() {
        inbound.issues.push(IssueKind::UnrestrictedSubject);
    }

    if inbound.usages.is_empty() {
        inbound.issues.push(IssueKind::UnusedSubject);
    }
}

fn collect_pkg_parent_issues(parent: &mut Option<Connection<WeakNodeRef<Node>>>) {
    use node::entity::Variant::*;

    if let Some(p) = parent {
        let parent = p.target.upgrade().unwrap();
        match parent.borrow().variant {
            Package(_) => return,
            _ => p.issue.push(IssueKind::InvalidPackageParent),
        }
    }
}

fn collect_attr_parent_issues(parent: &mut Option<Connection<WeakNodeRef<Node>>>) {
    use node::entity::Variant::*;

    if let Some(p) = parent {
        let parent = p.target.upgrade().unwrap();
        match parent.borrow().variant {
            Part(_) => return,
            _ => p.issue.push(IssueKind::InvalidAttributeParent),
        }
    }
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
fn collect_req_parent_issues(parent: &mut Option<Connection<WeakNodeRef<Node>>>) {
    use node::entity::Variant::*;

    if let Some(p) = parent {
        let parent = p.target.upgrade().unwrap();
        match parent.borrow().variant {
            Package(_) => return,
            _ => p.issue.push(IssueKind::InvalidRequirementParent),
        }
    }
}

fn collect_alloc_restr_compatibility_issues(
    outbound: &mut Outbound<Connection<WeakNodeRef<Node>>>,
) {
    let restr = &mut outbound.restriction;
    let alloc = &mut outbound.allocation;

    if let Some(alloc) = alloc
        && let Ok(restr) = restr
        && restr.target.as_ptr() == alloc.target.as_ptr()
    {
        alloc.issue.push(IssueKind::SameAllocationAndRestriction);
        restr.issue.push(IssueKind::SameAllocationAndRestriction);
    }
}

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
fn collect_traceability_target_variant_issues(conn: &mut Connection<WeakNodeRef<Node>>) {
    use node::entity::Variant::*;

    let tgt = conn.target.upgrade().unwrap();
    if let Requirement(_) = tgt.borrow().variant {
        return;
    }

    conn.issue.push(IssueKind::NotARequirement);
}

fn collect_refinement_issues(restr: &WeakNodeRef<Node>, conn: &mut Connection<WeakNodeRef<Node>>) {
    use IssueKind::RefinementMismatch;

    let upgraded = conn.target.upgrade().unwrap();
    let target_restr = &upgraded.borrow().neighbors.outbound.restriction;

    let mut candidate = restr.upgrade().unwrap();
    while let Ok(r) = target_restr
        && r.target.as_ptr() != Rc::as_ptr(&candidate)
    {
        if let None = candidate.borrow().neighbors.parent {
            conn.issue.push(RefinementMismatch);
            break;
        }

        let parent = {
            let p = &candidate.borrow().neighbors.parent;
            p.as_ref().unwrap().target.clone()
        };
        candidate = parent.upgrade().unwrap();
    }
}

fn collect_derivation_issues(
    usages: &Vec<*const RefCell<Node>>,
    restr: &WeakNodeRef<Node>,
    conn: &mut Connection<WeakNodeRef<Node>>,
) {
    use IssueKind::{DerivationMismatch, NoDerivedAllocation, UnrelatedDerivation};

    let target = conn.target.upgrade().unwrap();
    let target_alloc = &target.borrow().neighbors.outbound.allocation;
    match target_alloc {
        Some(al) if restr.as_ptr() != al.target.as_ptr() => conn.issue.push(DerivationMismatch),
        None => conn.issue.push(NoDerivedAllocation),
        _ => {}
    }

    let target_restr = &target.borrow().neighbors.outbound.restriction;
    if let Ok(target_restr) = target_restr {
        let out_restr_usage = usages.iter().find(|u| **u == target_restr.target.as_ptr());
        if out_restr_usage.is_none() {
            conn.issue.push(UnrelatedDerivation);
        }
    }
}

fn collect_specialization_issues(
    restr: &WeakNodeRef<Node>,
    conn: &mut Connection<WeakNodeRef<Node>>,
) {
    use IssueKind::SpecializationMismatch;

    if restr.as_ptr() != conn.target.as_ptr() {
        conn.issue.push(SpecializationMismatch);
    }
}

fn collect_restriction_issues(
    usages: &Vec<*const RefCell<Node>>,
    restr: &mut Connection<WeakNodeRef<Node>>,
) {
    use IssueKind::{NotAPartOrAttribute, UnrelatedRestriction};
    use node::entity::Variant::*;

    match restr.target.upgrade().unwrap().borrow().variant {
        Part(_) | Attribute(_) => {}
        _ => restr.issue.push(NotAPartOrAttribute),
    }

    let restr_usage = usages.iter().find(|u| restr.target.as_ptr() == **u);
    if restr_usage.is_none() {
        restr.issue.push(UnrelatedRestriction);
    }
}

fn collect_allocation_issues(
    is_bottom: bool,
    has_valid_derive: bool,
    alloc: &mut Connection<WeakNodeRef<Node>>,
) {
    use IssueKind::{InhibitorForbidDerives, MissingDerivation, NotAPartOrAttribute};
    use node::entity::Variant::*;

    match alloc.target.upgrade().unwrap().borrow().variant {
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

fn collect_inhibition_issues(
    is_inhibited: bool,
    trace: &mut Traceability<Connection<WeakNodeRef<Node>>>,
) {
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

fn collect_duplicated_connection(
    seen: &mut HashSet<*const RefCell<Node>>,
    connections: &mut Vec<Connection<WeakNodeRef<Node>>>,
) {
    use IssueKind::DuplicatedTrace;

    for conn in connections {
        let ptr = conn.target.as_ptr();
        if !seen.insert(ptr) {
            conn.issue.push(DuplicatedTrace);
        }
    }
}

fn collect_conflicting_connection(
    other: &[&HashSet<*const RefCell<Node>>],
    connections: &mut Vec<Connection<WeakNodeRef<Node>>>,
) {
    use IssueKind::ConflictingTrace;

    for conn in connections {
        let ptr = conn.target.as_ptr();
        if other.iter().find(|s| s.get(&ptr).is_some()).is_some() {
            conn.issue.push(ConflictingTrace);
        }
    }
}

fn is_valid_derivation(
    conn: &Connection<WeakNodeRef<Node>>,
    alloc: &Option<Connection<WeakNodeRef<Node>>>,
) -> bool {
    if let Some(alloc) = alloc
        && let Some(tgt) = conn.target.upgrade()
        && let Ok(restr) = &tgt.borrow().neighbors.outbound.restriction
        && restr.target.ptr_eq(&alloc.target)
    {
        return true;
    }

    return false;
}
