use crate::{
    node::expression::{self, *},
    syntax::{
        location::Span,
        parser::{identifier, path},
        tokenizer::expression::{Keyword, Kind, Operator, Punctuation, Token, tokenize},
    },
};

pub type TokenArena = super::TokenArena<Token, IdxMarker>;
pub type ParserArena = super::ParserArena<Token, Node, IdxMarker>;

impl super::Parser<'_> {
    pub(super) fn expression(&mut self, span: Span) -> Option<Idx> {
        let mark = self.mark();
        let input = &self.source[span.range()];
        let (expressions, errors) = parse(input, &mut self.identifiers);
        let root = match errors.is_empty() {
            true => self.expressions.extend(expressions),
            false => {
                let toks = expressions.tokens.into_tokens();
                self.expressions.tokens.extend(toks.into_iter());
                None
            }
        };
        self.relocate(mark, span.start);
        self.errors
            .extend(errors.into_iter().map(|error| error + span.start));
        root
    }
}

// The root is the last node, there are no nodes to use when there is an error
pub fn parse(input: &str, identifiers: &mut identifier::ParserArena) -> (ParserArena, Vec<Span>) {
    let expressions = ParserArena::from(tokenize(input));

    let mut parser = Parser {
        input,
        expressions,
        identifiers,
    };
    let result = match (parser.expression(0), parser.expressions.tokens.peek()) {
        (Some(_), None) => Some(()),
        _ => None,
    };

    let errors = match result {
        Some(()) => Vec::new(),
        None => {
            let error = match parser.expressions.tokens.peek() {
                Some(token) => token.span,
                None => {
                    let end = parser
                        .expressions
                        .tokens
                        .last()
                        .map_or_else(Default::default, |last| last.span.end);
                    Span::new(end, end)
                }
            };
            vec![error]
        }
    };
    (parser.expressions, errors)
}

struct Parser<'a> {
    input: &'a str,
    expressions: ParserArena,
    identifiers: &'a mut identifier::ParserArena,
}

fn cover(a: std::ops::Range<usize>, b: std::ops::Range<usize>) -> std::ops::Range<usize> {
    a.start.min(b.start)..a.end.max(b.end)
}

impl Parser<'_> {
    fn expression(&mut self, rbp: i32) -> Option<Idx> {
        let start = self.expressions.tokens.position();
        let mut lhs = match self
            .expressions
            .tokens
            .peek()
            .map(|token| UnaryOperator::try_from(token.kind))
        {
            Some(Ok(operator)) => {
                self.expressions.tokens.bump();
                let operand = self.expression(operator.precedence() - 1)?;
                let range = start..self.expressions.range_of(operand).end;
                let node = Node::new(Variant::UnaryOp(UnaryOp::new(operator, operand)));
                Some(self.expressions.push(node, range))
            }
            _ => self.primary(),
        }?;

        while let Some(Token { kind, .. }) = self.expressions.tokens.peek() {
            if let Ok(operator) = BinaryOperator::try_from(kind) {
                let precedence = operator.precedence();
                if precedence <= rbp {
                    break;
                }
                self.expressions.tokens.bump();
                let rbp = match operator.associativity() {
                    Associativity::Left => precedence,
                    Associativity::Right => precedence - 1,
                };
                let rhs = self.expression(rbp)?;
                let range = cover(
                    self.expressions.range_of(lhs),
                    self.expressions.range_of(rhs),
                );
                let node = Node::new(Variant::BinOp(BinOp::new(operator, lhs, rhs)));
                lhs = self.expressions.push(node, range);
            } else if kind == Kind::Operator(Operator::Factorial) {
                if UnaryOperator::Factorial.precedence() <= rbp {
                    break;
                }
                self.expressions.tokens.bump();
                let range =
                    self.expressions.range_of(lhs).start..self.expressions.tokens.position();
                let node = Node::new(Variant::UnaryOp(UnaryOp::new(
                    UnaryOperator::Factorial,
                    lhs,
                )));
                lhs = self.expressions.push(node, range);
            } else if kind == Kind::Punctuation(Punctuation::LeftParen) {
                if FUNCTION_PRECEDENCE <= rbp {
                    break;
                }
                lhs = self.call(lhs)?;
            } else {
                break;
            }
        }
        Some(lhs)
    }

    fn primary(&mut self) -> Option<Idx> {
        use Keyword::*;
        use Punctuation::*;

        let token = self.expressions.tokens.peek()?;
        let start = self.expressions.tokens.position();
        match token.kind {
            Kind::Number => {
                self.expressions.tokens.bump();
                let mut number = Number::new(NumberKind::Real);
                number.value = token.text(self.input).to_string();
                let node = Node::new(Variant::Number(number));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Keyword(Infinity) => {
                self.expressions.tokens.bump();
                let mut number = Number::new(NumberKind::Infinity);
                number.value = token.text(self.input).to_string();
                let node = Node::new(Variant::Number(number));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Keyword(True) | Kind::Keyword(False) => {
                self.expressions.tokens.bump();
                let value = Boolean::from(token.kind == Kind::Keyword(True));
                let node = Node::new(Variant::Boolean(value));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Keyword(Undefined) => {
                self.expressions.tokens.bump();
                let node = Node::new(Variant::Undefined);
                Some(self.expressions.push_from(node, start))
            }
            Kind::Identifier => {
                self.expressions.tokens.bump();

                let (path, tokens) = path::parse(token.text(self.input), token.span);
                let first = self.identifiers.tokens.len();
                self.identifiers.tokens.extend(tokens.into_iter());
                let range = first..self.identifiers.tokens.len();
                let target = self.identifiers.push(path, range);

                let node = Node::new(Variant::Identifier(Identifier::new(target)));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Punctuation(LeftBrace) => {
                let start = self.expressions.tokens.position();
                self.expressions.tokens.bump();
                let first = self.expression(0)?;
                let variant = if (self.expressions.tokens.peek())
                    .is_some_and(|t| t.kind == Kind::Keyword(Keyword::For))
                {
                    let element = self.binding()?;
                    Variant::Comprehension(Comprehension::new(None, element, first))
                } else {
                    let mut elements = vec![first];
                    elements.extend(self.rest()?);
                    Variant::Set(Set { elements })
                };
                self.expressions
                    .tokens
                    .expect(Kind::Punctuation(RightBrace))
                    .ok()?;
                let node = Node::new(variant);
                Some(self.expressions.push_from(node, start))
            }
            Kind::Punctuation(DoubleLeftBracket) => {
                let start = self.expressions.tokens.position();
                self.expressions.tokens.bump();
                let left = self.expression(0)?;
                self.expressions
                    .tokens
                    .expect(Kind::Punctuation(Comma))
                    .ok()?;
                let right = self.expression(0)?;
                (self.expressions.tokens)
                    .expect(Kind::Punctuation(DoubleRightBracket))
                    .ok()?;
                let node = Node::new(Variant::Range(Range::new(left, right)));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Punctuation(LeftBracket | RightBracket) => {
                use Punctuation::*;

                let start = self.expressions.tokens.position();
                let opening = self.expressions.tokens.bump().map(|token| token.kind);
                let left = self.expression(0)?;
                self.expressions
                    .tokens
                    .expect(Kind::Punctuation(Comma))
                    .ok()?;
                let right = self.expression(0)?;
                let closing = match self.expressions.tokens.peek() {
                    Some(Token {
                        kind: kind @ Kind::Punctuation(LeftBracket | RightBracket),
                        ..
                    }) => kind,
                    _ => return None,
                };
                self.expressions.tokens.bump();

                let left = match opening {
                    Some(Kind::Punctuation(LeftBracket)) => IntervalBound::closed(left),
                    _ => IntervalBound::open(left),
                };
                let right = match closing {
                    Kind::Punctuation(LeftBracket) => IntervalBound::open(right),
                    _ => IntervalBound::closed(right),
                };
                let node = Node::new(Variant::Interval(Interval::new(left, right)));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Punctuation(LeftParen) => {
                let start = self.expressions.tokens.position();
                self.expressions.tokens.bump();
                let inner = self.expression(0)?;
                self.expressions
                    .tokens
                    .expect(Kind::Punctuation(RightParen))
                    .ok()?;
                let range = cover(
                    self.expressions.range_of(inner),
                    start..self.expressions.tokens.position(),
                );
                self.expressions.ranges[inner] = range;
                Some(inner)
            }
            Kind::Keyword(When) => {
                let start = self.expressions.tokens.position();
                self.expressions.tokens.bump();
                let mut when = expression::When::new(Vec::new());
                loop {
                    let condition = self.expression(0)?;
                    self.expressions.tokens.expect(Kind::Keyword(Then)).ok()?;
                    let consequence = self.expression(0)?;
                    when.cases.push(Case::new(condition, consequence));

                    if !self.expressions.tokens.eat(Kind::Punctuation(Comma)) {
                        break;
                    }
                    if self.expressions.tokens.eat(Kind::Keyword(Otherwise)) {
                        when.otherwise = Some(self.expression(0)?);
                        break;
                    }
                }
                self.expressions.tokens.expect(Kind::Keyword(End)).ok()?;
                let node = Node::new(Variant::When(when));
                Some(self.expressions.push_from(node, start))
            }
            Kind::Keyword(keyword) => match Aggregator::try_from(keyword) {
                Ok(aggregator) => {
                    let start = self.expressions.tokens.position();
                    self.expressions.tokens.bump();
                    let first = self.expression(0)?;
                    let variant = if (self.expressions.tokens.peek())
                        .is_some_and(|t| t.kind == Kind::Keyword(Keyword::For))
                    {
                        let element = self.binding()?;
                        Variant::Comprehension(Comprehension::new(Some(aggregator), element, first))
                    } else {
                        let mut expressions = vec![first];
                        expressions.extend(self.rest()?);
                        Variant::Aggregation(Aggregation {
                            aggregator,
                            expressions,
                        })
                    };
                    self.expressions.tokens.expect(Kind::Keyword(End)).ok()?;
                    let node = Node::new(variant);
                    Some(self.expressions.push_from(node, start))
                }
                Err(_) => None,
            },
            _ => None,
        }
    }

    fn binding(&mut self) -> Option<SetElement> {
        self.expressions.tokens.bump();
        let variable = match self.expressions.tokens.peek() {
            Some(token)
                if token.kind == Kind::Identifier && !token.text(self.input).contains("::") =>
            {
                self.expressions.tokens.bump();
                token.text(self.input).to_string()
            }
            _ => return None,
        };
        self.expressions
            .tokens
            .expect(Kind::Keyword(Keyword::In))
            .ok()?;
        let domain = self.expression(0)?;
        let mut element = SetElement::new(variable, domain);
        if self.expressions.tokens.eat(Kind::Keyword(Keyword::Such)) {
            self.expressions
                .tokens
                .expect(Kind::Keyword(Keyword::That))
                .ok()?;
            element.filter = Some(self.expression(0)?);
        }
        Some(element)
    }

    fn rest(&mut self) -> Option<Vec<Idx>> {
        let mut expressions = Vec::new();
        while self
            .expressions
            .tokens
            .eat(Kind::Punctuation(Punctuation::Comma))
        {
            expressions.push(self.expression(0)?);
        }
        Some(expressions)
    }

    fn call(&mut self, callee: Idx) -> Option<Idx> {
        self.expressions.tokens.bump();
        let mut function = Function::new(callee);
        if (self.expressions.tokens).peek().map(|t| t.kind)
            != Some(Kind::Punctuation(Punctuation::RightParen))
        {
            function.arguments.push(self.expression(0)?);
            function.arguments.extend(self.rest()?);
        }
        (self.expressions.tokens)
            .expect(Kind::Punctuation(Punctuation::RightParen))
            .ok()?;
        let range = self.expressions.range_of(callee).start..self.expressions.tokens.position();
        Some(
            self.expressions
                .push(Node::new(Variant::Function(function)), range),
        )
    }
}

impl TryFrom<Kind> for BinaryOperator {
    type Error = Kind;

    fn try_from(kind: Kind) -> Result<Self, Self::Error> {
        use BinaryOperator::*;

        match kind {
            Kind::Operator(operator) => match operator {
                Operator::Plus => Ok(Plus),
                Operator::Minus => Ok(Minus),
                Operator::Multiply => Ok(Multiply),
                Operator::Divide => Ok(Divide),
                Operator::Remainder => Ok(Modulus),
                Operator::Power => Ok(Power),
                Operator::Greater => Ok(GreaterThan),
                Operator::Less => Ok(LessThan),
                Operator::GreaterOrEqual => Ok(GreaterOrEqual),
                Operator::LessOrEqual => Ok(LessOrEqual),
                Operator::NotEqual => Ok(NotEqual),
                Operator::Equal => Ok(Equal),
                Operator::Fix => Ok(Fix),
                Operator::Initial => Ok(Initially),
                Operator::Factorial => Err(kind),
            },
            Kind::Keyword(keyword) => match keyword {
                Keyword::And => Ok(And),
                Keyword::Or => Ok(Or),
                Keyword::Xor => Ok(Xor),
                Keyword::Implies => Ok(Implies),
                Keyword::Iff => Ok(Iff),
                Keyword::In => Ok(In),
                Keyword::Tol => Ok(Tolerance),
                Keyword::Union => Ok(Union),
                Keyword::Intersection => Ok(Intersection),
                Keyword::Difference => Ok(Difference),
                Keyword::Complement => Ok(Complement),
                Keyword::Includes => Ok(Includes),
                Keyword::Since => Ok(Since),
                _ => Err(kind),
            },
            _ => Err(kind),
        }
    }
}

// The operators that come before their operand, `!` comes after it
impl TryFrom<Kind> for UnaryOperator {
    type Error = Kind;

    fn try_from(kind: Kind) -> Result<Self, Self::Error> {
        match kind {
            Kind::Operator(Operator::Plus) => Ok(UnaryOperator::Plus),
            Kind::Operator(Operator::Minus) => Ok(UnaryOperator::Negation),
            Kind::Keyword(Keyword::Not) => Ok(UnaryOperator::Not),
            Kind::Keyword(Keyword::Previously) => Ok(UnaryOperator::Previously),
            Kind::Keyword(Keyword::Rising) => Ok(UnaryOperator::Rise),
            Kind::Keyword(Keyword::Falling) => Ok(UnaryOperator::Fall),
            Kind::Keyword(Keyword::Eventually) => Ok(UnaryOperator::Eventually),
            Kind::Keyword(Keyword::Always) => Ok(UnaryOperator::Always),
            _ => Err(kind),
        }
    }
}

impl TryFrom<Keyword> for Aggregator {
    type Error = Keyword;

    fn try_from(keyword: Keyword) -> Result<Self, Self::Error> {
        match keyword {
            Keyword::All => Ok(Aggregator::All),
            Keyword::Any => Ok(Aggregator::Any),
            Keyword::One => Ok(Aggregator::One),
            Keyword::Max => Ok(Aggregator::Maximum),
            Keyword::Min => Ok(Aggregator::Minimum),
            Keyword::Sum => Ok(Aggregator::Sum),
            Keyword::Prod => Ok(Aggregator::Product),
            Keyword::ArgMax => Ok(Aggregator::ArgMaximum),
            Keyword::ArgMin => Ok(Aggregator::ArgMinimum),
            Keyword::Sup => Ok(Aggregator::Supremum),
            Keyword::Inf => Ok(Aggregator::Infimum),
            Keyword::Count => Ok(Aggregator::Count),
            Keyword::Choose => Ok(Aggregator::Choose),
            Keyword::Union => Ok(Aggregator::Union),
            Keyword::Intersection => Ok(Aggregator::Intersection),
            _ => Err(keyword),
        }
    }
}

// Whatever the input is, the nodes come after their children and are made of existing tokens
#[cfg(test)]
fn assert_well_formed(expressions: &ParserArena, input: &str) {
    use crate::utils::children::ChildrenIter;

    let tokens = expressions.tokens.len();
    for (idx, node) in expressions.nodes.enumerate() {
        for child in node.children_iter() {
            assert!(child < idx, "{input:?}: {child:?} is not before {idx:?}");
        }
        let range = expressions.range_of(idx);
        assert!(range.start <= range.end && range.end <= tokens, "{input:?}");
    }
}

#[cfg(test)]
mod tests {
    use crate::node::{self, path::Path};

    use super::*;

    fn parse_alone(
        input: &str,
    ) -> Result<
        (
            node::expression::Arena<Node>,
            node::identifier::Arena<Path>,
            Idx,
        ),
        Span,
    > {
        let mut identifiers = identifier::ParserArena::default();
        let (expressions, errors) = parse(input, &mut identifiers);
        assert_well_formed(&expressions, input);
        if let Some(error) = errors.first() {
            return Err(*error);
        }
        let root = expressions.nodes.last_idx();
        Ok((expressions.nodes, identifiers.finish().0, root))
    }

    fn parses(input: &str) -> bool {
        parse_alone(input).is_ok()
    }

    fn root_variant(input: &str) -> Option<(Variant, node::identifier::Arena<Path>)> {
        let (expressions, identifiers, root) = parse_alone(input).ok()?;
        Some((expressions[root].variant.clone(), identifiers))
    }

    #[test]
    fn accepts_whole_expression() {
        assert!(parses("a and b"));
        assert!(parses("  a <> 1  "));
        assert!(parses("sum s for s in [[ 1, 3 ]] end"));
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(!parses(""));
        assert!(!parses("a b"));
        assert!(!parses("a and"));
        assert!(!parses("a ) b"));
        assert!(!parses("sum s for s in [[ 1, 3 ]] end end"));
        assert!(!parses("a +"));
    }

    #[test]
    fn keyword_prefix_is_one_identifier() {
        for word in [
            "notable",
            "ifsfklsdf",
            "trueValue",
            "falsehood",
            "infinityLoop",
            "undefinedX",
            "eventuallyDone",
            "alwaysOn",
            "forall_",
            "ending",
            "thenx",
            "index",
            "summary",
            "counter",
        ] {
            match root_variant(word) {
                Some((Variant::Identifier(id), identifiers)) => {
                    assert_eq!(identifiers[id.target].as_str(), word)
                }
                other => panic!("{word}: expected an identifier, got {other:?}"),
            }
        }
    }

    #[test]
    fn keyword_must_not_continue_a_word() {
        assert!(!parses("a andb"));
        assert!(!parses("a inb"));
        assert!(!parses("a unionb"));
        assert!(!parses("when a thenb c end"));
    }

    #[test]
    fn punctuation_ends_a_keyword() {
        assert!(parses("not(a)"));
        assert!(parses("(a)and(b)"));
        assert!(parses("{ s for s in [[ 1, 3 ]] }"));
        assert!(parses("(sum 1, 2 end)"));
    }

    #[test]
    fn keywords_are_reserved() {
        assert!(!parses("{ s for end in { 1 } }"));
        assert!(!parses("all true for then in real end"));
        assert!(!parses("{ s for for in { 1 } }"));
        assert!(!parses("and"));
        assert!(!parses("a + in"));
    }

    #[test]
    fn aggregators_are_reserved() {
        assert!(!parses("max + sum"));
        assert!(!parses("all max > 0 for max in real end"));
        assert!(!parses("count"));
        assert!(parses("all x > 0 for x in real end"));
    }

    #[test]
    fn if_else_is_replaced_by_when() {
        assert!(!parses("if a then b end"));
        assert!(!parses("if a then b else c end"));
        assert!(parses("when a then b end"));
        assert!(parses("when a then b, otherwise c end"));
        // `if` and `else` are not keywords anymore
        assert!(parses("if and else"));
    }

    #[test]
    fn when_has_any_number_of_cases_and_an_optional_otherwise() {
        assert!(parses(
            "when a then b, c then d, e then f, g then h, otherwise i end"
        ));
        assert!(parses("when a then b, c then d, e then f end"));
        assert!(!parses("when a then b, end"));
        assert!(!parses("when otherwise a end"));
        assert!(!parses("when a then b, otherwise c, d then e end"));
    }

    #[test]
    fn calls_have_no_empty_argument() {
        assert!(parses("f()"));
        assert!(parses("f(a)"));
        assert!(parses("f(a, b)(c)"));
        assert!(!parses("f(,a)"));
        assert!(!parses("f(a,)"));
        assert!(!parses("f(a b)"));
    }

    #[test]
    fn double_brackets_are_ranges_and_single_ones_intervals() {
        assert!(matches!(
            root_variant("[[1, 2]]"),
            Some((Variant::Range(_), _))
        ));
        assert!(matches!(
            root_variant("]1, 2["),
            Some((Variant::Interval(_), _))
        ));
        assert!(!parses("[ [1, 2] ]"));
        assert!(!parses("[[1, 2]"));
        assert!(!parses("[1, 2]]"));
    }

    #[test]
    fn not_is_weaker_than_the_comparisons() {
        match root_variant("not a > b") {
            Some((Variant::UnaryOp(op), _)) => assert_eq!(op.operator, UnaryOperator::Not),
            other => panic!("expected a negation, got {other:?}"),
        }
        match root_variant("not a and b") {
            Some((Variant::BinOp(op), _)) => assert_eq!(op.operator, BinaryOperator::And),
            other => panic!("expected a conjunction, got {other:?}"),
        }
    }

    #[test]
    fn power_is_right_associative_and_negation_is_tighter() {
        match root_variant("a ^ b ^ c") {
            Some((Variant::BinOp(op), _)) => assert_eq!(op.operator, BinaryOperator::Power),
            other => panic!("expected a power, got {other:?}"),
        }
        // (-a) ^ 2
        match root_variant("- a ^ 2") {
            Some((Variant::BinOp(op), _)) => assert_eq!(op.operator, BinaryOperator::Power),
            other => panic!("expected a power, got {other:?}"),
        }
        // -(a!)
        match root_variant("- a !") {
            Some((Variant::UnaryOp(op), _)) => assert_eq!(op.operator, UnaryOperator::Negation),
            other => panic!("expected a negation, got {other:?}"),
        }
    }

    #[test]
    fn identifiers_can_be_unicode() {
        match root_variant("température::é") {
            Some((Variant::Identifier(id), identifiers)) => {
                assert_eq!(identifiers[id.target].as_str(), "température::é")
            }
            other => panic!("expected an identifier, got {other:?}"),
        }
    }

    #[test]
    fn the_error_is_the_token_the_parse_could_not_go_on_with() {
        let at = |input: &str| {
            let error = parse_alone(input).unwrap_err();
            (error.start.index, error.end.index)
        };
        assert_eq!(at("a b"), (2, 3));
        assert_eq!(at("a ) b"), (2, 3));
        assert_eq!(at("a + ) b"), (4, 5));
        // At the end of the text it is the end
        assert_eq!(at("a +"), (3, 3));
        assert_eq!(at(""), (0, 0));
        assert_eq!(at("sum a for a in S"), (16, 16));
    }
}
