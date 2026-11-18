use crate::{
    node::path::Path,
    syntax::{
        location::Span,
        tokenizer::path::{Kind, Token, tokenize},
    },
};

// The path written as `text` at `span`, and its tokens: the names and the separators. A path
// with no name has a token with no length, so that it still is somewhere.
pub fn parse(text: &str, span: Span) -> (Path, Vec<Token>) {
    let mut tokens: Vec<_> = tokenize(text)
        .map(|token| Token {
            span: token.span + span.start,
            ..token
        })
        .collect();
    if tokens.is_empty() {
        tokens.push(Token {
            kind: Kind::Segment,
            span: Span::new(span.start, span.start),
        });
    }
    (Path::from(text), tokens)
}

#[cfg(test)]
mod tests {
    use crate::syntax::location::Position;

    use super::*;

    fn at(line: usize, column: usize, index: usize) -> Span {
        let start = Position::new(line, column, index);
        Span::new(start, start)
    }

    // (kind, line, column, index, length in bytes) of each token
    fn places(tokens: &[Token]) -> Vec<(Kind, usize, usize, usize, usize)> {
        tokens
            .iter()
            .map(|token| {
                let (start, end) = (token.span.start, token.span.end);
                (
                    token.kind,
                    start.line,
                    start.column,
                    start.index,
                    end.index - start.index,
                )
            })
            .collect()
    }

    #[test]
    fn the_tokens_are_where_the_path_is() {
        let (path, tokens) = parse("a::bc", at(2, 10, 40));
        assert_eq!(path.as_str(), "a::bc");
        assert_eq!(
            places(&tokens),
            [
                (Kind::Segment, 2, 10, 40, 1),
                (Kind::Separator, 2, 11, 41, 2),
                (Kind::Segment, 2, 13, 43, 2),
            ]
        );
    }

    #[test]
    fn a_path_with_no_name_has_a_token_with_no_length() {
        let (path, tokens) = parse("", at(1, 4, 9));
        assert_eq!(path.as_str(), "");
        assert_eq!(places(&tokens), [(Kind::Segment, 1, 4, 9, 0)]);
    }

    #[test]
    fn the_path_is_normalized_but_the_tokens_follow_the_text() {
        // `e` and a combining accent are two chars, the path has one
        let (path, tokens) = parse("e\u{301}::b", at(0, 0, 0));
        assert_eq!(path.as_str(), "\u{e9}::b");
        assert_eq!(tokens[0].span.end.column, 2);
        assert_eq!(tokens[2].span.start.column, 4);
    }
}
