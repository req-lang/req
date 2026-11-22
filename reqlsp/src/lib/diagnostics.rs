use reqdiag::Diagnostic;
use tower_lsp::lsp_types;

pub fn from_reqdiag_diagnostic(value: Diagnostic) -> lsp_types::Diagnostic {
    let span = from_diagnostic_span(&value.span);
    let mut diagnostic = lsp_types::Diagnostic::new_simple(span, value.message);
    diagnostic.severity = Some(from_semantic_severity(value.severity));
    diagnostic.code = Some(lsp_types::NumberOrString::String(value.code));
    diagnostic.source = Some(SOURCE.into());
    // The only tags of the protocol are `Unnecessary` and `Deprecated`, what the issue is about
    // goes in the data, for the clients that group or filter the diagnostics
    diagnostic.data = Some(serde_json::json!({ "kind": value.kind }));

    diagnostic
}

/// What the diagnostics say they come from
const SOURCE: &str = "req";

fn from_semantic_severity(value: &str) -> lsp_types::DiagnosticSeverity {
    use lsp_types::DiagnosticSeverity;

    match value {
        "light" => DiagnosticSeverity::HINT,
        "moderate" => DiagnosticSeverity::INFORMATION,
        "severe" => DiagnosticSeverity::WARNING,
        "critical" => DiagnosticSeverity::ERROR,
        _ => unreachable!(),
    }
}

fn from_diagnostic_span(value: &reqdiag::Span) -> lsp_types::Range {
    lsp_types::Range {
        start: lsp_types::Position::new(value.start.line, value.start.column),
        end: lsp_types::Position::new(value.end.line, value.end.column),
    }
}

#[cfg(test)]
mod tests {
    use reqlang::{Specification, syntax::parser};

    use super::*;

    fn diagnostics_of(source: &str) -> Vec<lsp_types::Diagnostic> {
        let (tree, info) = parser::parse(source);
        let mut spec = Specification::from(tree);
        spec.diagnose();

        Diagnostic::collect(&spec, &info, source)
            .into_iter()
            .map(from_reqdiag_diagnostic)
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
        assert_eq!(
            diagnostic.range,
            lsp_types::Range::new(
                lsp_types::Position::new(1, 2),
                lsp_types::Position::new(1, 6)
            )
        );
    }
}
