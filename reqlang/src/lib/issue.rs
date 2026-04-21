use std::cell::Ref;

use crate::{
    issue,
    node::expression::Variant,
    reference::Reference,
    semantics::{
        connection::{self, entity::Node},
        link::{self, WeakNodeRef},
        typing,
    },
    utils::iter::NodeIter,
};

pub enum Severity {
    Critical,
    Severe,
    Moderate,
    Light,
}

#[derive(Debug, thiserror::Error)]
pub enum Issue {
    #[error("reference error: {0}")]
    Reference(Reference),
    #[error("type error: {0}")]
    Type(typing::expression::Issue),
    #[error("connection error: {0}")]
    Connection(connection::issue::IssueKind),
}

impl From<&Issue> for Severity {
    fn from(value: &Issue) -> Self {
        match value {
            Issue::Reference(_) => Severity::Critical,
            Issue::Type(issue) => issue.into(),
            Issue::Connection(issue_kind) => issue_kind.into(),
        }
    }
}

pub struct IssueIterator {
    issues: Vec<Issue>,
}

impl From<Ref<'_, typing::entity::Node>> for IssueIterator {
    fn from(value: Ref<'_, typing::entity::Node>) -> Self {
        let mut issues = Vec::new();

        for ex in value.variant.expressions() {
            for ex in NodeIter::new(ex.clone()) {
                let ex_ref = ex.borrow();

                if let Err(typ) = &ex_ref.typ {
                    issues.push(typ.into());
                }

                if let Variant::Identifier(ident) = &ex_ref.variant
                    && let Err(rf) = &ident.target
                {
                    issues.push(rf.into());
                }
            }
        }

        Self { issues }
    }
}

impl From<Ref<'_, connection::entity::Node>> for IssueIterator {
    fn from(node: Ref<'_, connection::entity::Node>) -> Self {
        let mut issues = Vec::new();

        let parent = node.neighbors.parent.iter();
        issues.extend(conn_issues(parent).map(Issue::from));

        let ib = &node.neighbors.inbound;
        issues.extend(ib.issues.iter().map(|i| i.into()));
        issues.extend(conn_issues(ib.usages.iter()).map(Issue::from));
        issues.extend(conn_issues(ib.allocations.iter()).map(Issue::from));
        issues.extend(conn_issues(ib.restrictions.iter()).map(Issue::from));

        let tr = &ib.traceability;
        issues.extend(tr.issues.iter().map(|i| i.into()));
        issues.extend(conn_issues(tr.specializations.iter()).map(Issue::from));
        issues.extend(conn_issues(tr.refinements.iter()).map(Issue::from));
        issues.extend(conn_issues(tr.derivations.iter()).map(Issue::from));

        let ob = &node.neighbors.outbound;

        issues.extend(conn_issues(ob.usages.iter()).map(Issue::from));
        issues.extend(conn_issues(ob.allocation.iter()).map(Issue::from));

        match ob.restriction.as_ref() {
            Ok(restr) => issues.extend(restr.issue.iter().map(Issue::from)),
            Err(issue) => issues.push(issue.into()),
        }

        let tr = &ob.traceability;
        issues.extend(conn_issues(tr.specializations.iter()).map(Issue::from));
        issues.extend(conn_issues(tr.refinements.iter()).map(Issue::from));
        issues.extend(conn_issues(tr.derivations.iter()).map(Issue::from));

        Self { issues }
    }
}

impl From<Ref<'_, link::entity::Node>> for IssueIterator {
    fn from(node: Ref<link::entity::Node>) -> Self {
        let mut issues = Vec::new();

        let variant = &node.variant;
        let rf_errors = variant.references().filter_map(|r| r.as_ref().err());
        issues.extend(rf_errors.map(|err| err.into()));

        for mk in variant.markups() {
            let rf_errors = mk.container.references().filter_map(|r| r.as_ref().err());
            issues.extend(rf_errors.map(|err| err.into()));
        }

        Self { issues }
    }
}

impl Iterator for IssueIterator {
    type Item = Issue;

    fn next(&mut self) -> Option<Self::Item> {
        self.issues.pop()
    }
}

impl From<&Reference> for Issue {
    fn from(value: &Reference) -> Self {
        Issue::Reference(value.clone())
    }
}

impl From<&typing::expression::Issue> for Issue {
    fn from(value: &typing::expression::Issue) -> Self {
        Issue::Type(value.clone())
    }
}

impl From<&connection::issue::IssueKind> for Issue {
    fn from(value: &connection::issue::IssueKind) -> Self {
        Issue::Connection(value.clone())
    }
}

fn conn_issues<'a>(
    conn: impl Iterator<Item = &'a connection::entity::Connection<WeakNodeRef<Node>>>,
) -> impl Iterator<Item = &'a connection::issue::IssueKind> {
    conn.map(|i| &i.issue).flatten()
}
