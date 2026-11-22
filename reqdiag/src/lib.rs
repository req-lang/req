use std::fmt::Display;

use reqlang::{
    Specification,
    issue::{self, Issue, IssueCode, code::codes},
    node::path::Path,
    semantics::{
        connection::issue::{entity, reference},
        path, typing,
    },
    syntax::{self},
};
use serde::Serialize;

#[derive(Default, Debug, Clone, Serialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub index: u32,
}

impl From<syntax::location::Position> for Position {
    fn from(value: syntax::location::Position) -> Self {
        Position {
            line: value.line as u32,
            column: value.column as u32,
            index: value.index as u32,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl From<syntax::location::Span> for Span {
    fn from(value: syntax::location::Span) -> Self {
        Self {
            start: value.start.into(),
            end: value.end.into(),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub path: Option<String>,
    pub span: Span,
    pub message: String,
    pub severity: &'static str,
    pub kind: &'static str,
    pub snippet: String,
}

pub trait DiagnosticString {
    fn to_diag(&self) -> String;
}

pub trait DiagnosticStr {
    fn as_diag(&self) -> &'static str;
}

impl DiagnosticString for IssueCode {
    fn to_diag(&self) -> String {
        self.to_string()
    }
}

impl DiagnosticString for Issue {
    fn to_diag(&self) -> String {
        match self {
            Issue::Reference(_) | Issue::Identifier(_) => {
                "reference target not found in scope".to_string()
            }
            Issue::Type(issue, _) => issue.to_diag(),
            Issue::Owned(issue) => issue.to_diag(),
            Issue::Network(issue, _) => issue.to_diag(),
            Issue::Path(issue) => issue.to_diag(),
        }
    }
}

impl DiagnosticString for path::Issue {
    fn to_diag(&self) -> String {
        match self {
            path::Issue::Duplicated => "two entities have the same name at the same location",
        }
        .to_string()
    }
}

impl DiagnosticString for typing::expression::Issue {
    fn to_diag(&self) -> String {
        use typing::expression::Issue::*;

        match self {
            Poison => "previously caught error".to_string(),
            NotASet(found) => format!("expected a set type got {found}"),
            UnexpectedType(expected, found) => format!("expected type {expected} got {found}"),
            NotEvaluable => "this expression points to an entity that has no type".to_string(),
        }
    }
}

impl DiagnosticString for entity::Issue {
    fn to_diag(&self) -> String {
        use entity::Issue::*;

        match self {
            MissingRequirementBinding => "requirement is missing a constraint",
            MissingUpwardTrace => "missing upward traceability",
            MissingDownwardTrace => "missing downward traceability",
            InvalidRequirementParent => "requirements are expected to be contained in packages",
            InvalidPackageParent => "packages are expected to be contained in packages",
            InvalidAttributeParent => "attributes are expected to be contained in parts",
            UnrestrictedSubject => "this entity is not restricted by any requirement",
            UnusedSubject => "this entity is not used by any requirement",
        }
        .to_string()
    }
}

impl DiagnosticString for reference::Issue {
    fn to_diag(&self) -> String {
        use reference::Issue::*;

        match self {
            NotAPartOrAttribute => "part or attribute expected",
            NotARequirement => "requirement expected",
            InhibitorForbidDerives => {
                "the requirement is allocated but the inhibitor forbids derivation"
            }
            MissingDerivation => "allocation of an entity without a deriving requirement",
            DuplicatedTrace => "traceability link is duplicated",
            ConflictingTrace => "traceability link is in conflict",
            SpecializationMismatch => "specialization with different restricted subjects",
            CyclicTrace => "traceability link is part of a cycle",
            ForbiddenUpwardTrace => {
                "the requirement is inhibited upward but traces to another requirement"
            }
            ForbiddenDownwardTrace => "the traced requirement is inhibited downward",
            RefinementMismatch => "refinement does not restrict a child of the refined restriction",
            NoDerivedAllocation => "this requirement does not derive a requirement that allocates",
            DerivationMismatch => {
                "derived requirement is not allocated to the restriction of this requirement"
            }
            UnrelatedDerivation => "derived requirement is not using its parent restriction",
            UnrelatedRestriction => "requirement does not use its restriction in its expression",
            SameAllocationAndRestriction => {
                "allocation and restriction should be on different subjects"
            }
        }
        .to_string()
    }
}

impl DiagnosticStr for issue::code::Kind {
    fn as_diag(&self) -> &'static str {
        use issue::code::Kind::*;

        match self {
            Syntax => "syntax",
            Path => "path",
            Reference => "reference",
            Typing => "typing",
            Connection => "connection",
        }
    }
}

impl DiagnosticStr for issue::Severity {
    fn as_diag(&self) -> &'static str {
        use issue::Severity::*;

        match self {
            Critical => "critical",
            Severe => "severe",
            Moderate => "moderate",
            Light => "light",
        }
    }
}

impl Diagnostic {
    fn semantic(issue: Issue, path: &Path, span: syntax::location::Span, source: &str) -> Self {
        Self {
            code: issue.code().to_diag(),
            path: Some(path.to_string()),
            span: span.into(),
            message: issue.to_diag(),
            kind: issue.kind().as_diag(),
            severity: issue.severity().as_diag(),
            snippet: source[span.range()].to_string(),
        }
    }

    fn syntax(span: syntax::location::Span, source: &str) -> Self {
        let code = codes::SYNTAX_ERROR;
        Self {
            severity: issue::Severity::Critical.as_diag(),
            code: code.to_diag(),
            path: None,
            span: span.into(),
            message: "syntax error".into(),
            kind: code.kind().as_diag(),
            snippet: source[span.range()].to_string(),
        }
    }

    pub fn collect(
        spec: &Specification,
        info: &syntax::parser::Info,
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        diagnostics.extend(
            info.errors
                .iter()
                .map(|span| Diagnostic::syntax(*span, source)),
        );

        let issues = issue::issues(spec);
        diagnostics.extend(
            issues
                .indices()
                .zip(issues)
                .into_iter()
                .zip(&spec.analysis.paths)
                .map(|((idx, issues), loc)| {
                    issues.into_iter().map(move |issue| {
                        let span = issue.span(idx, &info);
                        Diagnostic::semantic(issue, &loc.path, span, source)
                    })
                })
                .flatten(),
        );

        diagnostics
    }
}
