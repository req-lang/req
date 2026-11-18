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
}

pub const REFERENCE_START: char = '{';
pub const REFERENCE_END: char = '}';

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
            start: self.position(),
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
        if kind == Kind::Reference || !token.span.range().is_empty() {
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
                let end = self.positions.position();
                let kind = if self.at_line_start {
                    Kind::Indent
                } else if self.in_reference {
                    Kind::Reference
                } else {
                    Kind::Text
                };
                self.push(kind, end);
                self.at_line_start = false;
                self.in_reference = false;
                break;
            };
            let next = self.positions.position();

            if self.at_line_start && !matches!(c, ' ' | '\t') {
                self.push(Kind::Indent, position);
                self.at_line_start = false;
            }

            match c {
                '\n' if !self.in_reference => {
                    self.push(Kind::Text, position);
                    self.push(Kind::Newline, next);
                    self.at_line_start = true;
                }
                REFERENCE_START if !self.in_reference => {
                    self.push(Kind::Text, position);
                    self.push(Kind::ReferenceStart, next);
                    self.in_reference = true;
                }
                REFERENCE_END if self.in_reference => {
                    self.push(Kind::Reference, position);
                    self.push(Kind::ReferenceEnd, next);
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

    #[test]
    fn tokenizes_the_indentation_of_each_line_but_the_first() {
        let input = "  first\n  \t second {r}\n\n    last";
        assert_eq!(
            lex(input),
            [
                (Text, "  first"),
                (Newline, "\n"),
                (Indent, "  \t "),
                (Text, "second "),
                (ReferenceStart, "{"),
                (Reference, "r"),
                (ReferenceEnd, "}"),
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
            "\n   a {b}\n \r\n\t\n  é",
            "{}",
            "a {",
            "a {b",
            "a {b\n  c",
            "a } b",
            "  {a{b}}",
            "{a}{b}",
        ] {
            let text: String = lex(input).iter().map(|(_, text)| *text).collect();
            assert_eq!(text, input);
        }
    }

    #[test]
    fn positions_the_tokens_in_the_text() {
        let text = "ab\n  c {é}";
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
        // The path, between the braces
        let (kind, span) = tokens.iter().find(|(k, _)| *k == Reference).unwrap();
        assert_eq!(*kind, Reference);
        assert_eq!(
            (span.start, span.end),
            (Position::new(1, 5, 8), Position::new(1, 6, 10))
        );
    }

    #[test]
    fn tokenizes_simple_input() {
        let input = "some text {a reference} some text";
        assert_eq!(
            lex(input),
            [
                (Text, "some text "),
                (ReferenceStart, "{"),
                (Reference, "a reference"),
                (ReferenceEnd, "}"),
                (Text, " some text")
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_the_start() {
        let input = "{a reference} some text";
        assert_eq!(
            lex(input),
            [
                (ReferenceStart, "{"),
                (Reference, "a reference"),
                (ReferenceEnd, "}"),
                (Text, " some text")
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_the_end() {
        let input = "some text {a reference}";
        assert_eq!(
            lex(input),
            [
                (Text, "some text "),
                (ReferenceStart, "{"),
                (Reference, "a reference"),
                (ReferenceEnd, "}"),
            ]
        );
    }

    #[test]
    fn tokenizes_reference_at_start_of_line() {
        let input = "some text\n{a reference}";
        assert_eq!(
            lex(input),
            [
                (Text, "some text"),
                (Newline, "\n"),
                (ReferenceStart, "{"),
                (Reference, "a reference"),
                (ReferenceEnd, "}"),
            ]
        );
    }

    #[test]
    fn tokenizes_reference_after_indentation() {
        let input = "a\n  {r}";
        assert_eq!(
            lex(input),
            [
                (Text, "a"),
                (Newline, "\n"),
                (Indent, "  "),
                (ReferenceStart, "{"),
                (Reference, "r"),
                (ReferenceEnd, "}"),
            ]
        );
    }

    #[test]
    fn the_tokens_end_with_the_text() {
        // Bounded, so that an iterator that never ends fails instead of filling the memory
        for input in ["", "a", "a {", "a {b", "{", "{}", "a\n  ", "a {b\n  c"] {
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
    fn tokenizes_empty_reference() {
        let input = "see {}";
        assert_eq!(
            lex(input),
            [
                (Text, "see "),
                (ReferenceStart, "{"),
                (Reference, ""),
                (ReferenceEnd, "}"),
            ]
        );
    }

    #[test]
    fn tokenizes_unclosed_reference() {
        // The path is there, there is no end
        assert_eq!(
            lex("see {"),
            [(Text, "see "), (ReferenceStart, "{"), (Reference, "")]
        );
        assert_eq!(
            lex("see {B::"),
            [(Text, "see "), (ReferenceStart, "{"), (Reference, "B::")]
        );
    }

    #[test]
    fn tokenizes_the_empty_path_where_the_cursor_goes() {
        let input = "see {}";
        let tokens = input
            .char_indices()
            .positioned()
            .tokens()
            .collect::<Vec<_>>();
        let path = tokens.iter().find(|t| t.kind == Reference).unwrap();
        assert_eq!(path.span.start, Position::new(0, 5, 5));
        assert_eq!(path.span.end, path.span.start);
    }

    #[test]
    fn a_closing_brace_alone_is_text() {
        assert_eq!(lex("a } b"), [(Text, "a } b")]);
    }

    #[test]
    fn an_opening_brace_in_a_reference_is_part_of_its_path() {
        assert_eq!(
            lex("{a{b}"),
            [
                (ReferenceStart, "{"),
                (Reference, "a{b"),
                (ReferenceEnd, "}")
            ]
        );
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
