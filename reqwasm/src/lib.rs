//! The req checker, formatter and HTML export for the browser.
//!
//! `check`, `format` and `exportHtml` do what `reqtool check`, `reqtool fmt` and `reqtool export` do, on
//! a string instead of a file. Only the checker's analysis is a copy: the export is `reqlang`'s own. The analysis below is copied from `reqtool/src/main.rs` (`Command::Check`) and kept in sync
//! by hand: the documentation repository (`lang-doc`, `scripts/playground_parity.mjs`) compares this
//! build with `reqtool check --json` on every example, so a difference shows up there.
//!
//! The report is not the CLI's JSON: it is a flat list of diagnostics, which is what an editor needs.
//! Positions are the ones of `reqlang`: `index` is a byte offset into the UTF-8 source.

use reqlang::{
    Specification, exporter,
    issue::{self, code::codes},
    renderer::formatter,
    syntax::{location::Position, parser},
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

impl From<Position> for Pos {
    fn from(p: Position) -> Self {
        Pos {
            line: p.line,
            column: p.column,
            index: p.index,
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    /// `syntax`, or the severity of an issue: `light`, `moderate`, `severe`, `critical`
    pub severity: &'static str,
    /// The code of the issue, `E04010`
    pub code: String,
    /// What the issue is about: `syntax`, `path`, `reference`, `typing` or `connection`
    pub kind: &'static str,
    pub message: String,
    /// The entity the issue is about (`System::Deceleration_bound`), none for a syntax error
    pub path: Option<String>,
    pub start: Pos,
    pub end: Pos,
    /// The source the diagnostic points at
    pub snippet: String,
}

fn severity_name(severity: issue::Severity) -> &'static str {
    match severity {
        issue::Severity::Critical => "critical",
        issue::Severity::Severe => "severe",
        issue::Severity::Moderate => "moderate",
        issue::Severity::Light => "light",
    }
}

/// Every syntax error, then every issue, in the order of `reqtool check --json`
pub fn diagnostics(source: &str) -> Vec<Diagnostic> {
    let (tree, info) = parser::parse(source);

    let mut found: Vec<Diagnostic> = info
        .errors
        .iter()
        .map(|span| {
            let end = span.end.index.min(source.len());
            let start = span.start.index.min(end);
            Diagnostic {
                severity: "syntax",
                code: codes::SYNTAX_ERROR.to_string(),
                kind: codes::SYNTAX_ERROR.kind().as_str(),
                message: "syntax error".to_string(),
                path: None,
                start: span.start.into(),
                end: span.end.into(),
                snippet: source[start..end].to_string(),
            }
        })
        .collect();

    let mut spec = Specification::from(tree);
    spec.diagnose();
    let issues = issue::issues(&spec);

    for ((idx, issues), path) in issues
        .indices()
        .zip(issues)
        .into_iter()
        .zip(spec.analysis.paths)
    {
        for issue in issues {
            let span = issue.span(idx, &info);
            found.push(Diagnostic {
                severity: severity_name(issue.severity()),
                code: issue.code().to_string(),
                kind: issue.kind().as_str(),
                message: issue.to_string(),
                path: Some(path.path.to_string()),
                start: span.start.into(),
                end: span.end.into(),
                snippet: source[span.range()].to_string(),
            });
        }
    }

    found
}

/// The formatted source, or why it cannot be formatted
pub fn format_source(source: &str) -> Result<String, String> {
    let (tree, info) = parser::parse(source);
    // The tree is incomplete when there are syntax errors, formatting it would lose code
    if !info.errors.is_empty() {
        return Err("the source has syntax errors".to_string());
    }
    Ok(formatter::format_string(&tree, 0))
}

/// The specification as an HTML fragment, `reqtool export`'s output. A source with syntax errors is
/// exported as far as the parser understood it, so a specification being typed still shows.
pub fn export_source(source: &str) -> String {
    let (tree, _) = parser::parse(source);
    exporter::html::render_string(&Specification::from(tree))
}

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// The diagnostics of `source`, as an array of plain objects
/// `{ severity, code, kind, message, path, start: {line, column, index}, end, snippet }`
#[wasm_bindgen]
pub fn check(source: &str) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    diagnostics(source)
        .serialize(&serializer)
        .map_err(JsValue::from)
}

/// `source`, formatted. Throws a string when the source has syntax errors.
#[wasm_bindgen]
pub fn format(source: &str) -> Result<String, JsValue> {
    format_source(source).map_err(|e| JsValue::from_str(&e))
}

/// The HTML fragment of `source` (headings, text, code), which links its references to `#pv-<n>` anchors
#[wasm_bindgen(js_name = exportHtml)]
pub fn export_html(source: &str) -> String {
    export_source(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    // source/tutorials/examples/diagnostics/type-mismatch.req of the documentation
    const TYPE_MISMATCH: &str = "\
package System
  part Wheel
    let deceleration_duration in real \\ s \\
  part

  requirement Wheel_deceleration top
    on Wheel
  is
    @@
      The [[Wheel]] shall decelerate within 3s.
    @@
  requirement

  requirement Deceleration_bound bottom
    on Wheel::deceleration_duration
    refines Wheel_deceleration
  is
    Wheel::deceleration_duration < false
  requirement
package
";

    #[test]
    fn reports_a_type_error_where_the_cli_does() {
        let found = diagnostics(TYPE_MISMATCH);
        assert_eq!(
            found,
            vec![Diagnostic {
                severity: "critical",
                code: "E03010".to_string(),
                kind: "typing",
                message: "expected type Number got Boolean".to_string(),
                path: Some("System::Deceleration_bound".to_string()),
                start: Pos {
                    line: 17,
                    column: 4,
                    index: 328
                },
                end: Pos {
                    line: 17,
                    column: 40,
                    index: 364
                },
                snippet: "Wheel::deceleration_duration < false".to_string(),
            }]
        );
    }

    #[test]
    fn a_clean_specification_has_no_diagnostic() {
        let clean = TYPE_MISMATCH.replace("< false", "< 3");
        assert_eq!(diagnostics(&clean), vec![]);
    }

    // A part without a name: `reqtool check --json` reports one syntax error, on `part`
    const SYNTAX_ERROR: &str = "package System\n  part\npackage\n";

    #[test]
    fn reports_a_syntax_error_without_a_path() {
        assert_eq!(
            diagnostics(SYNTAX_ERROR),
            vec![Diagnostic {
                severity: "syntax",
                code: "E00010".to_string(),
                kind: "syntax",
                message: "syntax error".to_string(),
                path: None,
                start: Pos {
                    line: 1,
                    column: 2,
                    index: 17
                },
                end: Pos {
                    line: 1,
                    column: 6,
                    index: 21
                },
                snippet: "part".to_string(),
            }]
        );
    }

    #[test]
    fn formatting_refuses_a_source_with_syntax_errors() {
        assert!(format_source(SYNTAX_ERROR).is_err());
    }

    #[test]
    fn formatting_a_valid_source_keeps_it_valid() {
        let formatted = format_source(TYPE_MISMATCH).unwrap();
        assert!(diagnostics(&formatted).len() == diagnostics(TYPE_MISMATCH).len());
    }

    #[test]
    fn exports_the_references_as_links() {
        let html = export_source(TYPE_MISMATCH);
        assert!(
            html.contains(r##"The <a href="#pv-1">Wheel</a> shall decelerate within 3s."##),
            "{html}"
        );
        assert!(html.contains(r#"<h3 id="pv-1">Wheel</h3>"#), "{html}");
    }

    #[test]
    fn exports_a_source_with_syntax_errors_as_far_as_it_was_understood() {
        assert!(export_source(SYNTAX_ERROR).contains("<h2 id="));
    }

    #[test]
    fn exports_nothing_for_an_empty_source() {
        assert_eq!(export_source(""), "");
    }
}
