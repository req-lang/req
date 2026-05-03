use crate::{
    node::{
        entity::{Idx, Node},
        expression::Variant,
    },
    semantics::{
        connection::{self, Connected},
        link::{self},
        path::{self, Located},
        typing::{self, Typed},
    },
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
    Reference(link::Issue),
    #[error("type error: {0}")]
    Type(typing::expression::Issue),
    #[error("connection error: {0}")]
    Connection(connection::issue::IssueKind),
    #[error("path error: {0}")]
    Path(path::Issue),
}

impl From<&Issue> for Severity {
    fn from(value: &Issue) -> Self {
        match value {
            Issue::Reference(_) => Severity::Critical,
            Issue::Type(issue) => issue.into(),
            Issue::Connection(issue_kind) => issue_kind.into(),
            Issue::Path(_) => Severity::Critical,
        }
    }
}

pub struct IssueIterator {
    issues: Vec<Issue>,
}

pub fn expr_errors<N, E: Typed, M>(value: &link::entity::Node<N, E, M>) -> IssueIterator {
    let mut issues = Vec::new();

    for ex in value.variant.expressions() {
        ex.for_each(|x, _| {
            if let Err(typ) = x.payload.type_result() {
                issues.push(typ.into());
            }
        });
    }

    IssueIterator { issues }
}

pub fn connection_errors<N: Connected, E, M>(node: &link::entity::Node<N, E, M>) -> IssueIterator {
    let mut issues = Vec::new();

    let neighbors = node.payload.neigbhors();
    let parent = neighbors.parent.iter();
    issues.extend(conn_issues(parent).map(Issue::from));

    let ib = &neighbors.inbound;
    issues.extend(ib.issues.iter().map(|i| i.into()));
    issues.extend(conn_issues(ib.usages.iter()).map(Issue::from));
    issues.extend(conn_issues(ib.allocations.iter()).map(Issue::from));
    issues.extend(conn_issues(ib.restrictions.iter()).map(Issue::from));

    let tr = &ib.traceability;
    issues.extend(tr.issues.iter().map(|i| i.into()));
    issues.extend(conn_issues(tr.specializations.iter()).map(Issue::from));
    issues.extend(conn_issues(tr.refinements.iter()).map(Issue::from));
    issues.extend(conn_issues(tr.derivations.iter()).map(Issue::from));

    let ob = &neighbors.outbound;

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

    IssueIterator { issues }
}

pub fn reference_errors<N, E, M>(node: &link::entity::Node<N, E, M>) -> IssueIterator {
    let mut issues = Vec::new();

    let variant = &node.variant;
    let rf_errors = variant
        .references()
        .filter_map(|r| r.resolved.as_ref().err());
    issues.extend(rf_errors.map(|err| err.into()));
    for mk in variant.markups() {
        let rf_errors = mk
            .container
            .references()
            .filter_map(|r| r.resolved.as_ref().err());
        issues.extend(rf_errors.map(|err| err.into()));
    }

    for ex in node.variant.expressions() {
        ex.for_each(|x, _| {
            if let Variant::Identifier(ident) = &x.variant
                && let Err(rf) = &ident.target.resolved
            {
                issues.push(rf.into());
            }
        });
    }

    IssueIterator { issues }
}

pub fn path_issues<N: Located, R, T, E, M>(node: &Node<N, R, T, E, M>) -> IssueIterator {
    let issue = &node.payload.location().issue;
    IssueIterator {
        issues: issue.iter().cloned().map(Issue::Path).collect(),
    }
}

impl Iterator for IssueIterator {
    type Item = Issue;

    fn next(&mut self) -> Option<Self::Item> {
        self.issues.pop()
    }
}

impl From<&link::Issue> for Issue {
    fn from(value: &link::Issue) -> Self {
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
    conn: impl Iterator<Item = &'a connection::entity::Connection<Idx>>,
) -> impl Iterator<Item = &'a connection::issue::IssueKind> {
    conn.map(|i| &i.issue).flatten()
}
