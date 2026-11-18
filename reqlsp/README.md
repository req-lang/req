# reqlsp

Language Server (LSP 3.18) for the [req language](https://req-lang.org), built on `tower-lsp` and the `reqlang` compiler crate.

Spec: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/

## Compiler API used

- `reqlang::syntax::parser::parse(&str) -> (Tree, Spans, Vec<Span>)`: tree, source spans per entity/expression, syntax errors.
- `Specification::from(tree)` then `.diagnose()`: semantic analysis.
- `reqlang::issue::issues(&spec)`: `Vec<Issue>` per entity (`Reference`, `Type`, `Connection`, `Path`), with `Severity` (`Light`..`Critical`). Issues carry no span; anchor them on the entity span from `Spans.entities`.

Beware: compiler `Position` is 1-based (line, column, byte index); LSP is 0-based and UTF-16 by default.

## TODO

### 1. Lifecycle and document sync
- [ ] Advertise `text_document_sync: TextDocumentSyncKind::FULL` in `initialize`
- [ ] Implement `did_open`, `did_change`, `did_close`
- [ ] Store documents (`DashMap<Url, Document>` or `RwLock<HashMap<..>>`); `Document` holds text, then `Tree`, `Spans`, `Specification`

### 2. Position conversion (before diagnostics)
- [ ] `to_lsp_position(text, byte_index) -> Position` (UTF-16 units within the line)
- [ ] `to_byte_index(text, Position) -> usize` (needed for hover/definition)
- [ ] Unit tests with non-ASCII input
- [ ] Check whether `reqlang`'s `column` counts chars or bytes; prefer converting from `Position.index`
- [ ] Optional: negotiate `positionEncoding: utf-8` when the client supports it

### 3. Diagnostics
- [ ] On open/change: parse, analyse, then `client.publish_diagnostics(uri, diags, version)`
- [ ] Syntax error `Span` -> `Diagnostic` (`ERROR`, `source: "reqlang"`)
- [ ] Semantic `Issue` -> `Diagnostic` on the entity span (`Critical`/`Severe` -> `ERROR`, `Moderate` -> `WARNING`, `Light` -> `HINT`/`INFORMATION`); message from `Display`
- [ ] Publish an empty list on close to clear stale diagnostics

### 4. Document symbols
- [ ] `textDocument/documentSymbol`: walk the entity tree from `Tree::root()`, return nested `DocumentSymbol`s

### 5. Hover
- [ ] Position -> byte index -> innermost entity/expression span
- [ ] Show entity path, kind, and expression type from `analysis.types`

### 6. Go to definition / references
- [ ] `textDocument/definition` via `analysis.links.references` and `links.identifiers`
- [ ] `textDocument/references`

### 7. Later
- [ ] Formatting via `renderer::formatter::format_string`
- [ ] Completion
- [ ] Incremental sync
- [ ] Debounce re-analysis
- [ ] Semantic tokens

### Open questions
- [ ] Is `parse` fast enough to run on every keystroke? (see `reqlang/benches`)
- [ ] Add a unified `Diagnostic { span, severity, message }` list to `reqlang`, or build it in `reqlsp`?
