use Keyword::*;

use crate::{
    syntax::location::{IntoPosition, Position, PositionIterator, Span, SpanLike},
    utils::cursor::Kinded,
};

pub const REFERENCE_SEPARATOR: &str = "::";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Keyword {
    Package,
    Part,
    Requirement,
    Import,
    Refines,
    Specializes,
    Derives,
    Is,
    In,
    Has,
    Let,
    On,
    Via,
    Top,
    Bottom,
    Type,
}

impl<'a> TryFrom<Lexeme<'a>> for Keyword {
    type Error = &'a str;

    fn try_from(value: Lexeme<'a>) -> Result<Self, Self::Error> {
        use Lexeme::*;

        if let Word(w) = value {
            match w {
                "package" => Ok(Package),
                "part" => Ok(Part),
                "requirement" => Ok(Requirement),
                "import" => Ok(Import),
                "refines" => Ok(Refines),
                "specializes" => Ok(Specializes),
                "derives" => Ok(Derives),
                "in" => Ok(In),
                "is" => Ok(Is),
                "has" => Ok(Has),
                "let" => Ok(Let),
                "on" => Ok(On),
                "via" => Ok(Via),
                "top" => Ok(Top),
                "bottom" => Ok(Bottom),
                "type" => Ok(Type),
                w => Err(w),
            }
        } else {
            Err("")
        }
    }
}

impl Keyword {
    pub fn as_str(&self) -> &'static str {
        match self {
            Package => "package",
            Part => "part",
            Requirement => "requirement",
            Import => "import",
            Refines => "refines",
            Specializes => "specializes",
            Derives => "derives",
            In => "in",
            Is => "is",
            Has => "has",
            Let => "let",
            On => "on",
            Via => "via",
            Top => "top",
            Bottom => "bottom",
            Type => "type",
        }
    }
}

/// What a token is; its text is `&source[token.span.range()]`, see [Token::text]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenVariant {
    Identifier,
    Expression,
    Type,
    Markup,
    Unit,
    /// `id value`: see [Token::tag]
    Tag,
    Reserved(Keyword),
}

pub fn tokenize(input: &str) -> impl Iterator<Item = Token> {
    input.char_indices().positioned().lexemes().tokens()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    pub span: Span,
    pub variant: TokenVariant,
}

impl Kinded for Token {
    type Kind = TokenVariant;

    fn kind(&self) -> TokenVariant {
        self.variant
    }
}

impl SpanLike for Token {
    fn span(&self) -> Span {
        self.span
    }
}

impl Token {
    pub fn new(span: Span, variant: TokenVariant) -> Self {
        Self { span, variant }
    }

    /// The text of the token in the `source` it was lexed from
    pub fn text<'s>(&self, source: &'s str) -> &'s str {
        match self.variant {
            TokenVariant::Reserved(k) => k.as_str(),
            _ => &source[self.span.range()],
        }
    }

    /// The id and the optional value of a [TokenVariant::Tag]: the id is the text up to the
    /// first whitespace, the value is the rest.
    pub fn tag<'s>(&self, source: &'s str) -> (&'s str, Option<&'s str>) {
        let text = self.text(source);
        match text.split_once(char::is_whitespace) {
            Some((id, value)) => (id, Some(value.trim_start())),
            None => (text, None),
        }
    }
}

pub const INFORMAL_DELIMITER: Lexeme = Lexeme::Word("@@");
pub const TAG_DELIMITER: Lexeme<'_> = Lexeme::Punctation('#');
pub const UNIT_DELIMITER: Lexeme<'_> = Lexeme::Punctation('\\');

#[inline]
pub fn is_reserved(c: char) -> bool {
    match c {
        '\\' => true,
        '#' => true,
        _ => false,
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Mode {
    Other,
    Expression(Span),
    Type(Span),
    Unit(Span),
    Informal(Span),
    TagId(Span),
    TagValue(Span, Span),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Lexeme<'a> {
    Punctation(char),
    Word(&'a str),
}

pub struct LexemeIterator<'a> {
    positions: PositionIterator<'a>,
    input: &'a str,
    last: Option<(Span, Lexeme<'a>)>,
}

impl<'a> Iterator for LexemeIterator<'a> {
    type Item = (Span, Lexeme<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        use Lexeme::*;

        let mut has_word = false;

        if let Some(_) = self.last {
            return std::mem::take(&mut self.last);
        }

        while let Some((pos, c)) = self.positions.next() {
            if c.is_whitespace() {
                continue;
            }

            let start = pos;
            let mut position = (pos, c);
            loop {
                let (pos, c) = position;
                if is_reserved(c) {
                    let end = Position::next(pos);
                    let span = Span::new(pos, end);
                    let ret = (span, Punctation(c));
                    if !has_word {
                        return Some(ret);
                    }

                    self.last = Some(ret);
                    let span = Span::new(start, pos);
                    let range = span.range();
                    return Some((span, Word(&self.input[range])));
                }
                if c.is_whitespace() {
                    let span = Span::new(start, pos);
                    let range = span.range();
                    return Some((span, Word(&self.input[range])));
                }

                has_word = true;
                match self.positions.next() {
                    Some(p) => {
                        position = p;
                    }

                    None => {
                        let span = Span::new(start, Position::next(pos));
                        let range = span.range();
                        return Some((span, Word(&self.input[range])));
                    }
                }
            }
        }

        None
    }
}

pub trait IntoLexeme<'a> {
    fn lexemes(self) -> LexemeIterator<'a>;
}

impl<'a> IntoLexeme<'a> for PositionIterator<'a> {
    fn lexemes(self) -> LexemeIterator<'a> {
        LexemeIterator {
            input: self.input(),
            positions: self,
            last: None,
        }
    }
}

pub struct TokenIterator<'a> {
    lexemes: LexemeIterator<'a>,
    mode: Mode,
    last: Option<Token>,
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        use Mode::*;

        if let Some(_) = self.last {
            return std::mem::take(&mut self.last);
        }

        while let Some((span, l)) = self.lexemes.next() {
            match self.mode {
                Other => {
                    if l == INFORMAL_DELIMITER {
                        if let Some((start, l)) = self.lexemes.next() {
                            if l != INFORMAL_DELIMITER {
                                self.mode = Informal(start);
                                continue;
                            }

                            let mut span = start;
                            span.end = span.start;
                            let variant = TokenVariant::Markup;
                            return Some(Token::new(span, variant));
                        }

                        continue;
                    }

                    if l == TAG_DELIMITER {
                        if let Some((span, l)) = self.lexemes.next() {
                            if l != TAG_DELIMITER {
                                self.mode = TagId(span);
                            }
                            continue;
                        }
                    }

                    match Keyword::try_from(l) {
                        Err(_) => {
                            return Some(Token::new(span, TokenVariant::Identifier));
                        }

                        Ok(k) => {
                            match k {
                                Is | In => {
                                    if let Some((span, l)) = self.lexemes.next() {
                                        if l == INFORMAL_DELIMITER {
                                            if let Some((start, l)) = self.lexemes.next() {
                                                if l == INFORMAL_DELIMITER {
                                                    let mut span = start;
                                                    span.end = span.start;
                                                    let variant = TokenVariant::Markup;
                                                    self.last = Some(Token::new(span, variant));

                                                    let variant = TokenVariant::Reserved(k);
                                                    return Some(Token::new(span, variant));
                                                }

                                                self.mode = Informal(start);
                                            }
                                        } else {
                                            self.mode = Expression(span);
                                        }
                                    }
                                }

                                Has => {
                                    if let Some((start, l)) = self.lexemes.next() {
                                        if let Ok(Keyword::Type) = Keyword::try_from(l) {
                                            let variant = TokenVariant::Reserved(Keyword::Type);
                                            self.last = Some(Token::new(start, variant));

                                            let variant = TokenVariant::Reserved(Has);
                                            return Some(Token::new(span, variant));
                                        }
                                        let mut span = start;
                                        span.end = span.start;
                                        self.mode = Type(span)
                                    }
                                }

                                _ => {}
                            }

                            return Some(Token::new(span, TokenVariant::Reserved(k)));
                        }
                    }
                }

                Expression(mut current) => {
                    if l == UNIT_DELIMITER {
                        let variant = TokenVariant::Expression;
                        let ret = Token::new(current, variant);
                        if let Some((span, _)) = self.lexemes.next() {
                            self.mode = Unit(span);
                        }
                        return Some(ret);
                    }

                    if l == INFORMAL_DELIMITER {
                        let variant = TokenVariant::Expression;
                        let ret = Token::new(current, variant);
                        if let Some((start, l)) = self.lexemes.next() {
                            if l != INFORMAL_DELIMITER {
                                self.mode = Informal(start);
                                return Some(ret);
                            }

                            let mut span = start;
                            span.end = span.start;
                            let variant = TokenVariant::Markup;
                            self.last = Some(Token::new(span, variant));
                            self.mode = Other;
                            return Some(ret);
                        }
                    }

                    if l == TAG_DELIMITER {
                        let variant = TokenVariant::Expression;
                        let ret = Token::new(current, variant);

                        if let Some((span, l)) = self.lexemes.next() {
                            if l != TAG_DELIMITER {
                                self.mode = TagId(span);
                                return Some(ret);
                            }
                        }
                        self.mode = Other;
                        return Some(ret);
                    }

                    if let Ok(first) = Keyword::try_from(l) {
                        match first {
                            Package | Part | Requirement | Let => {
                                let variant = TokenVariant::Expression;
                                let ret = Token::new(current, variant);
                                self.last = Some(Token::new(span, TokenVariant::Reserved(first)));
                                self.mode = Other;
                                return Some(ret);
                            }
                            _ => {}
                        }
                    }

                    current.end = span.end;
                    self.mode = Expression(current);
                }

                Type(mut current) => {
                    if let Ok(first) = Keyword::try_from(l) {
                        match first {
                            Keyword::Type => {
                                let variant = TokenVariant::Type;
                                let ret = Token::new(current, variant);
                                self.last = Some(Token::new(span, TokenVariant::Reserved(first)));
                                self.mode = Other;
                                return Some(ret);
                            }
                            _ => {}
                        }
                    }

                    current.end = span.end;
                    self.mode = Type(current);
                }

                Unit(mut current) => {
                    if l == UNIT_DELIMITER {
                        let variant = TokenVariant::Unit;
                        let ret = Token::new(current, variant);
                        self.mode = Other;
                        return Some(ret);
                    }

                    current.end = span.end;
                    self.mode = Unit(current);
                }

                Informal(mut current) => {
                    if l == INFORMAL_DELIMITER {
                        let variant = TokenVariant::Markup;
                        let ret = Token::new(current, variant);
                        self.mode = Other;
                        return Some(ret);
                    }

                    current.end = span.end;
                    self.mode = Informal(current);
                }

                TagId(current) => {
                    if l == TAG_DELIMITER {
                        let variant = TokenVariant::Tag;
                        let ret = Token::new(current, variant);
                        self.mode = Other;
                        return Some(ret);
                    }

                    self.mode = TagValue(current, span);
                }

                TagValue(mut id, mut value) => {
                    if l == TAG_DELIMITER {
                        let variant = TokenVariant::Tag;
                        id.end = value.end;
                        let ret = Token::new(id, variant);
                        self.mode = Other;
                        return Some(ret);
                    }

                    value.end = span.end;
                    self.mode = TagValue(id, value);
                }
            }
        }
        None
    }
}

pub trait IntoTokens<'a> {
    fn tokens(self) -> TokenIterator<'a>;
}

impl<'a> IntoTokens<'a> for LexemeIterator<'a> {
    fn tokens(self) -> TokenIterator<'a> {
        TokenIterator {
            lexemes: self,
            mode: Mode::Other,
            last: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::syntax::{
        location::{Position, Span},
        tokenizer::entity::{
            Keyword::{self, *},
            TokenVariant, tokenize,
        },
    };

    /// The variant and the text of each token
    fn lex(input: &str) -> Vec<(TokenVariant, &str)> {
        let tokens = tokenize(input);
        tokens.map(|t| (t.variant, t.text(input))).collect()
    }

    fn kw(k: Keyword) -> (TokenVariant, &'static str) {
        (TokenVariant::Reserved(k), k.as_str())
    }

    fn id(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Identifier, s)
    }

    fn expr(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Expression, s)
    }

    fn markup(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Markup, s)
    }

    fn typ(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Type, s)
    }

    fn unit(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Unit, s)
    }

    fn tag(s: &str) -> (TokenVariant, &str) {
        (TokenVariant::Tag, s)
    }

    #[test]
    fn splits_tags_at_the_first_whitespace() {
        let input = "# id some value # # alone # part";
        let tags: Vec<_> = tokenize(input)
            .filter(|t| t.variant == TokenVariant::Tag)
            .map(|t| t.tag(input))
            .collect();
        assert_eq!(tags, [("id", Some("some value")), ("alone", None)]);
    }

    #[test]
    fn produces_correct_spans() {
        let input = "package x package\nis testing something part";
        let tokens = tokenize(input);
        assert_eq!(
            tokens.into_iter().map(|x| x.span).collect::<Vec<_>>(),
            [
                Span::new(Position::new(0, 0, 0), Position::new(0, 7, 7)),
                Span::new(Position::new(0, 8, 8), Position::new(0, 9, 9)),
                Span::new(Position::new(0, 10, 10), Position::new(0, 17, 17)),
                Span::new(Position::new(1, 0, 18), Position::new(1, 2, 20)),
                Span::new(Position::new(1, 3, 21), Position::new(1, 20, 38)),
                Span::new(Position::new(1, 21, 39), Position::new(1, 25, 43)),
            ]
        );
    }

    #[test]
    fn ignore_keyword_within_comments() {
        let input = "@@ hello requirement is word @@ requirement is a requirement";
        assert_eq!(
            lex(input),
            [
                markup("hello requirement is word"),
                kw(Requirement),
                kw(Is),
                expr("a"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn ignore_at_symbols_within_requirements() {
        let input = "@@ hello @ word @@ requirement is @@ @ @@ requirement";
        assert_eq!(
            lex(input),
            [
                markup("hello @ word"),
                kw(Requirement),
                kw(Is),
                markup("@"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn tokenizes_simple_package() {
        let input = "package x package";
        assert_eq!(lex(input), [kw(Package), id("x"), kw(Package)]);
    }

    #[test]
    fn tokenizes_comment() {
        let input = "@@ something commenting @@ package x package";
        assert_eq!(
            lex(input),
            [
                markup("something commenting"),
                kw(Package),
                id("x"),
                kw(Package)
            ]
        );
    }

    #[test]
    fn tokenizes_informal_requirement() {
        let input = "requirement x is @@ word @@ requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("x"),
                kw(Is),
                markup("word"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn tokenizes_formal_requirement() {
        let input = "requirement x is y = z requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("x"),
                kw(Is),
                expr("y = z"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn tokenizes_tags() {
        let input = "# something commenting # package x package";
        assert_eq!(
            lex(input),
            [
                tag("something commenting"),
                kw(Package),
                id("x"),
                kw(Package)
            ]
        );
    }

    #[test]
    fn tokenizes_traced_requirement() {
        let input = "requirement a on y via z refines b is y = 1 requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("a"),
                kw(On),
                id("y"),
                kw(Via),
                id("z"),
                kw(Refines),
                id("b"),
                kw(Is),
                expr("y = 1"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn tokenizes_expression() {
        let input = "part x let y in real part";
        assert_eq!(
            lex(input),
            [
                kw(Part),
                id("x"),
                kw(Let),
                id("y"),
                kw(In),
                expr("real"),
                kw(Part)
            ]
        );
    }

    #[test]
    fn tokenizes_type() {
        let input = "type t has a | b | c type";
        assert_eq!(
            lex(input),
            [kw(Type), id("t"), kw(Has), typ("a | b | c"), kw(Type)]
        )
    }

    #[test]
    fn tokenizes_empty_type() {
        let input = "type t has type";
        assert_eq!(lex(input), [kw(Type), id("t"), kw(Has), kw(Type)])
    }

    #[test]
    fn tokenizes_expression_with_unit() {
        let input = "part x let y is real \\m\\ part";
        assert_eq!(
            lex(input),
            [
                kw(Part),
                id("x"),
                kw(Let),
                id("y"),
                kw(Is),
                expr("real"),
                unit("m"),
                kw(Part)
            ]
        );
    }

    #[test]
    fn tokenizes_expressions() {
        let input = "part x let y is real let z is real part";
        assert_eq!(
            lex(input),
            [
                kw(Part),
                id("x"),
                kw(Let),
                id("y"),
                kw(Is),
                expr("real"),
                kw(Let),
                id("z"),
                kw(Is),
                expr("real"),
                kw(Part)
            ]
        );
    }

    #[test]
    fn ends_expressions_with_markup() {
        let input = "is x @@ @@ part";
        assert_eq!(lex(input), [kw(Is), expr("x"), markup(""), kw(Part)]);
    }

    #[test]
    fn ends_expressions_with_tag() {
        let input = "is x # # part";
        assert_eq!(lex(input), [kw(Is), expr("x"), kw(Part)]);
    }

    #[test]
    fn tokenizes_expressions_with_unit() {
        let input = "part x let y is real \\m\\ let z is real \\m\\ part";
        assert_eq!(
            lex(input),
            [
                kw(Part),
                id("x"),
                kw(Let),
                id("y"),
                kw(Is),
                expr("real"),
                unit("m"),
                kw(Let),
                id("z"),
                kw(Is),
                expr("real"),
                unit("m"),
                kw(Part)
            ]
        );
    }

    #[test]
    fn produces_empty_markup() {
        let input = "requirement x is @@ \n @@ requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("x"),
                kw(Is),
                markup(""),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn produces_simple_traceability() {
        let input = "requirement x derives y is a requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("x"),
                kw(Derives),
                id("y"),
                kw(Is),
                expr("a"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn produces_multiple_traceability() {
        let input = "requirement x refines y specializes z is a requirement";
        assert_eq!(
            lex(input),
            [
                kw(Requirement),
                id("x"),
                kw(Refines),
                id("y"),
                kw(Specializes),
                id("z"),
                kw(Is),
                expr("a"),
                kw(Requirement)
            ]
        );
    }

    #[test]
    fn ignores_empty_tags() {
        let input = "# # package x package";
        assert_eq!(lex(input), [kw(Package), id("x"), kw(Package)]);
    }

    #[test]
    fn tokenizes_complex_input() {
        let input = "package p part x let y is real let z is boolean part requirement u refines v is y = z requirement package";
        assert_eq!(
            lex(input),
            [
                kw(Package),
                id("p"),
                kw(Part),
                id("x"),
                kw(Let),
                id("y"),
                kw(Is),
                expr("real"),
                kw(Let),
                id("z"),
                kw(Is),
                expr("boolean"),
                kw(Part),
                kw(Requirement),
                id("u"),
                kw(Refines),
                id("v"),
                kw(Is),
                expr("y = z"),
                kw(Requirement),
                kw(Package)
            ]
        );
    }
}
