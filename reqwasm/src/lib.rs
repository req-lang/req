use reqdiag::Diagnostic;
use reqexport::html;
use reqlang::{Specification, renderer::formatter, syntax::parser};
use serde::Serialize;
use wasm_bindgen::prelude::*;

pub fn diagnostics(source: &str) -> Vec<Diagnostic> {
    let (tree, info) = parser::parse(source);
    let mut spec = Specification::from(tree);
    spec.diagnose();
    Diagnostic::collect(&spec, &info, &source)
}

pub fn format_source(source: &str) -> Result<String, String> {
    let (tree, info) = parser::parse(source);
    // The tree is incomplete when there are syntax errors, formatting it would lose code
    if !info.errors.is_empty() {
        return Err("the source has syntax errors".to_string());
    }
    Ok(formatter::format_string(&tree, 0))
}

pub fn export_source(source: &str) -> String {
    let (tree, _) = parser::parse(source);
    html::render_string(&Specification::from(tree))
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
