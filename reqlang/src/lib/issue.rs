use crate::{
    Specification,
    issue::Issue::Reference,
    node::{
        self,
        entity::{self, Idx},
        expression::{self, Variant},
        identifier, reference,
        tree::Tree,
    },
    semantics::{
        Analysis, connection,
        link::Links,
        path,
        typing::{self, Result},
    },
    syntax::{location::Span, parser::Info},
};

pub mod code;

pub use code::{IssueCode, Kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Light,
    Moderate,
    Severe,
    Critical,
}

#[derive(Debug, thiserror::Error)]
pub enum Issue {
    #[error("reference target not found in scope")]
    Reference(reference::Idx),
    #[error("reference target not found in scope")]
    Identifier(identifier::Idx),
    #[error("{0}")]
    Type(typing::expression::Issue, expression::Idx),
    #[error("{0}")]
    Owned(connection::issue::entity::Issue),
    #[error("{0}")]
    Network(connection::issue::reference::Issue, reference::Idx),
    #[error("{0}")]
    Path(path::Issue),
}

impl From<&Issue> for Severity {
    fn from(value: &Issue) -> Self {
        value.severity()
    }
}

impl Issue {
    /// Where the issue is in the source: the most precise span the issue knows, `entity` (the
    /// span of the entity it was reported on) otherwise
    pub fn span(&self, entity: node::entity::Idx, info: &Info) -> Span {
        match self {
            Issue::Type(_, expr) => info.expression.span_of(*expr),
            Issue::Identifier(identifier) => info.identifiers.span_of(*identifier),
            Issue::Reference(reference) => info.references.span_of(*reference),
            Issue::Network(_, reference) => info.references.span_of(*reference),
            Issue::Owned(_) | Issue::Path(_) => {
                info.entities.tokens_of(entity).first().unwrap().span
            }
        }
    }

    pub fn severity(&self) -> Severity {
        match self {
            Issue::Reference(_) | Issue::Identifier(_) => Severity::Critical,
            Issue::Type(issue, _) => issue.into(),
            Issue::Owned(kind) => kind.into(),
            Issue::Network(kind, _) => kind.into(),
            Issue::Path(_) => Severity::Critical,
        }
    }

    /// The code that identifies the kind of the issue
    pub fn code(&self) -> IssueCode {
        match self {
            Issue::Reference(_) | Issue::Identifier(_) => code::codes::UNRESOLVED_REFERENCE,
            Issue::Type(issue, _) => issue.code(),
            Issue::Owned(issue) => issue.code(),
            Issue::Network(issue, _) => issue.code(),
            Issue::Path(issue) => issue.code(),
        }
    }

    /// What the issue is about, the kind of its code
    pub fn kind(&self) -> Kind {
        self.code().kind()
    }
}

/// Every issue of every entity, `analysis` must have been diagnosed
pub fn issues(spec: &Specification) -> entity::Arena<Vec<Issue>> {
    let analysis = &spec.analysis;
    spec.tree
        .entities
        .indices()
        .zip(&analysis.paths)
        .zip(&analysis.connection.entity)
        .map(|((idx, location), owned)| {
            path_issues(location)
                .chain(reference_errors(&spec.tree, &analysis.links, idx))
                .chain(connection_entity_errors(owned))
                .chain(connection_reference_errors(&spec.tree, analysis, idx))
                .chain(expr_errors(&spec.tree, &analysis.types, idx))
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
            if let Err(typ) = &types[expr]
                && !matches!(typ, typing::expression::Issue::Poison)
            {
                issues.push(Issue::Type(typ.clone(), expr));
            }
        }
    }

    IssueIterator { issues }
}

pub fn connection_entity_errors(owned: &[connection::issue::entity::Issue]) -> IssueIterator {
    IssueIterator {
        issues: owned.iter().cloned().map(Issue::Owned).collect(),
    }
}

/// The issues of the references of the entity
pub fn connection_reference_errors(tree: &Tree, analysis: &Analysis, idx: Idx) -> IssueIterator {
    let issues = tree.entities[idx]
        .variant
        .references()
        .flat_map(|reference| {
            analysis.connection.reference[reference]
                .iter()
                .map(move |kind| Issue::Network(kind.clone(), reference))
        })
        .collect();

    IssueIterator { issues }
}

pub fn reference_errors(tree: &Tree, links: &Links, idx: Idx) -> IssueIterator {
    let mut issues = Vec::new();

    let variant = &tree.entities[idx].variant;
    let references = variant.references().chain(
        variant
            .markups()
            .flat_map(|markup| tree.markups[markup].references()),
    );
    issues.extend(
        references
            .filter(|r| links.references[*r].is_none())
            .map(Reference),
    );

    for root in variant.expressions() {
        for (_, node) in tree.expressions.subtree(root) {
            if let Variant::Identifier(ident) = &node.variant
                && links.identifiers[ident.target].is_none()
            {
                issues.push(Issue::Identifier(ident.target));
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
