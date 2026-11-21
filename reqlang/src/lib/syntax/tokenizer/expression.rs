use crate::{
    syntax::location::{IntoPosition, Position, PositionIterator, Span, SpanLike},
    utils::cursor::Kinded,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Number,
    Identifier,
    Keyword(Keyword),
    Operator(Operator),
    Punctuation(Punctuation),
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Remainder,
    Power,
    Factorial,
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    NotEqual,
    Equal,
    Fix,
    Initial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punctuation {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    DoubleLeftBracket,
    DoubleRightBracket,
    Comma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    And,
    Or,
    Xor,
    Implies,
    Iff,
    In,
    Tol,
    Union,
    Intersection,
    Difference,
    Complement,
    Includes,
    Since,
    Not,
    Previously,
    Rising,
    Falling,
    Eventually,
    Always,
    When,
    Then,
    End,
    Such,
    That,
    For,
    True,
    False,
    Undefined,
    Otherwise,
    Infinity,
    All,
    Any,
    One,
    Max,
    Min,
    Sum,
    Prod,
    ArgMax,
    ArgMin,
    Sup,
    Inf,
    Count,
    Choose,
}

impl<'a> TryFrom<&'a str> for Keyword {
    type Error = &'a str;

    fn try_from(word: &'a str) -> Result<Self, Self::Error> {
        use Keyword::*;

        match word {
            "and" => Ok(And),
            "or" => Ok(Or),
            "xor" => Ok(Xor),
            "implies" => Ok(Implies),
            "iff" => Ok(Iff),
            "in" => Ok(In),
            "tol" => Ok(Tol),
            "union" => Ok(Union),
            "intersection" => Ok(Intersection),
            "difference" => Ok(Difference),
            "complement" => Ok(Complement),
            "includes" => Ok(Includes),
            "since" => Ok(Since),
            "not" => Ok(Not),
            "previously" => Ok(Previously),
            "rising" => Ok(Rising),
            "falling" => Ok(Falling),
            "eventually" => Ok(Eventually),
            "always" => Ok(Always),
            "when" => Ok(When),
            "then" => Ok(Then),
            "end" => Ok(End),
            "such" => Ok(Such),
            "that" => Ok(That),
            "for" => Ok(For),
            "true" => Ok(True),
            "false" => Ok(False),
            "undefined" => Ok(Undefined),
            "otherwise" => Ok(Otherwise),
            "infinity" => Ok(Infinity),
            "all" => Ok(All),
            "any" => Ok(Any),
            "one" => Ok(One),
            "max" => Ok(Max),
            "min" => Ok(Min),
            "sum" => Ok(Sum),
            "prod" => Ok(Prod),
            "argmax" => Ok(ArgMax),
            "argmin" => Ok(ArgMin),
            "sup" => Ok(Sup),
            "inf" => Ok(Inf),
            "count" => Ok(Count),
            "choose" => Ok(Choose),
            word => Err(word),
        }
    }
}

impl Keyword {
    pub fn as_str(&self) -> &'static str {
        use Keyword::*;

        match self {
            And => "and",
            Or => "or",
            Xor => "xor",
            Implies => "implies",
            Iff => "iff",
            In => "in",
            Tol => "tol",
            Union => "union",
            Intersection => "intersection",
            Difference => "difference",
            Complement => "complement",
            Includes => "includes",
            Since => "since",
            Not => "not",
            Previously => "previously",
            Rising => "rising",
            Falling => "falling",
            Eventually => "eventually",
            Always => "always",
            When => "when",
            Then => "then",
            End => "end",
            Such => "such",
            That => "that",
            For => "for",
            True => "true",
            False => "false",
            Undefined => "undefined",
            Otherwise => "otherwise",
            Infinity => "infinity",
            All => "all",
            Any => "any",
            One => "one",
            Max => "max",
            Min => "min",
            Sum => "sum",
            Prod => "prod",
            ArgMax => "argmax",
            ArgMin => "argmin",
            Sup => "sup",
            Inf => "inf",
            Count => "count",
            Choose => "choose",
        }
    }
}

#[cfg(test)]
pub const KEYWORDS: [Keyword; 43] = {
    use Keyword::*;
    [
        And,
        Or,
        Xor,
        Implies,
        Iff,
        In,
        Tol,
        Union,
        Intersection,
        Difference,
        Complement,
        Includes,
        Since,
        Not,
        Previously,
        Rising,
        Falling,
        Eventually,
        Always,
        When,
        Then,
        End,
        Such,
        That,
        For,
        True,
        False,
        Undefined,
        Otherwise,
        Infinity,
        All,
        Any,
        One,
        Max,
        Min,
        Sum,
        Prod,
        ArgMax,
        ArgMin,
        Sup,
        Inf,
        Count,
        Choose,
    ]
};

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

impl Kinded for Token {
    type Kind = Kind;

    fn kind(&self) -> Kind {
        self.kind
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

fn is_word_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

impl Iterator for TokenIterator<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        loop {
            let (start, c) = self.peek()?;
            let kind = match c {
                ' ' | '\t' | '\n' | '\r' => {
                    self.advance();
                    continue;
                }
                c if is_word_start(c) => self.word(),
                '0'..='9' => self.number(),
                c => self.symbol(c),
            };
            let end = self.here();
            return Some(Token {
                kind,
                span: Span::new(start, end),
            });
        }
    }
}

impl TokenIterator<'_> {
    fn peek(&self) -> Option<(Position, char)> {
        self.first
    }

    fn peek1(&self) -> Option<(Position, char)> {
        self.second
    }

    fn char(&self) -> Option<char> {
        self.peek().map(|(_, c)| c)
    }

    fn char1(&self) -> Option<char> {
        self.peek1().map(|(_, c)| c)
    }

    fn advance(&mut self) {
        self.first = self.second;
        self.second = self.positions.next();
    }

    // Where the next char is, which is where the token that was just read ends
    fn here(&self) -> Position {
        self.peek()
            .map_or_else(|| self.positions.current(), |(position, _)| position)
    }

    fn symbol(&mut self, c: char) -> Kind {
        use Operator::*;
        use Punctuation::*;

        let next = self.char1();
        let (kind, len) = match c {
            '+' => (Kind::Operator(Plus), 1),
            '-' => (Kind::Operator(Minus), 1),
            '*' => (Kind::Operator(Multiply), 1),
            '/' => (Kind::Operator(Divide), 1),
            '%' => (Kind::Operator(Remainder), 1),
            '^' => (Kind::Operator(Power), 1),
            '!' => (Kind::Operator(Factorial), 1),
            '=' => (Kind::Operator(Equal), 1),
            '>' if next == Some('=') => (Kind::Operator(GreaterOrEqual), 2),
            '>' => (Kind::Operator(Greater), 1),
            '<' if next == Some('=') => (Kind::Operator(LessOrEqual), 2),
            '<' if next == Some('>') => (Kind::Operator(NotEqual), 2),
            '<' => (Kind::Operator(Less), 1),
            ':' if next == Some('=') => (Kind::Operator(Fix), 2),
            '°' if next == Some('=') => (Kind::Operator(Initial), 2),
            '(' => (Kind::Punctuation(LeftParen), 1),
            ')' => (Kind::Punctuation(RightParen), 1),
            '{' => (Kind::Punctuation(LeftBrace), 1),
            '}' => (Kind::Punctuation(RightBrace), 1),
            '[' if next == Some('[') => (Kind::Punctuation(DoubleLeftBracket), 2),
            ']' if next == Some(']') => (Kind::Punctuation(DoubleRightBracket), 2),
            '[' => (Kind::Punctuation(LeftBracket), 1),
            ']' => (Kind::Punctuation(RightBracket), 1),
            ',' => (Kind::Punctuation(Comma), 1),
            _ => (Kind::Error, 1),
        };
        for _ in 0..len {
            self.advance();
        }
        kind
    }

    fn word(&mut self) -> Kind {
        let mut word = String::new();
        let mut path = false;
        loop {
            while let Some(c) = self.char().filter(|c| is_word_char(*c)) {
                word.push(c);
                self.advance();
            }
            if self.char() != Some(':') || self.char1() != Some(':') {
                break;
            }
            path = true;
            word.push_str("::");
            self.advance();
            self.advance();
            if !self.char().is_some_and(is_word_start) {
                break;
            }
        }

        match Keyword::try_from(word.as_str()) {
            Ok(keyword) if !path => Kind::Keyword(keyword),
            _ => Kind::Identifier,
        }
    }

    fn digits(&mut self) {
        while self.char().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }
    }

    fn number(&mut self) -> Kind {
        self.digits();
        if self.char() == Some('.') {
            self.advance();
            self.digits();
        }
        if matches!(self.char(), Some('e' | 'E'))
            && self.char1().is_some_and(|c| c.is_ascii_digit())
        {
            self.advance();
            self.digits();
        }
        Kind::Number
    }
}

#[cfg(test)]
mod tests {
    use super::{Keyword::*, Operator::*, Punctuation::*, *};

    fn lex(input: &str) -> Vec<(Kind, &str)> {
        tokenize(input).map(|t| (t.kind, t.text(input))).collect()
    }

    fn kinds(input: &str) -> Vec<Kind> {
        tokenize(input).map(|t| t.kind).collect()
    }

    #[test]
    fn keywords_are_whole_words() {
        for keyword in KEYWORDS {
            let word = keyword.as_str();
            assert_eq!(lex(word), [(Kind::Keyword(keyword), word)]);
            assert_eq!(Keyword::try_from(word), Ok(keyword));
            let longer = format!("{word}x");
            assert_eq!(lex(&longer), [(Kind::Identifier, longer.as_str())]);
        }
    }

    #[test]
    fn aggregators_are_keywords() {
        assert_eq!(
            kinds("sum union intersection argmax"),
            [
                Kind::Keyword(Sum),
                Kind::Keyword(Union),
                Kind::Keyword(Intersection),
                Kind::Keyword(ArgMax)
            ]
        );
    }

    #[test]
    fn operators_take_the_longest_match() {
        assert_eq!(
            lex(">= <= <> := °= > < = ! + - * / % ^"),
            [
                (Kind::Operator(GreaterOrEqual), ">="),
                (Kind::Operator(LessOrEqual), "<="),
                (Kind::Operator(NotEqual), "<>"),
                (Kind::Operator(Fix), ":="),
                (Kind::Operator(Initial), "°="),
                (Kind::Operator(Greater), ">"),
                (Kind::Operator(Less), "<"),
                (Kind::Operator(Equal), "="),
                (Kind::Operator(Factorial), "!"),
                (Kind::Operator(Plus), "+"),
                (Kind::Operator(Minus), "-"),
                (Kind::Operator(Multiply), "*"),
                (Kind::Operator(Divide), "/"),
                (Kind::Operator(Remainder), "%"),
                (Kind::Operator(Power), "^"),
            ]
        );
    }

    #[test]
    fn numbers() {
        for number in ["0", "12", "1.", "1.5", "1e5", "1.5E3", "2.e10"] {
            assert_eq!(lex(number), [(Kind::Number, number)]);
        }
        // An exponent needs digits
        assert_eq!(lex("1e"), [(Kind::Number, "1"), (Kind::Identifier, "e")]);
        assert_eq!(
            lex("1e-5"),
            [
                (Kind::Number, "1"),
                (Kind::Identifier, "e"),
                (Kind::Operator(Minus), "-"),
                (Kind::Number, "5")
            ]
        );
    }

    #[test]
    fn paths() {
        assert_eq!(lex("a::b::c"), [(Kind::Identifier, "a::b::c")]);
        assert_eq!(lex("in::and"), [(Kind::Identifier, "in::and")]);
        assert_eq!(lex("a::"), [(Kind::Identifier, "a::")]);
        assert_eq!(
            lex("a::1"),
            [(Kind::Identifier, "a::"), (Kind::Number, "1")]
        );
        assert_eq!(
            lex("::x"),
            [
                (Kind::Error, ":"),
                (Kind::Error, ":"),
                (Kind::Identifier, "x")
            ]
        );
        assert_eq!(
            lex("x:=1"),
            [
                (Kind::Identifier, "x"),
                (Kind::Operator(Fix), ":="),
                (Kind::Number, "1")
            ]
        );
    }

    #[test]
    fn double_brackets_are_their_own_tokens() {
        assert_eq!(
            lex("[[1, 2]]"),
            [
                (Kind::Punctuation(DoubleLeftBracket), "[["),
                (Kind::Number, "1"),
                (Kind::Punctuation(Comma), ","),
                (Kind::Number, "2"),
                (Kind::Punctuation(DoubleRightBracket), "]]"),
            ]
        );
        assert_eq!(
            kinds("[ [ ] ]"),
            [
                Kind::Punctuation(LeftBracket),
                Kind::Punctuation(LeftBracket),
                Kind::Punctuation(RightBracket),
                Kind::Punctuation(RightBracket),
            ]
        );
    }

    #[test]
    fn unknown_chars_are_errors() {
        assert_eq!(
            lex("a @ ° :"),
            [
                (Kind::Identifier, "a"),
                (Kind::Error, "@"),
                (Kind::Error, "°"),
                (Kind::Error, ":"),
            ]
        );
    }

    #[test]
    fn identifiers_can_be_unicode() {
        assert_eq!(
            lex("température::é + x2"),
            [
                (Kind::Identifier, "température::é"),
                (Kind::Operator(Plus), "+"),
                (Kind::Identifier, "x2"),
            ]
        );
    }

    #[test]
    fn incomplete_input_is_lexed() {
        assert_eq!(
            lex("1 >"),
            [(Kind::Number, "1"), (Kind::Operator(Greater), ">")]
        );
        assert_eq!(
            lex("f(a,"),
            [
                (Kind::Identifier, "f"),
                (Kind::Punctuation(LeftParen), "("),
                (Kind::Identifier, "a"),
                (Kind::Punctuation(Comma), ","),
            ]
        );
        assert_eq!(lex(""), []);
        assert_eq!(lex("  \n "), []);
    }

    #[test]
    fn tokens_cover_everything_but_whitespace() {
        for input in [
            "",
            "a + b",
            "a ::\n b",
            "when x > 1 then 2, otherwise é end",
            "[[1, 2]] °= {x for x in S}",
            "1.5e3 @ ! :",
        ] {
            let tokens: Vec<_> = tokenize(input).collect();
            let text: String = tokens.iter().map(|t| t.text(input)).collect();
            let expected: String = input.chars().filter(|c| !c.is_whitespace()).collect();
            assert_eq!(text, expected, "{input:?}");
            assert!(
                tokens
                    .windows(2)
                    .all(|w| w[0].span.end.index <= w[1].span.start.index)
            );
        }
    }

    #[test]
    fn the_tokens_end_with_the_text() {
        for input in ["", "a", "a ::", "1.", "é", "°", "[[", "x :", "\n\n"] {
            let count = tokenize(input).take(input.len() + 2).count();
            assert!(count <= input.len(), "{input:?}: {count} tokens");
        }
    }

    #[test]
    fn positions_are_relative_to_the_text() {
        let input = "ab\n  é + c";
        let plus = tokenize(input)
            .find(|t| t.kind == Kind::Operator(Plus))
            .unwrap();
        assert_eq!(plus.span.start, Position::new(1, 4, 8));
        assert_eq!(plus.span.end, Position::new(1, 5, 9));
    }
}
