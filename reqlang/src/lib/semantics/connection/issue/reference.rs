use std::collections::HashSet;

use crate::{
    issue::{
        Severity,
        code::{IssueCode, codes},
    },
    node::{
        self,
        entity::{Arena, Idx, TraceabilityKind},
        reference,
        tree::Tree,
    },
    semantics::{connection::entity::Neighbors, link::Links},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Issue {
    #[error("part or attribute expected")]
    NotAPartOrAttribute,
    #[error("requirement expected")]
    NotARequirement,
    #[error("the requirement is allocated but the inhibitor forbids derivation")]
    InhibitorForbidDerives,
    #[error("allocation of an entity without a deriving requirement")]
    MissingDerivation,
    #[error("traceability link is duplicated")]
    DuplicatedTrace,
    #[error("traceability link is in conflict")]
    ConflictingTrace,
    #[error("specialization with different restricted subjects")]
    SpecializationMismatch,
    #[error("traceability link is part of a cycle")]
    CyclicTrace,
    #[error("the requirement is inhibited upward but traces to another requirement")]
    ForbiddenUpwardTrace,
    #[error("the traced requirement is inhibited downward")]
    ForbiddenDownwardTrace,
    #[error("refinement does not restrict a child of the refined restriction")]
    RefinementMismatch,
    #[error("this requirement does not derive a requirement that allocates")]
    NoDerivedAllocation,
    #[error("derived requirement is not allocated to the restriction of this requirement")]
    DerivationMismatch,
    #[error("derived requirement is not using its parent restriction")]
    UnrelatedDerivation,
    #[error("requirement does not use its restriction in its expression")]
    UnrelatedRestriction,
    #[error("allocation and restriction should be on different subjects")]
    SameAllocationAndRestriction,
}

impl Issue {
    pub fn code(&self) -> IssueCode {
        match self {
            Issue::NotAPartOrAttribute => codes::NOT_A_PART_OR_ATTRIBUTE,
            Issue::NotARequirement => codes::NOT_A_REQUIREMENT,
            Issue::InhibitorForbidDerives => codes::INHIBITOR_FORBID_DERIVES,
            Issue::MissingDerivation => codes::MISSING_DERIVATION,
            Issue::DuplicatedTrace => codes::DUPLICATED_TRACE,
            Issue::ConflictingTrace => codes::CONFLICTING_TRACE,
            Issue::SpecializationMismatch => codes::SPECIALIZATION_MISMATCH,
            Issue::CyclicTrace => codes::CYCLIC_TRACE,
            Issue::ForbiddenUpwardTrace => codes::FORBIDDEN_UPWARD_TRACE,
            Issue::ForbiddenDownwardTrace => codes::FORBIDDEN_DOWNWARD_TRACE,
            Issue::RefinementMismatch => codes::REFINEMENT_MISMATCH,
            Issue::NoDerivedAllocation => codes::NO_DERIVED_ALLOCATION,
            Issue::DerivationMismatch => codes::DERIVATION_MISMATCH,
            Issue::UnrelatedDerivation => codes::UNRELATED_DERIVATION,
            Issue::UnrelatedRestriction => codes::UNRELATED_RESTRICTION,
            Issue::SameAllocationAndRestriction => codes::SAME_ALLOCATION_AND_RESTRICTION,
        }
    }
}

impl From<&Issue> for Severity {
    fn from(issue: &Issue) -> Self {
        use Issue::*;
        use Severity::*;

        match issue {
            NotARequirement => Critical,
            InhibitorForbidDerives => Critical,
            NotAPartOrAttribute => Critical,

            ConflictingTrace => Severe,
            CyclicTrace => Severe,
            SpecializationMismatch => Severe,
            RefinementMismatch => Severe,
            DerivationMismatch => Severe,

            UnrelatedRestriction => Moderate,
            UnrelatedDerivation => Moderate,
            SameAllocationAndRestriction => Moderate,
            DuplicatedTrace => Moderate,
            ForbiddenUpwardTrace => Moderate,
            ForbiddenDownwardTrace => Moderate,
            NoDerivedAllocation => Moderate,

            MissingDerivation => Light,
        }
    }
}

type Issues = reference::Arena<Vec<Issue>>;

pub fn diagnostics(tree: &Tree, links: &Links, neighbors: &Arena<Neighbors>) -> Issues {
    let mut issues = tree.references.parallel();
    Diagnoser::new(tree, links, neighbors).diagnose(&mut issues);
    issues
}

struct Diagnoser<'a> {
    tree: &'a Tree,
    links: &'a Links,
    neighbors: &'a Arena<Neighbors>,
    cycles: HashSet<(Idx, Idx)>,
}

impl<'a> Diagnoser<'a> {
    fn new(tree: &'a Tree, links: &'a Links, neighbors: &'a Arena<Neighbors>) -> Self {
        Self {
            tree,
            links,
            neighbors,
            cycles: cyclic_links(neighbors),
        }
    }

    fn diagnose(&self, issues: &mut Issues) {
        use node::entity::Variant::Requirement;

        for (idx, node) in self.tree.entities.enumerate() {
            if let Requirement(req) = &node.variant {
                self.requirement(idx, req, issues);
            }
        }
    }

    fn requirement(&self, idx: Idx, req: &node::entity::Requirement, issues: &mut Issues) {
        let outbound = &self.neighbors[idx].outbound;
        let (restriction, allocation) = (outbound.restriction, outbound.allocation);

        if let (Some(reference), Some(alloc)) = (req.allocation, allocation) {
            self.allocation(idx, alloc, req.inhibitor.bottom, &mut issues[reference]);
        }

        if let (Some(reference), Some(restr)) = (req.restriction, restriction) {
            self.restriction(idx, restr, &mut issues[reference]);
        }

        if let (Some(restr), Some(alloc)) = (restriction, allocation)
            && restr == alloc
        {
            for reference in req.restriction.into_iter().chain(req.allocation) {
                issues[reference].push(Issue::SameAllocationAndRestriction);
            }
        }

        self.traceability(idx, req, issues);
    }

    fn restriction(&self, idx: Idx, restr: Idx, issues: &mut Vec<Issue>) {
        if !self.is_part_or_attribute(restr) {
            issues.push(Issue::NotAPartOrAttribute);
        }

        if !self.neighbors[idx].outbound.usages.contains(&restr) {
            issues.push(Issue::UnrelatedRestriction);
        }
    }

    fn allocation(&self, idx: Idx, alloc: Idx, is_bottom: bool, issues: &mut Vec<Issue>) {
        if !self.is_part_or_attribute(alloc) {
            issues.push(Issue::NotAPartOrAttribute);
        }

        if is_bottom {
            issues.push(Issue::InhibitorForbidDerives);
        }

        if !self.has_valid_derivation(idx) {
            issues.push(Issue::MissingDerivation);
        }
    }

    fn traceability(&self, idx: Idx, req: &node::entity::Requirement, issues: &mut Issues) {
        use node::entity::Variant::Requirement;

        let entities = &self.tree.entities;
        let resolved: Vec<(reference::Idx, TraceabilityKind, Idx)> = req
            .traceability
            .iter()
            .filter_map(|link| Some((link.target, link.kind, self.links.references[link.target]?)))
            .collect();

        let mut seen: Vec<(TraceabilityKind, Idx)> = Vec::new();
        for &(reference, kind, target) in &resolved {
            let issues = &mut issues[reference];

            if seen.contains(&(kind, target)) {
                issues.push(Issue::DuplicatedTrace);
            }
            seen.push((kind, target));

            let is_conflicting = resolved
                .iter()
                .any(|&(_, other, t)| other != kind && t == target);
            if is_conflicting {
                issues.push(Issue::ConflictingTrace);
            }

            if let Some(restr) = self.neighbors[idx].outbound.restriction {
                match kind {
                    TraceabilityKind::Specialization
                        if self.is_specialization_mismatch(restr, target) =>
                    {
                        issues.push(Issue::SpecializationMismatch);
                    }
                    TraceabilityKind::Refinement if self.is_refinement_mismatch(restr, target) => {
                        issues.push(Issue::RefinementMismatch);
                    }
                    TraceabilityKind::Derivation => self.derivation(idx, restr, target, issues),
                    _ => {}
                }
            }

            if self.cycles.contains(&(idx, target)) {
                issues.push(Issue::CyclicTrace);
            }

            // Only derivations and refinements are in the hierarchy
            if kind != TraceabilityKind::Specialization {
                if req.inhibitor.top {
                    issues.push(Issue::ForbiddenUpwardTrace);
                }

                if let Requirement(traced) = &entities[target].variant
                    && traced.inhibitor.bottom
                {
                    issues.push(Issue::ForbiddenDownwardTrace);
                }
            }

            if !matches!(entities[target].variant, Requirement(_)) {
                issues.push(Issue::NotARequirement);
            }
        }
    }

    fn derivation(&self, idx: Idx, restr: Idx, target: Idx, issues: &mut Vec<Issue>) {
        match self.neighbors[target].outbound.allocation {
            Some(alloc) if restr != alloc => issues.push(Issue::DerivationMismatch),
            None => issues.push(Issue::NoDerivedAllocation),
            _ => {}
        }

        if self.is_unrelated_derivation(idx, target) {
            issues.push(Issue::UnrelatedDerivation);
        }
    }

    fn is_part_or_attribute(&self, idx: Idx) -> bool {
        use node::entity::Variant::*;

        matches!(self.tree.entities[idx].variant, Part(_) | Attribute(_))
    }

    /// Whether a requirement derives to the one that restricts what the entity is allocated to
    fn has_valid_derivation(&self, idx: Idx) -> bool {
        let neighbors = self.neighbors;
        let allocation = neighbors[idx].outbound.allocation;

        neighbors[idx]
            .inbound
            .traceability
            .derivations
            .iter()
            .any(|derivation| {
                allocation
                    .is_some_and(|alloc| neighbors[*derivation].outbound.restriction == Some(alloc))
            })
    }

    /// The specialized requirement restricts something else than what this one restricts
    fn is_specialization_mismatch(&self, restr: Idx, target: Idx) -> bool {
        self.neighbors[target]
            .outbound
            .restriction
            .is_some_and(|target_restr| target_restr != restr)
    }

    /// The requirement does not use what the requirement it derives from restricts
    // TODO: subject to change, see the Journal note "Derivation and the restriction of the parent"
    fn is_unrelated_derivation(&self, idx: Idx, target: Idx) -> bool {
        self.neighbors[target]
            .outbound
            .restriction
            .is_some_and(|target_restr| {
                !self.neighbors[idx].outbound.usages.contains(&target_restr)
            })
    }

    /// The refined requirement does not restrict an ancestor of what this one restricts
    fn is_refinement_mismatch(&self, restr: Idx, target: Idx) -> bool {
        let Some(target_restr) = self.neighbors[target].outbound.restriction else {
            return false;
        };

        let mut candidate = Some(restr);
        while let Some(cnd) = candidate {
            if cnd == target_restr {
                return false;
            }

            candidate = self.neighbors[cnd].parent;
        }

        true
    }
}

/// The traceability links that are part of a cycle, as the pair of the requirement that traces
/// and the requirement it traces to
fn cyclic_links(neighbors: &Arena<Neighbors>) -> HashSet<(Idx, Idx)> {
    let cycles = neighbors.cycles(|node| {
        node.inbound
            .traceability
            .iter()
            .map(|(t, _)| t)
            .collect::<Vec<_>>()
    });

    // Each requirement of a cycle is traced by the next one, the first by the last
    cycles
        .iter()
        .flat_map(|cycle| {
            cycle
                .iter()
                .zip(cycle.iter().cycle().skip(1))
                .map(|(traced, tracing)| (*tracing, *traced))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Specification, syntax::parser::parse};

    /// The issues of every reference of the input, in no particular order
    fn issues_of(input: &str) -> Vec<Issue> {
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty(), "syntax errors: {:?}", info.errors);

        let mut spec = Specification::from(tree);
        spec.diagnose();
        spec.analysis
            .connection
            .reference
            .into_iter()
            .flatten()
            .collect()
    }

    /// The wheel stops thanks to the brake, which derives the requirement of the wheel. No issue.
    fn deriving(brake_text: &str) -> String {
        format!(
            "package System
  part Wheel
  part

  part Brake
  part

  requirement Wheel_stop top
    on Wheel
    via Brake
  is
    @@
      The [[Wheel]] shall stop.
    @@
  requirement

  requirement Brake_stops_wheel bottom
    on Brake
    derives Wheel_stop
  is
    @@
      {brake_text}
    @@
  requirement
package
"
        )
    }

    /// A requirement on the wheel, and a specialization of it on `subject`
    fn specializing(subject: &str) -> String {
        format!(
            "package System
  part Wheel
  part

  part Brake
  part

  requirement Wheel_stop top bottom
    on Wheel
  is
    @@
      The [[Wheel]] shall stop.
    @@
  requirement

  requirement Specific_stop top bottom
    on {subject}
    specializes Wheel_stop
  is
    @@
      The [[{subject}]] shall stop on a wet road.
    @@
  requirement
package
"
        )
    }

    #[test]
    fn a_specialization_of_the_same_subject_is_valid() {
        let issues = issues_of(&specializing("Wheel"));

        assert!(
            !issues.contains(&Issue::SpecializationMismatch),
            "{issues:?}"
        );
    }

    #[test]
    fn a_specialization_of_another_subject_is_a_mismatch() {
        let issues = issues_of(&specializing("Brake"));

        assert_eq!(
            issues
                .iter()
                .filter(|i| **i == Issue::SpecializationMismatch)
                .count(),
            1,
            "{issues:?}"
        );
    }

    #[test]
    fn a_derivation_using_the_restriction_of_its_parent_is_related() {
        let issues = issues_of(&deriving("The [[Brake]] shall stop the [[Wheel]]."));

        assert_eq!(issues, vec![]);
    }

    #[test]
    fn a_derivation_ignoring_the_restriction_of_its_parent_is_unrelated() {
        let issues = issues_of(&deriving("The [[Brake]] shall act."));

        assert_eq!(issues, vec![Issue::UnrelatedDerivation]);
    }

    #[test]
    fn a_derivation_ignoring_its_own_restriction_is_an_unrelated_restriction_only() {
        let issues = issues_of(&deriving("The wheel shall stop thanks to the [[Wheel]]."));

        assert_eq!(issues, vec![Issue::UnrelatedRestriction]);
    }
}
