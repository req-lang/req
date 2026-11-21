use crate::syntax::location::{IntoPosition, Position, PositionIterator, Span, SpanLike};

pub fn tokenize(input: &str) -> Vec<Token> {
    input.char_indices().positioned().tokens().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    ReferenceStart,
    Reference,
    ReferenceEnd,
    UnfinishedReference,
    Newline,
    Indent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

impl SpanLike for Token {
    fn span(&self) -> Span {
        self.span
    }
}

impl Token {
    fn new(kind: Kind, start: Position, end: Position) -> Self {
        Self {
            kind,
            span: Span::new(start, end),
        }
    }

    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        &source[self.span.range()]
    }

    pub fn name(&self) -> Option<Span> {
        match self.kind {
            Kind::Reference => Some(self.span),
            Kind::UnfinishedReference => {
                let skip = REFERENCE_SYMBOL_LEN;
                let start = self.span.start;
                let start = Position::new(start.line, start.column + skip, start.index + skip);
                Some(Span::new(start, self.span.end))
            }
            _ => None,
        }
    }
}

pub const REFERENCE_START: &str = "[[";
pub const REFERENCE_END: &str = "]]";
pub const REFERENCE_SYMBOL_LEN: usize = 2;

fn is_identifier(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | ':')
}

enum Opening {
    Reference,
    Unfinished(usize),
}

impl Opening {
    fn of(rest: &str) -> Option<Self> {
        let rest = rest.strip_prefix(REFERENCE_START)?;
        if rest.starts_with('[') {
            return None;
        }
        let name = rest.find(|c| !is_identifier(c)).unwrap_or(rest.len());
        if name > 0 && rest[name..].starts_with(REFERENCE_END) {
            return Some(Self::Reference);
        }
        Some(Self::Unfinished(rest[..name].chars().count()))
    }
}

pub struct TokenIterator<'a> {
    positions: PositionIterator<'a>,
    start: Position,
    in_reference: bool,
    at_line_start: bool,
    ready: Option<Token>,
    after: Option<Token>,
}

pub trait IntoTokens<'a> {
    fn tokens(self) -> TokenIterator<'a>;
}

impl<'a> IntoTokens<'a> for PositionIterator<'a> {
    fn tokens(self) -> TokenIterator<'a> {
        TokenIterator {
            start: self.current(),
            positions: self,
            in_reference: false,
            at_line_start: false,
            ready: None,
            after: None,
        }
    }
}

impl TokenIterator<'_> {
    fn push(&mut self, kind: Kind, end: Position) {
        let token = Token::new(kind, self.start, end);
        if !token.span.range().is_empty() {
            match self.ready {
                None => self.ready = Some(token),
                Some(_) => self.after = Some(token),
            }
        }
        self.start = end;
    }
}

impl Iterator for TokenIterator<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        while self.ready.is_none() {
            let Some((position, c)) = self.positions.next() else {
                let end = self.positions.current();
                let kind = if self.at_line_start {
                    Kind::Indent
                } else {
                    Kind::Text
                };
                self.push(kind, end);
                self.at_line_start = false;
                break;
            };
            let next = self.positions.current();

            if self.at_line_start && !matches!(c, ' ' | '\t') {
                self.push(Kind::Indent, position);
                self.at_line_start = false;
            }

            match c {
                '\n' => {
                    self.push(Kind::Text, position);
                    self.push(Kind::Newline, next);
                    self.at_line_start = true;
                }
                '[' => match Opening::of(&self.positions.input()[position.index..]) {
                    Some(Opening::Reference) => {
                        self.push(Kind::Text, position);
                        self.positions.next();
                        self.push(Kind::ReferenceStart, self.positions.current());
                        self.in_reference = true;
                    }
                    Some(Opening::Unfinished(name)) => {
                        self.push(Kind::Text, position);
                        self.positions.nth(name);
                        self.push(Kind::UnfinishedReference, self.positions.current());
                    }
                    None => {}
                },
                ']' if self.in_reference => {
                    self.push(Kind::Reference, position);
                    self.positions.next();
                    self.push(Kind::ReferenceEnd, self.positions.current());
                    self.in_reference = false;
                }
                _ => {}
            }
        }
        let token = self.ready.take();
        self.ready = self.after.take();
        token
    }
}

#[cfg(test)]
mod tests {
    use super::{IntoTokens, Kind, Kind::*};
    use crate::syntax::{location::IntoPosition, location::Position};

    /// The kind and the text of each token
    fn lex(input: &str) -> Vec<(Kind, &str)> {
        let tokens = input.char_indices().positioned().tokens();
        tokens.map(|t| (t.kind, t.text(input))).collect()
    }

    fn references(input: &str) -> Vec<&str> {
        lex(input)
            .into_iter()
            .filter(|(kind, _)| *kind == Reference)
            .map(|(_, text)| text)
            .collect()
    }

    #[test]
    fn tokenizes_the_indentation_of_each_line_but_the_first() {
        let input = "  first\n  \t second [[r]]\n\n    last";
        assert_eq!(
            lex(input),
            [
                (Text, "  first"),
                (Newline, "\n"),
                (Indent, "  \t "),
                (Text, "second "),
                (ReferenceStart, "[["),
                (Reference, "r"),
                (ReferenceEnd, "]]"),
                (Newline, "\n"),
                (Newline, "\n"),
                (Indent, "    "),
                (Text, "last"),
            ]
        );
    }

    #[test]
    fn tokens_cover_the_whole_input() {
        for input in [
            "\n   a [[b]]\n \r\n\t\n  é",
            "[[]]",
            "a [[",
            "a [[b",
            "a [[b\n  c",
            "a ]] b",
            "  [[a[[b]]]]",
            "[[a]][[b]]",
            "\\frac{a}{b} x^{2} A{Choix}",
            "[[a b]] [[a\nb]]",
        ] {
            let text: String = lex(input).iter().map(|(_, text)| *text).collect();
            assert_eq!(text, input);
        }
    }

    #[test]
    fn positions_the_tokens_in_the_text() {
        let text = "ab\n  c [[é]]";
        let tokens = text
            .char_indices()
            .positioned()
            .tokens()
            .map(|t| (t.kind, t.span))
            .collect::<Vec<_>>();

        let (kind, span) = tokens[0];
        assert_eq!(kind, Text);
        assert_eq!(
            (span.start, span.end),
            (Position::new(0, 0, 0), Position::new(0, 2, 2))
        );
        // The reference, on the second line: columns restart at the line, and the index counts
        // the bytes of `é`
        // The identifier, between the brackets
        let (kind, span) = tokens.iter().find(|(k, _)| *k == Reference).unwrap();
        assert_eq!(*kind, Reference);
        assert_eq!(
            (span.start, span.end),
            (Position::new(1, 6, 9), Position::new(1, 7, 11))
        );
    }

    #[test]
    fn tokenizes_simple_input() {
        let input = "some text [[a_reference]] some text";
        assert_eq!(
            lex(input),
            [
                (Text, "some text "),
                (ReferenceStart, "[["),
                (Reference, "a_reference"),
                (ReferenceEnd, "]]"),
                (Text, " some text")
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_the_start() {
        let input = "[[a_reference]] some text";
        assert_eq!(
            lex(input),
            [
                (ReferenceStart, "[["),
                (Reference, "a_reference"),
                (ReferenceEnd, "]]"),
                (Text, " some text")
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_the_end() {
        let input = "some text [[a_reference]]";
        assert_eq!(
            lex(input),
            [
                (Text, "some text "),
                (ReferenceStart, "[["),
                (Reference, "a_reference"),
                (ReferenceEnd, "]]"),
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_start_of_line() {
        let input = "some text\n[[a_reference]]";
        assert_eq!(
            lex(input),
            [
                (Text, "some text"),
                (Newline, "\n"),
                (ReferenceStart, "[["),
                (Reference, "a_reference"),
                (ReferenceEnd, "]]"),
            ]
        );
    }

    #[test]
    fn tokenizes_reference_after_indentation() {
        let input = "a\n  [[r]]";
        assert_eq!(
            lex(input),
            [
                (Text, "a"),
                (Newline, "\n"),
                (Indent, "  "),
                (ReferenceStart, "[["),
                (Reference, "r"),
                (ReferenceEnd, "]]"),
            ]
        );
    }

    #[test]
    fn the_tokens_end_with_the_text() {
        // Bounded, so that an iterator that never ends fails instead of filling the memory
        for input in [
            "",
            "a",
            "a [[",
            "a [[b",
            "[[",
            "[[]]",
            "a\n  ",
            "a [[b\n  c",
        ] {
            let tokens = input
                .char_indices()
                .positioned()
                .tokens()
                .take(input.len() + 8)
                .count();
            assert!(tokens <= input.len() + 3, "{input:?}: {tokens} tokens");
        }
    }

    #[test]
    fn an_identifier_is_made_of_letters_digits_and_a_few_signs() {
        assert_eq!(references("[[A::b_c-d:e9]]"), ["A::b_c-d:e9"]);
        assert_eq!(references("[[é]]"), ["é"]);
        assert_eq!(
            lex("[[a b]]"),
            [(UnfinishedReference, "[[a"), (Text, " b]]")]
        );
        assert_eq!(
            lex("[[a.b]]"),
            [(UnfinishedReference, "[[a"), (Text, ".b]]")]
        );
        assert_eq!(
            lex("[[a|b]]"),
            [(UnfinishedReference, "[[a"), (Text, "|b]]")]
        );
    }

    #[test]
    fn an_empty_reference_is_unfinished() {
        assert_eq!(
            lex("see [[]]"),
            [(Text, "see "), (UnfinishedReference, "[["), (Text, "]]")]
        );
    }

    #[test]
    fn a_reference_that_is_not_closed_is_unfinished() {
        assert_eq!(lex("see [["), [(Text, "see "), (UnfinishedReference, "[[")]);
        assert_eq!(
            lex("see [[B::"),
            [(Text, "see "), (UnfinishedReference, "[[B::")]
        );
        assert_eq!(
            lex("see [[B]"),
            [(Text, "see "), (UnfinishedReference, "[[B"), (Text, "]")]
        );
        assert_eq!(
            lex("see [[é a"),
            [(Text, "see "), (UnfinishedReference, "[[é"), (Text, " a")]
        );
    }

    #[test]
    fn a_single_bracket_is_text() {
        assert_eq!(lex("see [B]] [ ["), [(Text, "see [B]] [ [")]);
    }

    #[test]
    fn a_reference_is_on_one_line() {
        assert_eq!(
            lex("a [[b\nc]]"),
            [
                (Text, "a "),
                (UnfinishedReference, "[[b"),
                (Newline, "\n"),
                (Text, "c]]")
            ]
        );
        assert_eq!(
            lex("a [[\nb]]"),
            [
                (Text, "a "),
                (UnfinishedReference, "[["),
                (Newline, "\n"),
                (Text, "b]]")
            ]
        );
    }

    #[test]
    fn what_follows_a_reference_that_is_not_closed_is_still_read() {
        assert_eq!(
            lex("[[a [[b]]"),
            [
                (UnfinishedReference, "[[a"),
                (Text, " "),
                (ReferenceStart, "[["),
                (Reference, "b"),
                (ReferenceEnd, "]]"),
            ]
        );
    }

    #[test]
    fn the_last_two_brackets_of_a_run_open_the_reference() {
        assert_eq!(
            lex("[[[ [[[b"),
            [
                (Text, "["),
                (UnfinishedReference, "[["),
                (Text, " ["),
                (UnfinishedReference, "[[b"),
            ]
        );
    }

    #[test]
    fn the_name_of_a_reference_is_what_is_typed_after_the_brackets() {
        let input = "ab [[é]] [[c [[d\n  [[";
        let names = input
            .char_indices()
            .positioned()
            .tokens()
            .filter_map(|token| token.name())
            .map(|name| {
                let (start, end) = (name.start, name.end);
                (&input[name.range()], start.line, start.column, end.column)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                ("é", 0, 5, 6),
                ("c", 0, 11, 12),
                ("d", 0, 15, 16),
                ("", 1, 4, 4)
            ]
        );
    }

    #[test]
    fn two_references_next_to_each_other_are_two_references() {
        assert_eq!(
            lex("[[a]][[b]]"),
            [
                (ReferenceStart, "[["),
                (Reference, "a"),
                (ReferenceEnd, "]]"),
                (ReferenceStart, "[["),
                (Reference, "b"),
                (ReferenceEnd, "]]"),
            ]
        );
    }

    #[test]
    fn extra_brackets_around_a_reference_are_text() {
        assert_eq!(
            lex("[[[a]]]"),
            [
                (Text, "["),
                (ReferenceStart, "[["),
                (Reference, "a"),
                (ReferenceEnd, "]]"),
                (Text, "]"),
            ]
        );
    }

    #[test]
    fn a_closing_bracket_alone_is_text() {
        assert_eq!(lex("a ]] b"), [(Text, "a ]] b")]);
    }

    #[test]
    fn braces_are_text() {
        for input in ["\\frac{a}{b}", "x^{2}", "A{Choix}", "{a}", "{", "}", "{}"] {
            assert_eq!(lex(input), [(Text, input)], "{input:?}");
        }
    }

    #[test]
    fn tokenizes_new_lines() {
        let input = "some text\nother text";
        assert_eq!(
            lex(input),
            [(Text, "some text"), (Newline, "\n"), (Text, "other text")]
        );
    }
}
