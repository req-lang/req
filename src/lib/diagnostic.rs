use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use serde::Serialize;

use crate::{
    syntax::entity::{EntityKey, Reference},
    verifier::typing::TypeKind,
};

#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct IssueHint(pub String);

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum IssueKind<'a> {
    #[error("Import not found at {0}")]
    ImportNotFound(EntityKey<'a>),
    #[error("Reference not found: {0}")]
    ReferenceNotFound(&'a Reference),
    #[error("Ambiguous reference: {0}")]
    AmbiguousReference(&'a Reference),
    #[error("Redefined entity: {0}")]
    RedefinedEntity(EntityKey<'a>),
    #[error("Expected {0} got {1}")]
    UnexpectedType(TypeKind, TypeKind),
    #[error("Expected an attribute at {0}")]
    NotAnAttribute(Reference),
    #[error("Expected a requirement at {0}")]
    NotARequirement(EntityKey<'a>),
    #[error("Expected a set got {0}")]
    NotASet(TypeKind),
    #[error("Circular traceability")]
    CircularTrace(Vec<EntityKey<'a>>),
    #[error("Part or attribute expected at {0}")]
    NotPartOrAttribute(EntityKey<'a>),
    #[error("Requirement is missing a constraint")]
    MissingRequirementBinding,
    #[error("Expected specialization to constraint the same entity as {0}")]
    SpecializationMismatch(EntityKey<'a>),
    #[error("Expected refinement to constraint a children entity from {0}")]
    RefinementMismatch(EntityKey<'a>),
    #[error("Expected derivation to constraint the backward allocated entity by {0}")]
    DerivationMismatch(EntityKey<'a>),
    #[error("Expected allocation in the derived requirement {0}")]
    NoDerivedAllocation(EntityKey<'a>),
    #[error("Allocation of an entity without a deriving requirement")]
    MissingDerivation,
    #[error("Same traceability link to {0} is featured multiple times")]
    DuplicatedTrace(EntityKey<'a>),
    #[error("Different traceability link for the same target {0}")]
    ConflictingTrace(EntityKey<'a>),
    #[error("Tracebility link is duplicated")]
    SameAllocationAndConstraint,
    #[error("Unused entity")]
    Unused,
    #[error("Unrestricted entity")]
    Unrestricted,
    #[error("Uncommented entity")]
    Uncommented,
    #[error("Expected the attribute to be typed")]
    UntypedAttribute,
    #[error("Units are expected only for numerical attributes")]
    UnexpectedUnit,
    #[error("Unexpected attribute at {0}")]
    UnexpectedAttribute(EntityKey<'a>),
    #[error("Unexpected forward traceability")]
    UnexpectedForwardTrace,
    #[error("Unexpected backward traceability")]
    UnexpectedBrackwardTrace,
    #[error("Missing forward traceability")]
    MissingForwardTrace,
    #[error("Missing backward traceability")]
    MissingBackwardTrace,
    #[error("Empty package")]
    EmptyPackage,
    #[error("The requirement is allocated but the inhibitor forbids derivation")]
    InhibitorForbidDerives,
    #[error("The requirement might be a duplicate of {0}")]
    PossibleDuplicate(EntityKey<'a>),
}

impl PartialOrd for IssueKind<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        IssueSeverity::from(self).partial_cmp(&other.into())
    }
}

impl Ord for IssueKind<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        IssueSeverity::from(self).cmp(&other.into())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Critical,
    Severe,
    Moderate,
    Light,
}

impl From<&IssueKind<'_>> for IssueSeverity {
    fn from(kind: &IssueKind) -> IssueSeverity {
        use IssueKind::*;
        use IssueSeverity::*;

        match kind {
            ImportNotFound(_) => Critical,
            ReferenceNotFound(_) => Critical,
            AmbiguousReference(_) => Critical,
            RedefinedEntity(_) => Critical,
            UnexpectedType(_, _) => Critical,
            NotAnAttribute(_) => Critical,
            NotARequirement(_) => Critical,
            NotPartOrAttribute(_) => Critical,
            UnexpectedAttribute(_) => Critical,
            NotASet(_) => Critical,
            InhibitorForbidDerives => Critical,

            MissingRequirementBinding => Severe,
            ConflictingTrace(_) => Severe,
            CircularTrace(_) => Severe,
            SpecializationMismatch(_) => Severe,
            RefinementMismatch(_) => Severe,
            DerivationMismatch(_) => Severe,

            UnexpectedUnit => Moderate,
            SameAllocationAndConstraint => Moderate,
            UnexpectedForwardTrace => Moderate,
            UnexpectedBrackwardTrace => Moderate,
            DuplicatedTrace(_) => Moderate,
            UntypedAttribute => Moderate,
            NoDerivedAllocation(_) => Moderate,
            Unrestricted => Moderate,

            Unused => Light,
            Uncommented => Light,
            MissingDerivation => Light,
            MissingForwardTrace => Light,
            MissingBackwardTrace => Light,
            EmptyPackage => Light,
            PossibleDuplicate(_) => Light,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Issue<'a> {
    // TODO: To be changed if the error occurs on an expression, markup
    pub location: EntityKey<'a>,
    pub kind: IssueKind<'a>,
}

impl Display for Issue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl<'a> Issue<'a> {
    pub fn new(location: EntityKey<'a>, kind: IssueKind<'a>) -> Self {
        Self { location, kind }
    }

    pub fn propagated(location: EntityKey<'a>, err: Issue<'a>) -> Self {
        Self {
            location,
            kind: err.kind,
        }
    }
}

impl std::error::Error for Issue<'_> {}

pub struct ResultErr<U, T, E>(pub (U, Result<T, E>));

impl<U, T, E> ResultErr<U, T, E> {
    fn unwrap(self) -> (U, Result<T, E>) {
        let Self((key, result)) = self;
        (key, result)
    }
}

impl<T, E, U> FromIterator<ResultErr<U, T, E>> for (HashMap<U, T>, Vec<E>)
where
    U: Eq + std::hash::Hash,
{
    fn from_iter<I: IntoIterator<Item = ResultErr<U, T, E>>>(iter: I) -> (HashMap<U, T>, Vec<E>) {
        let (ok, errs): (Vec<_>, Vec<_>) = iter
            .into_iter()
            .map(ResultErr::unwrap)
            .partition(|(_, result)| result.is_ok());
        (
            ok.into_iter()
                .map(|(u, ok)| (u, ok.ok().unwrap()))
                .collect(),
            errs.into_iter()
                .map(|(_, err)| err.err().unwrap())
                .collect(),
        )
    }
}
