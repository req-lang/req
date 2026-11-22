use reqlang::{
    issue::Issue,
    semantics::{
        connection::issue::{entity, reference},
        typing,
    },
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Light,
    Moderate,
    Severe,
    Critical,
}

impl From<&Issue> for Severity {
    fn from(value: &Issue) -> Self {
        match value {
            Issue::Reference(_) | Issue::Identifier(_) => Severity::Critical,
            Issue::Type(issue, _) => issue.into(),
            Issue::Owned(issue) => issue.into(),
            Issue::Network(issue, _) => issue.into(),
            Issue::Path(_) => Severity::Critical,
        }
    }
}

impl From<&typing::expression::Issue> for Severity {
    fn from(_: &typing::expression::Issue) -> Self {
        Severity::Critical
    }
}

impl From<&entity::Issue> for Severity {
    fn from(issue: &entity::Issue) -> Self {
        use Severity::*;
        use entity::Issue::*;

        match issue {
            InvalidRequirementParent => Critical,
            InvalidPackageParent => Critical,
            InvalidAttributeParent => Critical,

            MissingRequirementBinding => Severe,

            UnrestrictedSubject => Moderate,

            UnusedSubject => Light,
            MissingUpwardTrace => Light,
            MissingDownwardTrace => Light,
        }
    }
}

impl From<&reference::Issue> for Severity {
    fn from(issue: &reference::Issue) -> Self {
        use Severity::*;
        use reference::Issue::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severities_are_ordered_from_light_to_critical() {
        assert!(Severity::Light < Severity::Moderate);
        assert!(Severity::Moderate < Severity::Severe);
        assert!(Severity::Severe < Severity::Critical);
    }

    #[test]
    fn a_severity_is_serialized_in_lowercase() {
        assert_eq!(
            serde_json::to_string(&Severity::Critical).unwrap(),
            "\"critical\""
        );
        assert_eq!(serde_json::to_string(&Severity::Light).unwrap(), "\"light\"");
    }
}
