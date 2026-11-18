use reqlang::{Specification, syntax};
use tower_lsp::lsp_types;

use crate::{
    diagnostics::DiagnosticKind::{Semantic, Syntax},
    location,
};

pub fn collect_diagnostics(spec: &Specification, info: &syntax::parser::Info) -> Vec<Diagnostic> {
    let syntax = info.errors.iter().cloned().map(Diagnostic::from);
    let issues = reqlang::issue::issues(&spec);
    let semantics = issues
        .indices()
        .into_iter()
        .zip(issues)
        .map(|(idx, issues)| {
            issues.into_iter().map(move |issue| {
                let span = issue.span(idx, info);
                Diagnostic::from_semantic(span, issue)
            })
        })
        .flatten();

    syntax.chain(semantics).collect()
}

pub enum DiagnosticKind {
    Syntax,
    Semantic(reqlang::issue::Issue),
}

pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: location::Span,
}

impl Diagnostic {
    fn from_semantic(span: reqlang::syntax::location::Span, value: reqlang::issue::Issue) -> Self {
        Self {
            kind: Semantic(value),
            span: span.into(),
        }
    }
}

impl From<reqlang::syntax::location::Span> for Diagnostic {
    fn from(value: reqlang::syntax::location::Span) -> Self {
        Self {
            kind: Syntax,
            span: value.into(),
        }
    }
}

impl From<Diagnostic> for lsp_types::Diagnostic {
    fn from(value: Diagnostic) -> Self {
        use DiagnosticKind::*;

        let message = match &value.kind {
            Syntax => "syntax error".into(),
            Semantic(issue) => issue.to_string(),
        };

        let severity = match &value.kind {
            Semantic(kind) => from_semantic_severity(&kind.severity()),
            Syntax => lsp_types::DiagnosticSeverity::ERROR,
        };

        let mut diagnostic = lsp_types::Diagnostic::new_simple(value.span.into(), message);
        diagnostic.severity = Some(severity);

        diagnostic
    }
}

fn from_semantic_severity(value: &reqlang::issue::Severity) -> lsp_types::DiagnosticSeverity {
    use lsp_types::DiagnosticSeverity;
    use reqlang::issue::Severity::*;

    match value {
        Light => DiagnosticSeverity::HINT,
        Moderate => DiagnosticSeverity::INFORMATION,
        Severe => DiagnosticSeverity::WARNING,
        Critical => DiagnosticSeverity::ERROR,
    }
}
