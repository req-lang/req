//! The req checker and formatter for the browser.
//!
//! `check` and `format` do what `reqtool check` and `reqtool fmt` do, on a string instead of a
//! file. The analysis below is copied from `reqtool/src/main.rs` (`Command::Check`) and kept in sync
//! by hand: the documentation repository (`lang-doc`, `scripts/playground_parity.mjs`) compares this
//! build with `reqtool check --json` on every example, so a difference shows up there.
//!
//! The report is not the CLI's JSON: it is a flat list of diagnostics, which is what an editor needs.
//! Positions are the ones of `reqlang`: `index` is a byte offset into the UTF-8 source.

use reqlang::{
    Specification, issue,
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
    pub message: String,
    /// The entity the issue is about (`System::Deceleration_bound`), none for a syntax error
    pub path: Option<String>,
    pub start: Pos,
    pub end: Pos,
    /// The source the diagnostic points at
    pub code: String,
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
                message: "syntax error".to_string(),
                path: None,
                start: span.start.into(),
                end: span.end.into(),
                code: source[start..end].to_string(),
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
                message: issue.to_string(),
                path: Some(path.path.to_string()),
                start: span.start.into(),
                end: span.end.into(),
                code: source[span.range()].to_string(),
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

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// The diagnostics of `source`, as an array of plain objects
/// `{ severity, message, path, start: {line, column, index}, end, code }`
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
      The {Wheel} shall decelerate within 3s.
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
                message: "expected type Number got Boolean".to_string(),
                path: Some("System::Deceleration_bound".to_string()),
                start: Pos { line: 17, column: 4, index: 326 },
                end: Pos { line: 17, column: 40, index: 362 },
                code: "Wheel::deceleration_duration < false".to_string(),
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
                message: "syntax error".to_string(),
                path: None,
                start: Pos { line: 1, column: 2, index: 17 },
                end: Pos { line: 1, column: 6, index: 21 },
                code: "part".to_string(),
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
}
