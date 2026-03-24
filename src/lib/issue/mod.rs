use crate::syntax::{entity::EntityKey, expression::ExpressionKey, markup::MarkupKey};

pub mod entity;

pub trait Issue<'a> {
    fn issue(&self) -> IssueLocation<'a>;
    fn message(&self) -> String;
    fn hint(&self) -> String;
    fn severity(&self) -> IssueSeverity;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IssueSeverity {
    Critical,
    Severe,
    Moderate,
    Light,
}

pub enum IssueLocation<'a> {
    Entity(EntityKey<'a>),
    Expression(ExpressionKey<'a>),
    Markup(MarkupKey<'a>),
}

impl<T: Issue> PartialOrd for T {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.severity().partial_cmp(&other.severity())
    }
}

impl<T: Issue> Ord for T {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.severity().cmp(&other.severity())
    }
}
