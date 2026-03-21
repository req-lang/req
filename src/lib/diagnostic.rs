use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use serde_derive::Serialize;

pub type CheckResult<T> = (T, Vec<Issue>);

use crate::{
    mapper::path::Path,
    syntax::{NodeId, entity::Reference},
    verifier::typing::TypeKind,
};

#[derive(Serialize, PartialOrd, Ord, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct IssueId(u32);

impl IssueId {
    pub fn new() -> Self {
        unsafe {
            let did = IssueId(ISSUE_ID_GENERATOR);
            ISSUE_ID_GENERATOR += 1;
            did
        }
    }
}

static mut ISSUE_ID_GENERATOR: u32 = 0;

#[derive(Clone, Debug, Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct IssueHint(pub String);

#[derive(Debug, Clone, Serialize, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum IssueKind {
    #[error("Import not found: {0}")]
    ImportNotFound(Path),
    #[error("Reference not found: {0}")]
    ReferenceNotFound(Reference),
    #[error("Ambiguous reference: {0}")]
    AmbiguousReference(Reference),
    #[error("Redefined entity: {0}")]
    RedefinedEntity(Path),
    #[error("Expected {0} got {1}")]
    UnexpectedType(TypeKind, TypeKind),
    #[error("Requiring type to be: {0}")]
    MissingType(TypeKind),
    #[error("Expected an attribute: {0}")]
    NotAnAttribute(Reference),
    #[error("Expected a requirement: {0}")]
    NotARequirement(Path),
    #[error("Expected a set: {0}")]
    NotASet(TypeKind),
    #[error("Circular traceability")]
    CircularTrace(Vec<Path>),
    #[error("Part or attribute expected: {0}")]
    NotPartOrAttribute(Path),
    #[error("Requirement is missing a constraint")]
    MissingRequirementBinding,
}

impl PartialOrd for IssueKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        IssueSeverity::from(self).partial_cmp(&other.into())
    }
}

impl Ord for IssueKind {
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

impl From<&IssueKind> for IssueSeverity {
    fn from(kind: &IssueKind) -> IssueSeverity {
        use IssueKind::*;
        use IssueSeverity::*;

        match kind {
            ImportNotFound(_) => Critical,
            ReferenceNotFound(_) => Critical,
            AmbiguousReference(_) => Critical,
            RedefinedEntity(_) => Critical,
            MissingType(_) => Critical,
            UnexpectedType(_, _) => Critical,
            NotAnAttribute(_) => Critical,
            NotARequirement(_) => Critical,
            NotPartOrAttribute(_) => Critical,
            NotASet(_) => Critical,
            CircularTrace(_) => Severe,
            MissingRequirementBinding => Severe,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Issue {
    pub did: IssueId,
    pub id: NodeId,
    pub kind: IssueKind,
    pub severity: IssueSeverity,
}

impl Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Issue {
    pub fn new(id: NodeId, kind: IssueKind) -> Self {
        let did = IssueId::new();
        let severity = IssueSeverity::from(&kind);
        Self {
            did,
            id,
            kind,
            severity,
        }
    }

    pub fn propagated(referer: NodeId, err: Issue) -> Self {
        Self {
            id: referer,
            did: err.did,
            kind: err.kind,
            severity: err.severity,
        }
    }
}

impl std::error::Error for Issue {}

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
