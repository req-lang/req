use std::collections::HashMap;

use reqlang::{
    Specification,
    syntax::{self},
};
use tower_lsp::lsp_types::Position;
use tower_lsp::lsp_types::*;

use crate::{description, document::completion::CompletionKind};
use crate::{
    description::kind_name,
    location::{self, Contains, Target},
};
use crate::{diagnostics::collect_diagnostics, document::completion::completion_kind};

mod completion;
pub mod semantic_tokens;

#[derive(Debug)]
struct Document {
    uri: Url,
    spec: Specification,
    syntax: syntax::parser::Info,
    version: i32,
    end: Position,
}

impl Document {
    pub fn definition(&self, pos: Position) -> Option<Location> {
        let pos = location::Position::from(pos);
        let (target, _) = pos.target(&self.syntax)?;
        let resolved = target.resolve(&self.spec.analysis.links)?;
        let def = resolved.definition(&self.syntax);
        Some(Location::new(self.uri.clone(), def.into()))
    }

    pub fn references(&self, pos: Position, include_declaration: bool) -> Option<Vec<Location>> {
        let pos = location::Position::from(pos);
        let (target, _) = pos.target(&self.syntax)?;
        if let Target::Entity(idx) = target
            && let Some(name) = self.syntax.entities.name_of(idx).map(location::Span::from)
            && !name.contains(pos)
        {
            return None;
        }

        let resolved = target.resolve(&self.spec.analysis.links)?;
        let mut refs = resolved.references(&self.syntax, &self.spec.analysis.links);
        if include_declaration {
            refs.push(resolved.definition(&self.syntax));
        }

        refs.sort_by_key(|span| (span.start.line, span.start.column));
        let locs = refs
            .into_iter()
            .map(|span| Location::new(self.uri.clone(), span.into()))
            .collect();
        Some(locs)
    }

    pub fn hover(&self, pos: Position) -> Option<Hover> {
        let pos = location::Position::from(pos);
        let (found, span) = pos.target(&self.syntax)?;
        let value = description::render(&self.spec, &found);
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(span.into()),
        })
    }

    pub fn symbols(&self) -> Vec<DocumentSymbol> {
        let entities = &self.spec.tree.entities;
        if entities.is_empty() {
            return Vec::new();
        }
        self.symbol(self.spec.tree.root()).into_iter().collect()
    }

    fn symbol(&self, idx: reqlang::node::entity::Idx) -> Option<DocumentSymbol> {
        use reqlang::utils::children::ChildrenIter;

        let node = &self.spec.tree.entities[idx];
        let range = self.syntax.entities.span_of(idx);
        let selection_range = self
            .syntax
            .entities
            .name_of(idx)
            .unwrap_or(self.syntax.entities.span_of(idx));
        let children = node
            .variant
            .children_iter()
            .filter_map(|child| self.symbol(child))
            .collect::<Vec<_>>();

        use reqlang::node::entity::Variant::*;

        #[allow(deprecated)]
        Some(DocumentSymbol {
            name: node.meta.label.clone(),
            detail: None,
            kind: match node.variant {
                Part(_) => SymbolKind::OBJECT,
                Package(_) => SymbolKind::MODULE,
                Requirement(_) => SymbolKind::FUNCTION,
                Attribute(_) => SymbolKind::FIELD,
                Type(_) => SymbolKind::ENUM,
            },
            tags: None,
            deprecated: None,
            range: location::Span::from(range).into(),
            selection_range: location::Span::from(selection_range).into(),
            children: (!children.is_empty()).then_some(children),
        })
    }

    pub fn semantic_tokens(&self) -> SemanticTokens {
        let toks = semantic_tokens::collect(&self.syntax, &self.spec);
        SemanticTokens {
            result_id: None,
            data: semantic_tokens::encode(toks),
        }
    }

    pub fn completion(&self, pos: Position) -> Option<Vec<CompletionItem>> {
        let entities = &self.spec.tree.entities;
        let paths = &self.spec.analysis.paths;

        let pos = location::Position::from(pos);
        let scope = match pos.innermost(&self.syntax.entities) {
            Some((idx, _)) => completion::scope(&self.spec, idx),
            None => Vec::new(),
        };
        let ctx = completion::context(&self.syntax, pos)?;
        let completions = completion::candidates(&self.spec, &scope, &ctx.kind);
        let span = ctx.span;
        let completions = completions
            .into_iter()
            .map(|(path, kind)| match kind {
                CompletionKind::Entity(idx) => {
                    let node = &entities[idx];
                    let full = &paths[idx].path;
                    let detail = format!("{} {full}", kind_name(&node.variant));
                    let kind = completion_kind(&node.variant);
                    item(path, kind, Some(detail), span.into())
                }
                CompletionKind::Enum(_, _) => {
                    let kind = CompletionItemKind::ENUM_MEMBER;
                    item(path, kind, None, span.into())
                }
                CompletionKind::Builtin(_) => {
                    item(path, CompletionItemKind::CLASS, None, span.into())
                }
            })
            .collect();
        Some(completions)
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        collect_diagnostics(&self.spec, &self.syntax)
            .into_iter()
            .map(|v| v.into())
            .collect()
    }

    pub fn new(uri: Url, text: &str, version: i32) -> Self {
        let (tree, syntax) = reqlang::syntax::parser::parse(text);
        let mut spec = reqlang::Specification::from(tree);
        spec.diagnose();
        let last_line = text.rsplit('\n').next().unwrap_or_default();
        let end = Position::new(
            text.matches('\n').count() as u32,
            last_line.chars().count() as u32,
        );
        Self {
            uri,
            spec,
            version,
            syntax,
            end,
        }
    }

    pub fn format(&self) -> Option<Vec<TextEdit>> {
        if !self.syntax.errors.is_empty() {
            return None;
        }
        let text = reqlang::renderer::formatter::format_string(&self.spec.tree, 0);
        let range = Range::new(Position::new(0, 0), self.end);
        Some(vec![TextEdit::new(range, text)])
    }
}

#[derive(Default, Debug)]
pub struct DocumentManager {
    opened: tokio::sync::RwLock<HashMap<Url, Document>>,
}

impl DocumentManager {
    pub async fn insert(&self, uri: &Url, text: &str, version: i32) {
        let doc = Document::new(uri.clone(), text, version);
        let mut specs = self.opened.write().await;
        if specs.get(uri).is_some_and(|d| d.version > version) {
            return;
        }
        specs.insert(uri.clone(), doc);
    }

    pub async fn diagnostics(&self, uri: &Url) -> Vec<Diagnostic> {
        let mut specs = self.opened.write().await;
        let spec = specs.get_mut(uri).unwrap();
        spec.diagnostics()
    }

    pub async fn symbols(&self, uri: &Url) -> Option<Vec<DocumentSymbol>> {
        self.opened.read().await.get(uri).map(Document::symbols)
    }

    pub async fn format(&self, uri: &Url) -> Option<Vec<TextEdit>> {
        self.opened.read().await.get(uri)?.format()
    }

    pub async fn definition(&self, uri: &Url, pos: Position) -> Option<Location> {
        self.opened.read().await.get(uri)?.definition(pos)
    }

    pub async fn references(
        &self,
        uri: &Url,
        pos: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        self.opened
            .read()
            .await
            .get(uri)?
            .references(pos, include_declaration)
    }

    pub async fn completion(&self, uri: &Url, pos: Position) -> Option<Vec<CompletionItem>> {
        self.opened.read().await.get(uri)?.completion(pos)
    }

    pub async fn semantic_tokens(&self, uri: &Url) -> Option<SemanticTokens> {
        Some(self.opened.read().await.get(uri)?.semantic_tokens())
    }

    pub async fn hover(&self, uri: &Url, pos: Position) -> Option<Hover> {
        self.opened.read().await.get(uri)?.hover(pos)
    }

    pub async fn remove(&self, uri: &Url) {
        self.opened.write().await.remove(uri);
    }
}

pub fn item(
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    range: Range,
) -> CompletionItem {
    CompletionItem {
        filter_text: Some(label.clone()),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
            range,
            label.clone(),
        ))),
        label,
        kind: Some(kind),
        detail,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "package A
  part B
    @@
    Power draw
    @@
    let x in real
  part
  requirement R1
    on B
  is
    B::x > 10
  requirement
package
";

    fn document() -> Document {
        Document::new(uri(), SOURCE, 0)
    }

    fn uri() -> Url {
        Url::parse("file:///a.req").unwrap()
    }

    /// The position of the `n`th character of the first occurrence of `needle` after `line`
    fn at(line: u32, needle: &str, n: usize) -> Position {
        let text = SOURCE.lines().nth(line as usize).unwrap();
        let column = text.find(needle).unwrap() + n;
        Position::new(line, column as u32)
    }

    fn found(doc: &Document, pos: Position) -> Option<Target> {
        location::Position::from(pos)
            .target(&doc.syntax)
            .map(|(found, _)| found)
    }

    #[test]
    fn entity_name() {
        let doc = document();
        assert!(matches!(
            found(&doc, at(7, "R1", 0)),
            Some(Target::Entity(_))
        ));
    }

    #[test]
    fn identifier_wins_over_its_expression() {
        let doc = document();
        assert!(matches!(
            found(&doc, at(10, "B::x", 0)),
            Some(Target::Identifier(_))
        ));
        assert!(matches!(
            found(&doc, at(10, "B::x", 3)),
            Some(Target::Identifier(_))
        ));
    }

    #[test]
    fn identifier_hover_has_type_and_target() {
        let doc = document();
        let hover = doc.hover(at(10, "B::x", 0)).unwrap();
        let HoverContents::Markup(content) = hover.contents else {
            panic!("expected markup");
        };
        assert!(content.value.contains("attribute"), "{}", content.value);
        assert!(content.value.contains("`A::B::x`"), "{}", content.value);
        // The type of the attribute is the one of the elements of its domain
        assert!(
            content.value.contains("type: `Number`"),
            "{}",
            content.value
        );
    }

    #[test]
    fn operator_and_literal_are_expressions() {
        let doc = document();
        assert!(matches!(
            found(&doc, at(10, ">", 0)),
            Some(Target::Expression(_))
        ));
        assert!(matches!(
            found(&doc, at(10, "10", 1)),
            Some(Target::Expression(_))
        ));
    }

    #[test]
    fn end_of_span_is_excluded() {
        let doc = document();
        // Right after `B::x`: the identifier is over, the comparison goes on
        assert!(matches!(
            found(&doc, at(10, "B::x", 4)),
            Some(Target::Expression(_))
        ));
    }

    #[test]
    fn definition_of_a_reference_is_the_entity_name() {
        let doc = document();
        let location = doc.definition(at(8, "B", 0)).unwrap();
        assert_eq!(location.uri, uri());
        // `part B`, on the name only
        let start = at(1, "B", 0);
        assert_eq!(
            location.range,
            Range::new(start, Position::new(start.line, start.character + 1))
        );
    }

    #[test]
    fn definition_of_an_identifier_is_the_attribute_name() {
        let doc = document();
        let location = doc.definition(at(10, "B::x", 0)).unwrap();
        let start = at(5, "x", 0);
        assert_eq!(
            location.range,
            Range::new(start, Position::new(start.line, start.character + 1))
        );
    }

    fn lines(locations: Vec<Location>) -> Vec<u32> {
        locations.iter().map(|l| l.range.start.line).collect()
    }

    #[test]
    fn references_of_an_entity_from_its_name() {
        let doc = document();
        // `part B` is used by `on B`
        let without = doc.references(at(1, "B", 0), false).unwrap();
        assert_eq!(lines(without), vec![8]);
        let with = doc.references(at(1, "B", 0), true).unwrap();
        assert_eq!(lines(with), vec![1, 8]);
    }

    #[test]
    fn references_from_a_usage() {
        let doc = document();
        // `B::x` is the usage of the attribute `x`, declared in `let x`
        let from_reference = doc.references(at(8, "B", 0), true).unwrap();
        assert_eq!(lines(from_reference), vec![1, 8]);
        let from_identifier = doc.references(at(10, "B::x", 0), true).unwrap();
        assert_eq!(lines(from_identifier), vec![5, 10]);
    }

    #[test]
    fn inside_an_entity_only() {
        let doc = document();
        assert!(matches!(
            found(&doc, Position::new(9, 3)),
            Some(Target::Entity(_))
        ));
        assert_eq!(found(&doc, Position::new(100, 0)), None);
    }

    #[test]
    fn format_replaces_the_whole_document() {
        let source = "package   A\n      part B\n  part\npackage\n";
        let doc = Document::new(uri(), source, 0);
        let edits = doc.format().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(
            edits[0].range,
            Range::new(Position::new(0, 0), Position::new(4, 0))
        );
        assert_eq!(edits[0].new_text, "package A\n  part B\n  part\npackage\n");
    }

    #[test]
    fn format_ends_on_the_last_line_without_a_final_newline() {
        let doc = Document::new(uri(), "package A\npackage", 0);
        assert_eq!(doc.format().unwrap()[0].range.end, Position::new(1, 7));
    }

    #[test]
    fn format_leaves_a_document_with_errors_alone() {
        let doc = Document::new(uri(), "package A\n  requirement R\n    on\npackage\n", 0);
        assert!(!doc.syntax.errors.is_empty());
        assert!(doc.format().is_none());
    }

    #[test]
    fn format_is_the_identity_on_formatted_documents() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../reqlang/tests/data/valid");
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if !path.is_file() {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let doc = Document::new(uri(), &source, 0);
            let edits = doc.format().unwrap_or_else(|| panic!("{path:?}"));
            assert_eq!(edits[0].new_text, source, "{path:?}");
        }
    }

    #[test]
    fn no_panic_on_broken_input() {
        for source in ["", "package A\n  let in\n", "package", "part x y z\n@@ @@"] {
            let doc = Document::new(uri(), source, 0);
            for line in 0..4 {
                for character in 0..20 {
                    let _ = doc.hover(Position::new(line, character));
                }
            }
        }
    }
}
