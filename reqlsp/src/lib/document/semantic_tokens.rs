use reqlang::{
    Specification,
    node::entity,
    semantics::link::Target,
    syntax::{
        self,
        tokenizer::{self, entity::TokenVariant, expression, markup, path},
    },
};
use tower_lsp::lsp_types::*;

use crate::location::Span;

// The index of a kind is the one of its type in the legend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Keyword,
    Operator,
    Number,
    String,
    Comment,
    Variable,
    Namespace,
    Class,
    Property,
    Function,
    Enum,
    EnumMember,
    Type,
}

// The bit of a modifier is its index in the legend
const DECLARATION: u32 = 1;

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::COMMENT,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::CLASS,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::ENUM,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::TYPE,
        ],
        token_modifiers: vec![SemanticTokenModifier::DECLARATION],
    }
}

impl From<&entity::Variant> for Kind {
    fn from(value: &entity::Variant) -> Self {
        use entity::Variant::*;

        match value {
            Package(_) => Kind::Namespace,
            Part(_) => Kind::Class,
            Requirement(_) => Kind::Function,
            Attribute(_) => Kind::Property,
            Type(_) => Kind::Enum,
        }
    }
}

impl Kind {
    fn decl(token: &tokenizer::entity::Keyword) -> Option<Self> {
        use tokenizer::entity::Keyword::*;

        match token {
            Package => Some(Kind::Namespace),
            Part => Some(Kind::Class),
            Requirement => Some(Kind::Function),
            Let => Some(Kind::Property),
            Type => Some(Kind::Enum),
            _ => None,
        }
    }

    fn meta(token: tokenizer::entity::TokenVariant) -> Option<Self> {
        use tokenizer::entity::TokenVariant::*;

        match token {
            Reserved(_) => Some(Kind::Keyword),
            Tag => Some(Kind::Comment),
            Unit => Some(Kind::String),
            _ => None,
        }
    }

    fn expr(token: tokenizer::expression::Kind) -> Option<Self> {
        use tokenizer::expression::Kind::*;

        match token {
            Keyword(_) => Some(Kind::Keyword),
            Operator(_) => Some(Kind::Operator),
            Number => Some(Kind::Number),
            _ => None,
        }
    }
}

pub struct Token {
    kind: Kind,
    span: Span,
    is_declaration: bool,
}

impl Token {
    fn new(kind: Kind, span: Span) -> Self {
        Self {
            kind,
            span,
            is_declaration: false,
        }
    }

    fn decl(kind: Kind, span: Span) -> Self {
        Self {
            kind,
            span,
            is_declaration: true,
        }
    }

    fn entity(current: &tokenizer::entity::Token) -> Option<Self> {
        Some(Token::new(
            Kind::meta(current.variant)?,
            current.span.into(),
        ))
    }

    fn entity2(
        previous: &tokenizer::entity::Token,
        current: &tokenizer::entity::Token,
    ) -> Option<Token> {
        if let TokenVariant::Reserved(previous) = previous.variant
            && let Some(kind) = Kind::decl(&previous)
            && current.variant == TokenVariant::Identifier
        {
            return Some(Token::decl(kind, current.span.into()));
        }

        None
    }

    fn expr(current: &tokenizer::expression::Token) -> Option<Self> {
        Some(Token::new(Kind::expr(current.kind)?, current.span.into()))
    }

    fn expr2(
        previous: &tokenizer::expression::Token,
        current: &tokenizer::expression::Token,
    ) -> Option<Token> {
        use expression::Keyword;
        use expression::Kind::*;

        if previous.kind == Keyword(Keyword::For) && current.kind == Identifier {
            return Some(Token::decl(Kind::Variable, current.span.into()));
        }

        None
    }

    fn markup(token: &tokenizer::markup::Token) -> Option<Token> {
        let kind = match token.kind {
            markup::Kind::Text => Some(Kind::String),
            _ => None,
        }?;
        Some(Token::new(kind, token.span.into()))
    }

    fn path(
        spec: &Specification,
        tokens: &[tokenizer::path::Token],
        tgt: Option<Target>,
    ) -> Vec<Token> {
        let mut kinds = match tgt {
            Some(tgt) => target_segments_kind(spec, tgt),
            None => Vec::new(),
        }
        .into_iter();
        tokens
            .iter()
            .rev()
            .map(|tok| match tok.kind {
                path::Kind::Segment => {
                    Token::new(kinds.next().unwrap_or(Kind::Variable), tok.span.into())
                }
                path::Kind::Separator => Token::new(Kind::Operator, tok.span.into()),
            })
            .collect()
    }
}

fn entity_segments_kind(spec: &Specification, idx: entity::Idx) -> Vec<Kind> {
    let entities = &spec.tree.entities;
    let neighbors = &spec.analysis.neighbors;

    let mut kinds = Vec::new();
    let mut current = Some(idx);
    while let Some(idx) = current {
        kinds.push((&entities[idx].variant).into());
        current = neighbors[idx].parent;
    }
    kinds
}

fn target_segments_kind(spec: &Specification, target: Target) -> Vec<Kind> {
    match target {
        Target::Entity(entity) => entity_segments_kind(spec, entity),
        Target::Enum(entity, _) => {
            [vec![Kind::EnumMember], entity_segments_kind(spec, entity)].concat()
        }
        Target::Builtin(_) => vec![Kind::Type],
        Target::Local(_) => Vec::new(),
    }
}

pub fn collect(info: &syntax::parser::Info, spec: &Specification) -> Vec<Token> {
    let mut sems: Vec<Token> = Vec::new();

    let tokens = info.entities.tokens();
    let entities = tokens.iter().filter_map(Token::entity);
    let entities2 = tokens
        .windows(2)
        .filter_map(|tok| Token::entity2(&tok[0], &tok[1]));
    sems.extend(entities);
    sems.extend(entities2);

    let tokens = info.markups.tokens();
    let markups = tokens.iter().filter_map(Token::markup);
    sems.extend(markups);

    let tokens = info.expression.tokens();
    let expr = tokens.iter().filter_map(Token::expr);
    let expr2 = tokens
        .windows(2)
        .filter_map(|tok| Token::expr2(&tok[0], &tok[1]));
    sems.extend(expr);
    sems.extend(expr2);

    let refs = spec.analysis.links.references.enumerate();
    let refs = refs.into_iter().flat_map(|(idx, rf)| {
        let tokens = info.references.tokens_of(idx);
        Token::path(spec, tokens, rf.map(Target::Entity))
    });
    sems.extend(refs);

    let ids = spec.analysis.links.identifiers.enumerate();
    let ids = ids.into_iter().flat_map(|(idx, tgt)| {
        let tokens = info.identifiers.tokens_of(idx);
        Token::path(&spec, tokens, *tgt)
    });
    sems.extend(ids);
    sems.sort_by_key(|tok| (tok.span.start.line, tok.span.start.column));
    sems
}

#[derive(Debug, Default)]
pub struct Delta {
    line: u32,
    start: u32,
}

fn delta(spans: impl Iterator<Item = Span>) -> Vec<Delta> {
    spans
        .scan(Delta::default(), |delta, span| {
            let line = span.start.line - delta.line;
            let start = match line {
                0 => span.start.column - delta.start,
                _ => span.start.column,
            };
            delta.line = span.start.line;
            delta.start = span.start.column;
            Some(Delta { line, start })
        })
        .collect()
}

// Each token is relative to the previous one. A token is on one line.
pub fn encode(tokens: Vec<Token>) -> Vec<SemanticToken> {
    let non_zeros = tokens.iter().filter(|tok| {
        let span = tok.span;
        let length = span.end.column.saturating_sub(span.start.column);
        span.start.line == span.end.line && length > 0
    });
    let delta = delta(non_zeros.clone().into_iter().map(|t| t.span));
    non_zeros
        .zip(delta)
        .map(|(tok, delta)| {
            let span = tok.span;
            let kind = tok.kind;
            let modifiers = match tok.is_declaration {
                true => DECLARATION,
                false => 0,
            };
            let length = span.end.column.saturating_sub(span.start.column);
            SemanticToken {
                delta_line: delta.line,
                delta_start: delta.start,
                length,
                token_type: kind as u32,
                token_modifiers_bitset: modifiers,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::document::Document;

    use super::*;

    // (line, column, length, kind, modifiers) of each token
    fn decoded(source: &str) -> Vec<(u32, u32, u32, u32, u32)> {
        let uri = Url::parse("file:///a.req").unwrap();
        let tokens = Document::new(uri, source, 0).semantic_tokens();

        let (mut line, mut start) = (0, 0);
        tokens
            .data
            .iter()
            .map(|token| {
                line += token.delta_line;
                start = match token.delta_line {
                    0 => start + token.delta_start,
                    _ => token.delta_start,
                };
                (
                    line,
                    start,
                    token.length,
                    token.token_type,
                    token.token_modifiers_bitset,
                )
            })
            .collect()
    }

    fn on_line(source: &str, line: u32) -> Vec<(u32, u32, u32, u32, u32)> {
        let tokens = decoded(source);
        tokens.into_iter().filter(|token| token.0 == line).collect()
    }

    const KEYWORD: u32 = Kind::Keyword as u32;
    const OPERATOR: u32 = Kind::Operator as u32;
    const NUMBER: u32 = Kind::Number as u32;
    const STRING: u32 = Kind::String as u32;
    const COMMENT: u32 = Kind::Comment as u32;
    const VARIABLE: u32 = Kind::Variable as u32;
    const NAMESPACE: u32 = Kind::Namespace as u32;
    const CLASS: u32 = Kind::Class as u32;
    const PROPERTY: u32 = Kind::Property as u32;
    const FUNCTION: u32 = Kind::Function as u32;
    const ENUM: u32 = Kind::Enum as u32;
    const ENUM_MEMBER: u32 = Kind::EnumMember as u32;
    const TYPE: u32 = Kind::Type as u32;

    #[test]
    fn the_legend_follows_the_kinds() {
        let legend = legend();
        let types = legend.token_types;
        for (kind, expected) in [
            (Kind::Keyword, SemanticTokenType::KEYWORD),
            (Kind::Operator, SemanticTokenType::OPERATOR),
            (Kind::Number, SemanticTokenType::NUMBER),
            (Kind::String, SemanticTokenType::STRING),
            (Kind::Comment, SemanticTokenType::COMMENT),
            (Kind::Variable, SemanticTokenType::VARIABLE),
            (Kind::Namespace, SemanticTokenType::NAMESPACE),
            (Kind::Class, SemanticTokenType::CLASS),
            (Kind::Property, SemanticTokenType::PROPERTY),
            (Kind::Function, SemanticTokenType::FUNCTION),
            (Kind::Enum, SemanticTokenType::ENUM),
            (Kind::EnumMember, SemanticTokenType::ENUM_MEMBER),
            (Kind::Type, SemanticTokenType::TYPE),
        ] {
            assert_eq!(types[kind as usize], expected);
        }
        assert_eq!(types.len(), 13);
        assert_eq!(
            legend.token_modifiers[DECLARATION.trailing_zeros() as usize],
            SemanticTokenModifier::DECLARATION
        );
    }

    #[test]
    fn keywords_operators_numbers_and_names_are_tokens() {
        let source = "package A
  let x in real
  requirement R
    on x
  is
    x > 1
  requirement
package";
        assert_eq!(
            decoded(source),
            [
                (0, 0, 7, KEYWORD, 0),
                (0, 8, 1, NAMESPACE, DECLARATION),
                (1, 2, 3, KEYWORD, 0),
                (1, 6, 1, PROPERTY, DECLARATION),
                (1, 8, 2, KEYWORD, 0),
                (1, 11, 4, TYPE, 0),
                (2, 2, 11, KEYWORD, 0),
                (2, 14, 1, FUNCTION, DECLARATION),
                (3, 4, 2, KEYWORD, 0),
                (3, 7, 1, PROPERTY, 0),
                (4, 2, 2, KEYWORD, 0),
                (5, 4, 1, PROPERTY, 0),
                (5, 6, 1, OPERATOR, 0),
                (5, 8, 1, NUMBER, 0),
                (6, 2, 11, KEYWORD, 0),
                (7, 0, 7, KEYWORD, 0),
            ]
        );
    }

    #[test]
    fn each_name_of_a_path_is_what_it_leads_to_or_contains() {
        let source = "package S
  part P
    let x in real
  part
  requirement R
    on S::P::x
  is
    P::x > 1
  requirement
package";
        // The full path, from the root
        assert_eq!(
            on_line(source, 5),
            [
                (5, 4, 2, KEYWORD, 0),
                (5, 7, 1, NAMESPACE, 0),
                (5, 8, 2, OPERATOR, 0),
                (5, 10, 1, CLASS, 0),
                (5, 11, 2, OPERATOR, 0),
                (5, 13, 1, PROPERTY, 0),
            ]
        );
        // From the package of the requirement
        assert_eq!(
            on_line(source, 7),
            [
                (7, 4, 1, CLASS, 0),
                (7, 5, 2, OPERATOR, 0),
                (7, 7, 1, PROPERTY, 0),
                (7, 9, 1, OPERATOR, 0),
                (7, 11, 1, NUMBER, 0),
            ]
        );
    }

    #[test]
    fn a_type_and_its_members() {
        let source = "package S
  type T has
    A |
    B
  type
  let x in S::T
  requirement R
    on x
  is
    x = T::A
  requirement
package";
        assert_eq!(on_line(source, 1)[1], (1, 7, 1, ENUM, DECLARATION));
        assert_eq!(
            on_line(source, 9),
            [
                (9, 4, 1, PROPERTY, 0),
                (9, 6, 1, OPERATOR, 0),
                (9, 8, 1, ENUM, 0),
                (9, 9, 2, OPERATOR, 0),
                (9, 11, 1, ENUM_MEMBER, 0),
            ]
        );
    }

    #[test]
    fn what_does_not_lead_to_anything_is_a_variable() {
        let source = "package A\n  let x in A::b::c\npackage";
        assert_eq!(
            decoded(source)[3..],
            [
                (1, 6, 1, PROPERTY, DECLARATION),
                (1, 8, 2, KEYWORD, 0),
                (1, 11, 1, VARIABLE, 0),
                (1, 12, 2, OPERATOR, 0),
                (1, 14, 1, VARIABLE, 0),
                (1, 15, 2, OPERATOR, 0),
                (1, 17, 1, VARIABLE, 0),
                (2, 0, 7, KEYWORD, 0),
            ]
        );

        // The separator at the end is a token too
        let tokens = decoded("package A\n  let x in A::\npackage");
        assert!(tokens.contains(&(1, 11, 1, VARIABLE, 0)));
        assert!(tokens.contains(&(1, 12, 2, OPERATOR, 0)));
    }

    #[test]
    fn a_label_is_one_token_even_if_it_has_a_separator() {
        let line = on_line("package X\n  part A::B\n  part\npackage", 1);
        assert_eq!(line, [(1, 2, 4, KEYWORD, 0), (1, 7, 4, CLASS, DECLARATION)]);
    }

    #[test]
    fn the_variable_of_a_for_is_a_declaration() {
        let source =
            "package A\n  requirement R\n  is\n    sum s for s in S end\n  requirement\npackage";
        assert_eq!(
            on_line(source, 3),
            [
                (3, 4, 3, KEYWORD, 0),
                (3, 8, 1, VARIABLE, 0),
                (3, 10, 3, KEYWORD, 0),
                (3, 14, 1, VARIABLE, DECLARATION),
                (3, 16, 2, KEYWORD, 0),
                (3, 19, 1, VARIABLE, 0),
                (3, 21, 3, KEYWORD, 0),
            ]
        );
    }

    #[test]
    fn what_is_not_parsed_is_still_highlighted() {
        // The requirement is not complete, it is not in the tree: its name is a label, and what
        // it names is not linked to anything
        let source = "package A\n  requirement R\n  is\n    x >\n  requirement\npackage";
        let tokens = decoded(source);
        assert!(tokens.contains(&(1, 14, 1, FUNCTION, DECLARATION)));
        assert!(tokens.contains(&(3, 4, 1, VARIABLE, 0)));
        assert!(tokens.contains(&(3, 6, 1, OPERATOR, 0)));
    }

    #[test]
    fn the_text_of_a_requirement_is_a_string_and_its_references_are_paths() {
        let source = "package A\n  requirement R\n  is\n    @@\n    see {R} and {B::x}\n    @@\n  requirement\npackage";
        // The braces are left out
        assert_eq!(
            on_line(source, 4),
            [
                (4, 4, 4, STRING, 0),
                (4, 9, 1, FUNCTION, 0),
                (4, 11, 5, STRING, 0),
                (4, 17, 1, VARIABLE, 0),
                (4, 18, 2, OPERATOR, 0),
                (4, 20, 1, VARIABLE, 0),
            ]
        );
    }

    #[test]
    fn tags_and_units_are_tokens() {
        let source = "package A\n  # kind draft #\n  let x in real \\ m \\\npackage";
        let tokens = decoded(source);
        assert!(tokens.iter().any(|token| token.3 == COMMENT));
        assert!(tokens.iter().any(|token| token.3 == STRING));
    }

    #[test]
    fn broken_or_empty_input_does_not_fail() {
        for source in [
            "",
            "pack",
            "package A\n  requirement R on",
            "{ @@ {",
            "\n\n  1 +",
            "package a::\n  part ::\n@@ {a::\n b} @@",
        ] {
            let _ = decoded(source);
        }
    }
}
