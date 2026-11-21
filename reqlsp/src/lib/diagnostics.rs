use reqlang::{Specification, issue::code::codes, syntax};
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

        let code = match &value.kind {
            Syntax => codes::SYNTAX_ERROR,
            Semantic(issue) => issue.code(),
        };

        let mut diagnostic = lsp_types::Diagnostic::new_simple(value.span.into(), message);
        diagnostic.severity = Some(severity);
        diagnostic.code = Some(lsp_types::NumberOrString::String(code.to_string()));
        diagnostic.source = Some(SOURCE.into());
        // The only tags of the protocol are `Unnecessary` and `Deprecated`, what the issue is about
        // goes in the data, for the clients that group or filter the diagnostics
        diagnostic.data = Some(serde_json::json!({ "kind": code.kind().as_str() }));

        diagnostic
    }
}

/// What the diagnostics say they come from
const SOURCE: &str = "req";

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

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostics_of(source: &str) -> Vec<lsp_types::Diagnostic> {
        let (tree, info) = syntax::parser::parse(source);
        let mut spec = Specification::from(tree);
        spec.diagnose();

        collect_diagnostics(&spec, &info)
            .into_iter()
            .map(Into::into)
            .collect()
    }

    fn with_code<'a>(
        diagnostics: &'a [lsp_types::Diagnostic],
        code: &str,
    ) -> Option<&'a lsp_types::Diagnostic> {
        diagnostics
            .iter()
            .find(|d| d.code == Some(lsp_types::NumberOrString::String(code.to_string())))
    }

    #[test]
    fn an_issue_carries_its_code_and_its_kind() {
        // an attribute that is not in a part
        let found = diagnostics_of("package p\n  let x in real\npackage\n");

        let diagnostic = with_code(&found, "E04060").expect("E04060 is reported");
        assert_eq!(diagnostic.source.as_deref(), Some("req"));
        assert_eq!(
            diagnostic.data,
            Some(serde_json::json!({ "kind": "connection" }))
        );
    }

    #[test]
    fn a_syntax_error_carries_the_syntax_code() {
        // a part without a name
        let found = diagnostics_of("package System\n  part\npackage\n");

        let diagnostic = with_code(&found, "E00010").expect("E00010 is reported");
        assert_eq!(
            diagnostic.severity,
            Some(lsp_types::DiagnosticSeverity::ERROR)
        );
        assert_eq!(
            diagnostic.data,
            Some(serde_json::json!({ "kind": "syntax" }))
        );
    }
}
