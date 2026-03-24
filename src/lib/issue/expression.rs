use crate::{
    issue::{IssueCode, IssueLocation, IssueSeverity},
    syntax::{entity::EntityKey, expression::ExpressionKey},
    verifier::typing::TypeKind,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExpressionIssueKind<'a> {
    UnexpectedType(TypeKind, TypeKind),
    NotAnAttribute(EntityKey<'a>),
    NotASet(TypeKind),
}

impl ExpressionIssueKind<'_> {
    pub const UNEXPECTED_TYPE: IssueCode = IssueCode(33);
    pub const NOT_AN_ATTRIBUTE: IssueCode = IssueCode(34);
    pub const NOT_A_SET: IssueCode = IssueCode(35);
}

pub struct ExpressionIssue<'a> {
    pub location: ExpressionKey<'a>,
    pub kind: ExpressionIssueKind<'a>,
}

impl<'a> ExpressionIssue<'a> {
    pub fn new(location: ExpressionKey<'a>, kind: ExpressionIssueKind<'a>) -> Self {
        Self { location, kind }
    }
}

impl<'a> super::Issue<'a> for ExpressionIssue<'a> {
    fn location(&self) -> IssueLocation<'a> {
        IssueLocation::Expression(self.location.clone())
    }

    fn message(&self) -> String {
        use ExpressionIssueKind::*;
        let label = |key: &EntityKey| key.0.meta.label.to_string();
        match &self.kind {
            UnexpectedType(expected, got) => format!("Expected {expected} got {got}"),
            NotAnAttribute(key) => format!("Expected an attribute at {}", label(key)),
            NotASet(t) => format!("Expected a set got {t}"),
        }
    }

    fn hint(&self) -> String {
        todo!()
    }

    fn severity(&self) -> IssueSeverity {
        IssueSeverity::Critical
    }

    fn code(&self) -> IssueCode {
        use ExpressionIssueKind::*;
        match &self.kind {
            UnexpectedType(_, _) => ExpressionIssueKind::UNEXPECTED_TYPE,
            NotAnAttribute(_) => ExpressionIssueKind::NOT_AN_ATTRIBUTE,
            NotASet(_) => ExpressionIssueKind::NOT_A_SET,
        }
    }
}
