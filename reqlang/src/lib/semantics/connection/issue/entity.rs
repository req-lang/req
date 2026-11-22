use crate::{
    node::{
        self,
        entity::{Arena, Idx},
        tree::Tree,
    },
    semantics::connection::entity::{Neighbors, Traceability},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Issue {
    MissingRequirementBinding,
    MissingUpwardTrace,
    MissingDownwardTrace,
    InvalidRequirementParent,
    InvalidPackageParent,
    InvalidAttributeParent,
    UnrestrictedSubject,
    UnusedSubject,
}

pub fn diagnostics(tree: &Tree, neighbors: &Arena<Neighbors>) -> Arena<Vec<Issue>> {
    tree.entities
        .indices()
        .map(|idx| entity_issues(tree, neighbors, idx))
        .collect()
}

fn entity_issues(tree: &Tree, neighbors: &Arena<Neighbors>, idx: Idx) -> Vec<Issue> {
    use node::entity::Variant::*;

    let entities = &tree.entities;
    let nd = &neighbors[idx];
    let mut issues = Vec::new();

    match &entities[idx].variant {
        Part(_) => subject_issues(nd, &mut issues),
        Attribute(_) => {
            if let Some(parent) = nd.parent
                && !matches!(entities[parent].variant, Part(_))
            {
                issues.push(Issue::InvalidAttributeParent);
            }

            subject_issues(nd, &mut issues);
        }
        Package(_) => {
            if let Some(parent) = nd.parent
                && !matches!(entities[parent].variant, Package(_))
            {
                issues.push(Issue::InvalidPackageParent);
            }
        }
        Type(_) => {}
        Requirement(req) => {
            if let Some(parent) = nd.parent
                && !matches!(entities[parent].variant, Package(_))
            {
                issues.push(Issue::InvalidRequirementParent);
            }

            if !req.inhibitor.top && !has_trace(&nd.outbound.traceability) {
                issues.push(Issue::MissingUpwardTrace);
            }
            if !req.inhibitor.bottom && !has_trace(&nd.inbound.traceability) {
                issues.push(Issue::MissingDownwardTrace);
            }

            if nd.outbound.restriction.is_none() {
                issues.push(Issue::MissingRequirementBinding);
            }
        }
    }

    issues
}

fn subject_issues(nd: &Neighbors, issues: &mut Vec<Issue>) {
    if nd.inbound.restrictions.is_empty() {
        issues.push(Issue::UnrestrictedSubject);
    }

    if nd.inbound.usages.is_empty() {
        issues.push(Issue::UnusedSubject);
    }
}

/// Whether a requirement is traced to another, or traced by another, in the hierarchy
fn has_trace(trace: &Traceability) -> bool {
    !trace.derivations.is_empty() || !trace.refinements.is_empty()
}
