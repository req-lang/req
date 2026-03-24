use crate::syntax::{entity::EntityKey, expression::ExpressionKey, markup::MarkupKey};

pub mod entity;
pub mod expression;

pub trait Issue<'a> {
    fn location(&self) -> IssueLocation<'a>;
    fn message(&self) -> String;
    fn hint(&self) -> String;
    fn severity(&self) -> IssueSeverity;
    fn code(&self) -> IssueCode;
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

pub struct IssueCode(usize);
