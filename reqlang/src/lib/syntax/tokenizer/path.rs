use crate::syntax::location::{IntoPosition, Position, PositionIterator, Span, SpanLike};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Segment,
    Separator,
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
    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        &source[self.span.range()]
    }
}

pub fn tokenize(input: &str) -> TokenIterator<'_> {
    input.char_indices().positioned().tokens()
}

pub trait IntoTokens<'a> {
    fn tokens(self) -> TokenIterator<'a>;
}

impl<'a> IntoTokens<'a> for PositionIterator<'a> {
    fn tokens(mut self) -> TokenIterator<'a> {
        let first = self.next();
        let second = self.next();
        TokenIterator {
            positions: self,
            first,
            second,
        }
    }
}

pub struct TokenIterator<'a> {
    positions: PositionIterator<'a>,
    first: Option<(Position, char)>,
    second: Option<(Position, char)>,
}

impl Iterator for TokenIterator<'_> {
    type Item = Token;

    // Every token consumes at least one char, so the iterator always ends
    fn next(&mut self) -> Option<Token> {
        let (start, _) = self.peek()?;
        let kind = match self.at_separator() {
            true => {
                self.advance();
                self.advance();
                Kind::Separator
            }
            false => {
                self.advance();
                while self.peek().is_some() && !self.at_separator() {
                    self.advance();
                }
                Kind::Segment
            }
        };
        Some(Token {
            kind,
            span: Span::new(start, self.here()),
        })
    }
}

impl TokenIterator<'_> {
    fn peek(&self) -> Option<(Position, char)> {
        self.first
    }

    fn at_separator(&self) -> bool {
        let chars = (self.first, self.second);
        matches!(chars, (Some((_, ':')), Some((_, ':'))))
    }

    fn advance(&mut self) {
        self.first = self.second;
        self.second = self.positions.next();
    }

    // Where the next char is, which is where the token that was just read ends
    fn here(&self) -> Position {
        self.peek()
            .map_or_else(|| self.positions.position(), |(position, _)| position)
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind::*, *};

    fn lex(input: &str) -> Vec<(Kind, &str)> {
        tokenize(input).map(|t| (t.kind, t.text(input))).collect()
    }

    #[test]
    fn a_name_is_one_segment() {
        assert_eq!(lex("abc"), [(Segment, "abc")]);
        assert_eq!(lex(""), []);
    }

    #[test]
    fn segments_are_separated_by_a_double_colon() {
        assert_eq!(
            lex("a::b::c"),
            [
                (Segment, "a"),
                (Separator, "::"),
                (Segment, "b"),
                (Separator, "::"),
                (Segment, "c"),
            ]
        );
    }

    #[test]
    fn a_separator_does_not_need_its_segments() {
        assert_eq!(lex("a::"), [(Segment, "a"), (Separator, "::")]);
        assert_eq!(lex("::x"), [(Separator, "::"), (Segment, "x")]);
        assert_eq!(
            lex("a::::b"),
            [
                (Segment, "a"),
                (Separator, "::"),
                (Separator, "::"),
                (Segment, "b")
            ]
        );
        assert_eq!(lex("::"), [(Separator, "::")]);
    }

    #[test]
    fn a_single_colon_is_part_of_a_segment() {
        assert_eq!(lex("a:b"), [(Segment, "a:b")]);
        assert_eq!(lex("a:"), [(Segment, "a:")]);
        assert_eq!(
            lex("a:::b"),
            [(Segment, "a"), (Separator, "::"), (Segment, ":b")]
        );
    }

    #[test]
    fn a_segment_can_hold_a_newline() {
        assert_eq!(
            lex("a\nb::c"),
            [(Segment, "a\nb"), (Separator, "::"), (Segment, "c")]
        );
    }

    #[test]
    fn columns_count_chars_and_indices_count_bytes() {
        let input = "é::ü";
        let tokens: Vec<_> = tokenize(input).collect();
        let columns: Vec<_> = tokens
            .iter()
            .map(|t| (t.span.start.column, t.span.end.column))
            .collect();
        let indices: Vec<_> = tokens
            .iter()
            .map(|t| (t.span.start.index, t.span.end.index))
            .collect();
        assert_eq!(columns, [(0, 1), (1, 3), (3, 4)]);
        assert_eq!(indices, [(0, 2), (2, 4), (4, 6)]);
    }

    #[test]
    fn the_tokens_cover_the_whole_input_and_end_with_it() {
        for input in ["", "a", "::", "a::b", ":::", "a::::", "::a::", "é\n::ü:"] {
            let tokens: Vec<_> = tokenize(input).take(input.len() + 2).collect();
            assert!(tokens.len() <= input.len(), "{input:?}");
            let text: String = tokens.iter().map(|t| t.text(input)).collect();
            assert_eq!(text, input);
        }
    }
}
