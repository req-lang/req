use crate::{
    Specification,
    issue::Issue::Reference,
    node::{
        entity::{self, Idx},
        expression::Variant,
        tree::Tree,
    },
    semantics::{
        connection::{self, entity::Neighbors},
        link::Links,
        path,
        typing::{self, Result},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Light,
    Moderate,
    Severe,
    Critical,
}

#[derive(Debug, thiserror::Error)]
pub enum Issue {
    #[error("reference not found")]
    Reference,
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
            Issue::Reference => Severity::Critical,
            Issue::Type(issue) => issue.into(),
            Issue::Connection(kind) => kind.into(),
            Issue::Path(_) => Severity::Critical,
        }
    }
}

/// Every issue of every entity, `analysis` must have been diagnosed
pub fn issues(spec: &Specification) -> entity::Arena<Vec<Issue>> {
    spec.tree
        .entities
        .indices()
        .zip(&spec.analysis.paths)
        .zip(&spec.analysis.neighbors)
        .map(|((idx, location), neighbors)| {
            path_issues(location)
                .chain(reference_errors(&spec.tree, &spec.analysis.links, idx))
                .chain(connection_errors(neighbors))
                .chain(expr_errors(&spec.tree, &spec.analysis.types, idx))
                .map(Issue::from)
                .collect()
        })
        .collect()
}

pub struct IssueIterator {
    issues: Vec<Issue>,
}

pub fn expr_errors(
    tree: &Tree,
    types: &crate::node::expression::Arena<Result>,
    idx: Idx,
) -> IssueIterator {
    let mut issues = Vec::new();

    for root in tree.entities[idx].variant.expressions() {
        for (expr, _) in tree.expressions.subtree(root) {
            if let Err(typ) = &types[expr] {
                issues.push(typ.into());
            }
        }
    }

    IssueIterator { issues }
}

pub fn connection_errors(neighbors: &Neighbors) -> IssueIterator {
    let Neighbors {
        inbound,
        outbound,
        parent,
    } = neighbors;
    let trace = &outbound.traceability;

    let connections = parent
        .iter()
        .chain(&outbound.restriction)
        .chain(&outbound.allocation)
        .chain(&trace.specializations)
        .chain(&trace.refinements)
        .chain(&trace.derivations)
        .flat_map(|c| &c.issues);

    IssueIterator {
        issues: connections
            .chain(&inbound.issues)
            .chain(&inbound.traceability.issues)
            .chain(&outbound.issues)
            .cloned()
            .map(Issue::Connection)
            .collect(),
    }
}

pub fn reference_errors(tree: &Tree, links: &Links, idx: Idx) -> IssueIterator {
    let mut issues = Vec::new();

    let variant = &tree.entities[idx].variant;
    let references = variant
        .references()
        .chain(variant.markups().flat_map(|markup| markup.references()));
    issues.extend(
        references
            .filter(|r| links.references[*r].is_none())
            .map(|_| Reference),
    );

    for root in variant.expressions() {
        for (_, node) in tree.expressions.subtree(root) {
            if let Variant::Identifier(ident) = &node.variant
                && links.identifiers[ident.target].is_none()
            {
                issues.push(Reference);
            }
        }
    }

    IssueIterator { issues }
}

pub fn path_issues(location: &path::Location) -> IssueIterator {
    IssueIterator {
        issues: location.issue.iter().cloned().map(Issue::Path).collect(),
    }
}

impl Iterator for IssueIterator {
    type Item = Issue;

    fn next(&mut self) -> Option<Self::Item> {
        self.issues.pop()
    }
}

impl From<&typing::expression::Issue> for Issue {
    fn from(value: &typing::expression::Issue) -> Self {
        Issue::Type(value.clone())
    }
}
