use reqlang::{
    Specification,
    node::{entity, path::Path},
    semantics::link::BuiltinKind,
    syntax::{
        self,
        tokenizer::{
            entity::{Keyword, Token, TokenVariant},
            expression, markup,
        },
    },
};
use tower_lsp::lsp_types::*;

use crate::location::{self, Contains};

pub enum ContextKind {
    Reference,
    Operand,
}

pub struct Context {
    pub kind: ContextKind,
    pub span: location::Span,
}

fn takes_reference(keyword: Keyword) -> bool {
    use Keyword::*;

    matches!(keyword, On | Via | Refines | Specializes | Derives | Import)
}

fn expects_operand(kind: expression::Kind) -> bool {
    use expression::{Keyword::*, Kind, Operator, Punctuation::*};

    match kind {
        Kind::Operator(operator) => operator != Operator::Factorial,
        Kind::Punctuation(punctuation) => matches!(
            punctuation,
            LeftParen | LeftBrace | LeftBracket | DoubleLeftBracket | Comma
        ),
        Kind::Keyword(keyword) => matches!(
            keyword,
            And | Or
                | Xor
                | Implies
                | Iff
                | In
                | Tol
                | Union
                | Intersection
                | Difference
                | Complement
                | Includes
                | Since
                | Not
                | Previously
                | Rising
                | Falling
                | Eventually
                | Always
                | When
                | Then
                | Otherwise
                | That
                | All
                | Any
                | One
                | Max
                | Min
                | Sum
                | Prod
                | ArgMax
                | ArgMin
                | Sup
                | Inf
                | Count
                | Choose
        ),
        _ => false,
    }
}

pub fn completion_kind(variant: &entity::Variant) -> CompletionItemKind {
    use entity::Variant::*;

    match variant {
        Package(_) => CompletionItemKind::MODULE,
        Part(_) => CompletionItemKind::CLASS,
        Requirement(_) => CompletionItemKind::FUNCTION,
        Attribute(_) => CompletionItemKind::FIELD,
        Type(_) => CompletionItemKind::ENUM,
    }
}

fn spelling(full: &Path, scope: &[&Path]) -> String {
    for prefix in scope
        .into_iter()
        .map(|s| s.as_str())
        .filter(|prefix| !prefix.is_empty())
    {
        if let Some(rest) = full
            .as_str()
            .strip_prefix(prefix)
            .and_then(|r| r.strip_prefix("::"))
        {
            return rest.to_string();
        }
    }
    full.to_string()
}

#[allow(dead_code)]
pub enum CompletionKind {
    Entity(entity::Idx),
    Enum(entity::Idx, usize),
    Builtin(BuiltinKind),
}

const ALL_BUILTIN: [BuiltinKind; 3] = [
    BuiltinKind::Real,
    BuiltinKind::Integer,
    BuiltinKind::Boolean,
];

pub fn candidates(
    spec: &Specification,
    scope: &[&Path],
    ctx: &ContextKind,
) -> Vec<(String, CompletionKind)> {
    let entities = &spec.tree.entities;
    let paths = &spec.analysis.paths;

    match ctx {
        ContextKind::Reference => paths
            .enumerate()
            .map(|(idx, loc)| {
                let label = spelling(&loc.path, &scope[..]);
                (label, CompletionKind::Entity(idx))
            })
            .collect(),
        ContextKind::Operand => {
            let ents = paths
                .enumerate()
                .filter(|(idx, _)| entities[*idx].variant.attribute().is_some())
                .map(|(idx, loc)| {
                    let label = spelling(&loc.path, &scope[..]);
                    (label, CompletionKind::Entity(idx))
                });
            let enums = paths
                .enumerate()
                .filter(|(idx, _)| entities[*idx].variant.typ().is_some())
                .flat_map(|(idx, loc)| {
                    let label = spelling(&loc.path, &scope[..]);
                    let typ = entities[idx].variant.typ().unwrap();
                    typ.members
                        .iter()
                        .enumerate()
                        .map(move |(x, m)| (format!("{label}::{m}"), CompletionKind::Enum(idx, x)))
                });
            let builtin = ALL_BUILTIN
                .iter()
                .map(|b| (b.as_str().to_string(), CompletionKind::Builtin(*b)));

            ents.chain(enums).chain(builtin).collect()
        }
    }
}

pub fn context(info: &syntax::parser::Info, pos: location::Position) -> Option<Context> {
    let tokens = info.entities.tokens();
    let (current, token) = pos.rightmost(tokens)?;
    let previous = current.checked_sub(1).map(|i| &tokens[i]);
    let span = span(info, pos, token, previous)?;
    let kind = token.variant.into();
    Some(Context { span, kind })
}

fn span(
    info: &syntax::parser::Info,
    pos: location::Position,
    current: &Token,
    previous: Option<&Token>,
) -> Option<location::Span> {
    let inside = pos <= location::Position::from(current.span.end);
    let outer = location::Span::from(current.span);
    match current.variant {
        TokenVariant::Expression => {
            let exprs = info.expression.tokens();
            let tokens: Vec<_> = exprs
                .iter()
                .filter(|tok| outer.contains(location::Span::from(tok.span)))
                .cloned()
                .collect();
            let Some((current, token)) = pos.rightmost(&tokens) else {
                return Some(location::Span::empty(pos));
            };

            use expression::Kind::*;

            let span = location::Span::from(token.span);
            let previous = current.checked_sub(1).map(|idx| tokens[idx].kind);
            match token.kind {
                Identifier | Keyword(_) if pos <= span.end.into() => match previous {
                    Some(kind) if expects_operand(kind) => Some(span.at_line_before(pos)),
                    None => Some(span.at_line_before(pos)),
                    _ => None,
                },
                _ if expects_operand(token.kind) => Some(location::Span::empty(pos)),
                _ => None,
            }
        }
        TokenVariant::Markup if inside => {
            let name = info
                .markups
                .tokens()
                .iter()
                .filter_map(markup::Token::name)
                .map(location::Span::from)
                .find(|name| outer.contains(*name) && name.includes(pos))?;
            Some(name.at_line_before(pos))
        }
        TokenVariant::Identifier if inside => match previous.map(|p| p.variant) {
            Some(TokenVariant::Reserved(keyword)) if takes_reference(keyword) => {
                Some(outer.at_line_before(pos))
            }
            _ => None,
        },
        TokenVariant::Reserved(keyword) if !inside && takes_reference(keyword) => {
            Some(location::Span::empty(pos))
        }
        TokenVariant::Reserved(Keyword::Is | Keyword::In) if !inside => {
            Some(location::Span::empty(pos))
        }
        _ => None,
    }
}

impl From<TokenVariant> for ContextKind {
    fn from(value: TokenVariant) -> Self {
        match value {
            TokenVariant::Expression => ContextKind::Operand,
            TokenVariant::Markup => ContextKind::Reference,
            TokenVariant::Identifier => ContextKind::Reference,
            TokenVariant::Reserved(keyword) if takes_reference(keyword) => ContextKind::Reference,
            TokenVariant::Reserved(Keyword::Is | Keyword::In) => ContextKind::Operand,
            _ => ContextKind::Reference,
        }
    }
}

pub fn scope(spec: &Specification, idx: entity::Idx) -> Vec<&Path> {
    use entity::Variant::*;
    let entities = &spec.tree.entities;
    let refs = &spec.tree.references;

    let scope = match entities[idx].variant {
        Requirement(_) | Attribute(_) if let Some(idx) = spec.analysis.neighbors[idx].parent => idx,
        _ => idx,
    };

    let imports = entities[scope]
        .variant
        .imports()
        .into_iter()
        .flatten()
        .map(|imp| &refs[*imp]);

    std::iter::once(&spec.analysis.paths[scope].path)
        .chain(imports)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    use super::*;

    // The text where the last `|` is the position of the cursor, and the position
    fn document(marked: &str) -> (Document, Position) {
        let index = marked.rfind('|').unwrap();
        let before = &marked[..index];
        let line = before.matches('\n').count() as u32;
        let column = before.rsplit('\n').next().unwrap().chars().count() as u32;
        let uri = Url::parse("file:///a.req").unwrap();
        let mut text = marked.to_string();
        text.remove(index);
        (Document::new(uri, &text, 0), Position::new(line, column))
    }

    // Sorted, the order of the entities in the tree is the one of the parser
    fn labels(items: &[CompletionItem]) -> Vec<&str> {
        let mut labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        labels.sort();
        labels
    }

    fn range(item: &CompletionItem) -> Range {
        match &item.text_edit {
            Some(CompletionTextEdit::Edit(edit)) => edit.range,
            other => panic!("expected an edit, got {other:?}"),
        }
    }

    fn complete(marked: &str) -> Option<Vec<CompletionItem>> {
        let (document, pos) = document(marked);
        document.completion(pos.into())
    }

    const REQUIREMENT: &str = "package A
  part B
    let x in real
  part
  requirement R
    on B
  is
    1 > 0
  requirement
package
";

    #[test]
    fn a_reference_is_expected_after_a_keyword_that_takes_one() {
        // The requirement is not complete, it is not in the tree
        let items = complete("package A\n  part B\n  part\n  requirement R\n    on |\n  is\n    1 > 0\n  requirement\npackage")
            .unwrap();
        assert_eq!(labels(&items), ["A", "B"]);

        // Nothing is typed yet
        let at = Position::new(4, 7);
        assert!(items.iter().all(|item| range(item) == Range::new(at, at)));
        let part = items.iter().find(|item| item.label == "B").unwrap();
        assert_eq!(part.filter_text.as_deref(), Some("B"));
        assert_eq!(part.detail.as_deref(), Some("part A::B"));
    }

    #[test]
    fn a_reference_that_is_being_typed_is_replaced_up_to_the_cursor() {
        let marked = REQUIREMENT.replace("on B", "on B|");
        let items = complete(&marked).unwrap();
        assert!(labels(&items).contains(&"B"));
        assert!(
            items
                .iter()
                .all(|item| range(item) == Range::new(Position::new(5, 7), Position::new(5, 8)))
        );

        // In the middle of the name, only what is before the cursor is replaced
        let marked = REQUIREMENT.replace("on B", "on |B");
        let items = complete(&marked).unwrap();
        assert!(
            items
                .iter()
                .all(|item| range(item) == Range::new(Position::new(5, 7), Position::new(5, 7)))
        );
    }

    #[test]
    fn names_are_written_from_the_scope_of_the_entity() {
        let marked = REQUIREMENT.replace("on B", "on B|");
        let items = complete(&marked).unwrap();
        // The requirement is read from the package, whose path is not written
        assert_eq!(labels(&items), ["A", "B", "B::x", "R"]);
    }

    #[test]
    fn a_name_is_not_completed_where_it_is_declared() {
        assert!(complete("package A\n  part |").is_none());
        assert!(complete("package A\n  part B|\n  part\npackage").is_none());
        assert!(complete("package A\n  requirement R |\n  is 1\n  requirement\npackage").is_none());
    }

    #[test]
    fn an_operand_is_expected_after_an_operator() {
        let marked = "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x > |\n  requirement\npackage";
        let items = complete(marked).unwrap();
        // The packages and the parts are not operands, nor is R: it is not complete
        assert_eq!(labels(&items), ["boolean", "integer", "real", "x"]);
        let at = Position::new(5, 8);
        assert!(items.iter().all(|item| range(item) == Range::new(at, at)));
    }

    #[test]
    fn an_operand_that_is_being_typed_is_replaced_up_to_the_cursor() {
        let marked = "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x > x|\n  requirement\npackage";
        let items = complete(marked).unwrap();
        assert_eq!(labels(&items), ["boolean", "integer", "real", "x"]);
        let range = range(&items[0]);
        assert_eq!(range, Range::new(Position::new(5, 8), Position::new(5, 9)));

        // Even if what is typed looks like a keyword
        let marked = "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x > in|\n  requirement\npackage";
        assert!(complete(marked).is_some());
    }

    #[test]
    fn the_first_operand_is_expected_at_the_start_of_the_expression() {
        let marked = "package A\n  let x in real\n  requirement R\n    on x\n  is\n    |\n  requirement\npackage";
        assert!(complete(marked).is_some());
        let marked = "package A\n  let x in |\npackage";
        let items = complete(marked).unwrap();
        assert!(labels(&items).contains(&"real"));
    }

    #[test]
    fn nothing_is_expected_after_an_operand() {
        for marked in [
            "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x |> 1\n  requirement\npackage",
            "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x > 1 |\n  requirement\npackage",
            "package A\n  let x in real\n  requirement R\n    on x\n  is\n    f(x)|\n  requirement\npackage",
            "package A\n  let x in real\n  requirement R\n    on x\n  is\n    x! |\n  requirement\npackage",
        ] {
            assert!(complete(marked).is_none(), "{marked}");
        }
    }

    #[test]
    fn the_members_of_a_type_are_operands() {
        let marked = "package S
  type T has
    A |
    B
  type
  let x in S::T
  requirement R
    on x
  is
    x = |
  requirement
package";
        let items = complete(marked).unwrap();
        assert_eq!(
            labels(&items),
            ["T::A", "T::B", "boolean", "integer", "real", "x"]
        );
    }

    #[test]
    fn a_reference_is_expected_between_the_brackets_of_a_text() {
        let marked = "package A
  part B
  part
  requirement R
  is
    @@
    see [[|]]
    @@
  requirement
package";
        let items = complete(marked).unwrap();
        assert_eq!(labels(&items), ["A", "B", "R"]);
        let at = Position::new(6, 10);
        assert!(items.iter().all(|item| range(item) == Range::new(at, at)));
    }

    #[test]
    fn a_reference_that_is_being_typed_in_a_text_is_replaced_up_to_the_cursor() {
        for (typed, start, end) in [
            ("see [[B|]]", 10, 11),
            ("see [[|B]]", 10, 10),
            ("see [[B]] and [[A::|B]]", 20, 23),
        ] {
            let marked = format!(
                "package A
  part B
  part
  requirement R
  is
    @@
    {typed}
    @@
  requirement
package"
            );
            let items = complete(&marked).unwrap();
            assert!(labels(&items).contains(&"B"), "{typed}");
            let expected = Range::new(Position::new(6, start), Position::new(6, end));
            assert!(items.iter().all(|item| range(item) == expected), "{typed}");
        }
    }

    #[test]
    fn a_reference_that_is_not_closed_is_completed_too() {
        for (typed, column) in [("see [[|", 10), ("see [[B|", 10), ("see [[a [[|", 14)] {
            let marked = format!(
                "package A
  part B
  part
  requirement R
  is
    @@
    {typed}
    @@
  requirement
package"
            );
            let items = complete(&marked).unwrap();
            assert!(labels(&items).contains(&"B"), "{typed}");
            let start = items.first().map(|item| range(item).start.character);
            assert_eq!(start, Some(column), "{typed}");
        }
    }

    #[test]
    fn nothing_is_expected_in_the_text_around_the_references() {
        for line in [
            "see| [[B]]",
            "see [|[B]]",
            "see [[B]]|",
            "see [[B]] and|",
            "see [|",
            "see [[a |b]]",
            "see [[a.|b]]",
            "see {|}",
            "see {B|",
            "x^{|2}",
        ] {
            let marked = format!(
                "package A\n  requirement R\n  is\n    @@\n    {line}\n    @@\n  requirement\npackage"
            );
            assert!(complete(&marked).is_none(), "{line}");
        }
    }

    #[test]
    fn a_reference_is_on_one_line() {
        let marked = "package A\n  requirement R\n  is\n    @@\n    see [[\n    B|\n    @@\n  requirement\npackage";
        assert!(complete(marked).is_none());
    }

    #[test]
    fn broken_or_empty_input_does_not_fail() {
        for marked in [
            "|",
            "pack|",
            "package A\n  requirement R on |",
            "[[|",
            "@@ [[|",
            "{|",
            "@@ {|",
        ] {
            let _ = complete(marked);
        }
    }
}
