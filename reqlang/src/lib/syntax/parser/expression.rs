use std::{cell::RefCell, sync::OnceLock};

use pest::{
    Parser,
    iterators::Pair,
    pratt_parser::{Assoc, Op, PrattParser},
};

use crate::{
    node::{
        expression::{self, *},
        identifier,
        path::Path,
    },
    syntax::location::{Position, Span},
};

use pest_derive;
#[derive(pest_derive::Parser)]
#[grammar = "grammars/expression.pest"]
struct PairParser;

#[derive(Debug)]
pub(super) struct Draft<'a> {
    expressions: &'a mut Vec<expression::Node>,
    identifiers: &'a mut Vec<Path>,
    spans: &'a mut Vec<Span>,
}

impl<'a> Draft<'a> {
    pub fn new(
        expressions: &'a mut Vec<expression::Node>,
        identifiers: &'a mut Vec<Path>,
        spans: &'a mut Vec<Span>,
    ) -> Self {
        Self {
            expressions,
            identifiers,
            spans,
        }
    }

    pub fn push_node(&mut self, node: Node, span: Span) -> Idx {
        let idx = self.expressions.len();
        self.expressions.push(node);
        self.spans.push(span);
        idx.into()
    }

    pub fn push_identifier(&mut self, path: Path) -> identifier::Idx {
        let idx = self.identifiers.len();
        self.identifiers.push(path);
        idx.into()
    }
}

pub(super) fn parse(input: &str, offset: &Position, mut draft: Draft) -> Result<Idx, Span> {
    match PairParser::parse(Rule::root, input) {
        Ok(mut pair) => {
            let draft = RefCell::new(&mut draft);
            parse_pair(offset, pair.next().unwrap(), &draft);
            let root = draft.borrow().expressions.len() - 1;
            Ok(root.into())
        }
        Err(err) => Err(Span::from((err.line_col, err.location)) + *offset),
    }
}

fn parse_pair(ctx: &Position, pair: Pair<Rule>, draft: &RefCell<&mut Draft>) -> Idx {
    pratt_parser()
        .map_primary(|primary| {
            let span = Span::from(primary.as_span()) + *ctx;
            match primary.as_rule() {
                Rule::ident => {
                    let path = Path::from(primary.as_str());
                    let target = draft.borrow_mut().push_identifier(path);
                    let ident = Identifier::new(target);
                    let variant = Variant::Identifier(ident);
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::num => {
                    let value = primary.as_str().to_string();
                    let kind = primary.into_inner().next().unwrap().as_rule().into();
                    let mut number = Number::new(kind);
                    number.value = value;
                    let variant = Variant::Number(number);
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::bool => {
                    let value = primary.into_inner().next().unwrap().as_rule().into();
                    let variant = Variant::Boolean(value);
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::undefined => {
                    let variant = Variant::Undefined;
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::when => {
                    let inner = operands(primary);
                    let mut when = When::new(Vec::new());
                    for p in inner {
                        match p.as_rule() {
                            Rule::case => {
                                let mut inner = operands(p);
                                let condition = parse_pair(ctx, inner.next().unwrap(), draft);
                                let consequence = parse_pair(ctx, inner.next().unwrap(), draft);
                                when.cases.push(Case::new(condition, consequence));
                            }

                            Rule::otherwise => {
                                let node = parse_pair(ctx, operands(p).next().unwrap(), draft);
                                when.otherwise = Some(node);
                            }

                            _ => unreachable!(),
                        }
                    }

                    let variant = Variant::When(when);
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::set | Rule::aggregation => {
                    let is_aggregation = primary.as_rule() == Rule::aggregation;
                    let mut inner = operands(primary);
                    let aggregator: Option<Aggregator> =
                        is_aggregation.then(|| inner.next().unwrap().as_rule().into());
                    let first = parse_pair(ctx, inner.next().unwrap(), draft);

                    let variant = match inner.next() {
                        Some(p) if p.as_rule() == Rule::binding => {
                            let mut inner = operands(p);
                            let variable = inner.next().unwrap().as_str().to_string();
                            let domain = parse_pair(ctx, inner.next().unwrap(), draft);
                            let mut set = SetElement::new(variable, domain);
                            if let Some(p) = inner.next() {
                                set.filter = Some(parse_pair(ctx, p, draft));
                            }

                            Variant::Comprehension(Comprehension::new(aggregator, set, first))
                        }

                        next => {
                            let mut expressions = vec![first];
                            for p in next.into_iter().chain(inner) {
                                expressions.push(parse_pair(ctx, p, draft));
                            }

                            match aggregator {
                                Some(aggregator) => Variant::Aggregation(Aggregation {
                                    aggregator,
                                    expressions,
                                }),
                                None => Variant::Set(Set {
                                    elements: expressions,
                                }),
                            }
                        }
                    };

                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::rng => {
                    let mut inner = primary.into_inner();

                    let lvalue = parse_pair(ctx, inner.next().unwrap(), draft);
                    let rvalue = parse_pair(ctx, inner.next().unwrap(), draft);
                    let interval = Range::new(lvalue, rvalue);
                    let variant = Variant::Range(interval);

                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::intv => {
                    let mut inner = primary.into_inner();
                    let lbrack = inner.next().unwrap();
                    let lvalue = parse_pair(ctx, inner.next().unwrap(), draft);
                    let lbound = match lbrack.as_rule() {
                        Rule::lbrkt => IntervalBound::closed(lvalue),
                        Rule::rbrkt => IntervalBound::open(lvalue),
                        _ => unreachable!(),
                    };

                    let rvalue = parse_pair(ctx, inner.next().unwrap(), draft);
                    let rbrack = inner.next().unwrap();
                    let rbound = match rbrack.as_rule() {
                        Rule::lbrkt => IntervalBound::open(rvalue),
                        Rule::rbrkt => IntervalBound::closed(rvalue),
                        _ => unreachable!(),
                    };

                    let interval = Interval::new(lbound, rbound);
                    let variant = Variant::Interval(interval);
                    draft.borrow_mut().push_node(Node::new(variant), span)
                }

                Rule::expr => parse_pair(ctx, primary, draft),

                _ => unreachable!(),
            }
        })
        .map_infix(|lhs, op, rhs| {
            let span = Span::from(op.as_span()) + *ctx;
            let operator = op.as_rule().into();
            let variant = Variant::BinOp(BinOp::new(operator, lhs, rhs));

            draft.borrow_mut().push_node(Node::new(variant), span)
        })
        .map_prefix(|op, operand| {
            let span = Span::from(op.as_span()) + *ctx;
            let operator = op.as_rule().into();
            let variant = Variant::UnaryOp(UnaryOp::new(operator, operand));

            draft.borrow_mut().push_node(Node::new(variant), span)
        })
        .map_postfix(|operand, op| {
            let span = Span::from(op.as_span()) + *ctx;
            let variant = match op.as_rule() {
                Rule::call => {
                    let mut function = Function::new(operand);
                    for p in op.into_inner() {
                        function.arguments.push(parse_pair(ctx, p, draft));
                    }
                    Variant::Function(function)
                }
                Rule::fac => Variant::UnaryOp(UnaryOp::new(UnaryOperator::Factorial, operand)),
                _ => unreachable!(),
            };

            draft.borrow_mut().push_node(Node::new(variant), span)
        })
        .parse(pair.into_inner())
}

/// Inner pairs of a rule without its keyword tokens (`if`, `then`, `end`, `for`...)
fn operands(pair: Pair<Rule>) -> impl DoubleEndedIterator<Item = Pair<Rule>> {
    pair.into_inner().filter(|p| !is_keyword(p.as_rule()))
}

/// Keywords that only delimit a construct. Operators, aggregators and literals are
/// keywords too, but they carry meaning and are read by the parser.
fn is_keyword(rule: Rule) -> bool {
    matches!(
        rule,
        Rule::whe
            | Rule::then
            | Rule::end
            | Rule::such
            | Rule::that
            | Rule::r#in
            | Rule::r#for
            | Rule::other
    )
}

fn pratt_parser() -> &'static PrattParser<Rule> {
    use Assoc::*;
    use Rule::*;

    static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();
    PRATT_PARSER.get_or_init(|| {
        PrattParser::new()
            .op(Op::infix(iff, Left) | Op::infix(implies, Left))
            .op(Op::infix(or, Left) | Op::infix(xor, Left))
            .op(Op::infix(and, Left))
            .op(Op::prefix(not))
            .op(Op::prefix(previous)
                | Op::prefix(rise)
                | Op::prefix(fall)
                | Op::prefix(event)
                | Op::prefix(always))
            .op(Op::infix(since, Left))
            .op(Op::infix(uni, Left) | Op::infix(diff, Left))
            .op(Op::infix(inter, Left))
            .op(Op::infix(comp, Left))
            .op(Op::infix(ins, Left) | Op::infix(incl, Left))
            .op(Op::infix(tol, Left))
            .op(Op::infix(ini, Left)
                | Op::infix(fix, Left)
                | Op::infix(eq, Left)
                | Op::infix(neq, Left))
            .op(Op::infix(lt, Left)
                | Op::infix(gt, Left)
                | Op::infix(le, Left)
                | Op::infix(ge, Left))
            .op(Op::infix(add, Left) | Op::infix(sub, Left))
            .op(Op::infix(mul, Left) | Op::infix(div, Left) | Op::infix(rem, Left))
            .op(Op::infix(pow, Right))
            .op(Op::prefix(neg) | Op::prefix(plus) | Op::prefix(not))
            .op(Op::postfix(fac))
            .op(Op::postfix(call))
    })
}

impl From<Rule> for UnaryOperator {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::plus => UnaryOperator::Plus,
            Rule::neg => UnaryOperator::Negation,
            Rule::not => UnaryOperator::Not,
            Rule::fac => UnaryOperator::Factorial,
            Rule::previous => UnaryOperator::Previously,
            Rule::rise => UnaryOperator::Rise,
            Rule::fall => UnaryOperator::Fall,
            Rule::event => UnaryOperator::Eventually,
            Rule::always => UnaryOperator::Always,
            _ => unreachable!(),
        }
    }
}

impl From<Rule> for BinaryOperator {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::add => BinaryOperator::Plus,
            Rule::sub => BinaryOperator::Minus,
            Rule::mul => BinaryOperator::Multiply,
            Rule::div => BinaryOperator::Divide,
            Rule::rem => BinaryOperator::Modulus,
            Rule::pow => BinaryOperator::Power,

            Rule::and => BinaryOperator::And,
            Rule::or => BinaryOperator::Or,
            Rule::xor => BinaryOperator::Xor,
            Rule::implies => BinaryOperator::Implies,
            Rule::iff => BinaryOperator::Iff,

            Rule::ins => BinaryOperator::In,
            Rule::tol => BinaryOperator::Tolerance,

            Rule::gt => BinaryOperator::GreaterThan,
            Rule::lt => BinaryOperator::LessThan,
            Rule::ge => BinaryOperator::GreaterOrEqual,
            Rule::le => BinaryOperator::LessOrEqual,
            Rule::neq => BinaryOperator::NotEqual,
            Rule::eq => BinaryOperator::Equal,
            Rule::fix => BinaryOperator::Fix,
            Rule::ini => BinaryOperator::Initially,

            Rule::uni => BinaryOperator::Union,
            Rule::inter => BinaryOperator::Intersection,
            Rule::diff => BinaryOperator::Difference,
            Rule::comp => BinaryOperator::Complement,
            Rule::incl => BinaryOperator::Includes,

            Rule::since => BinaryOperator::Since,
            _ => unreachable!(),
        }
    }
}

impl From<Rule> for Aggregator {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::all => Aggregator::All,
            Rule::any => Aggregator::Any,
            Rule::one => Aggregator::One,
            Rule::max => Aggregator::Maximum,
            Rule::min => Aggregator::Minimum,
            Rule::sum => Aggregator::Sum,
            Rule::prod => Aggregator::Product,
            Rule::choose => Aggregator::Choose,
            Rule::argmax => Aggregator::ArgMaximum,
            Rule::argmin => Aggregator::ArgMinimum,
            Rule::sup => Aggregator::Supremum,
            Rule::inf => Aggregator::Infimum,
            Rule::count => Aggregator::Count,
            Rule::inter => Aggregator::Intersection,
            Rule::uni => Aggregator::Union,
            _ => unreachable!(),
        }
    }
}

impl From<Rule> for NumberKind {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::int => NumberKind::Integer,
            Rule::real => NumberKind::Real,
            Rule::infinity => NumberKind::Infinity,
            _ => unreachable!(),
        }
    }
}

impl From<Rule> for Boolean {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::tru => Self { value: true },
            Rule::fals => Self { value: false },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_alone(input: &str) -> Result<(Vec<Node>, Vec<Path>, Idx), Span> {
        let mut expressions = Vec::new();
        let mut identifiers = Vec::new();
        let mut spans = Vec::new();
        let draft = Draft::new(&mut expressions, &mut identifiers, &mut spans);
        let root = parse(input, &Position::default(), draft)?;
        Ok((expressions, identifiers, root))
    }

    fn parses(input: &str) -> bool {
        parse_alone(input).is_ok()
    }

    #[test]
    fn accepts_whole_expression() {
        assert!(parses("a and b"));
        assert!(parses("  a <> 1  "));
        assert!(parses("sum s for s in [[ 1, 3 ]] end"));
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(!parses("a b"));
        assert!(!parses("a and"));
        assert!(!parses("a ) b"));
        assert!(!parses("sum s for s in [[ 1, 3 ]] end end"));
        assert!(!parses("a +"));
    }

    fn root_variant(input: &str) -> Option<(Variant, Vec<Path>)> {
        let (expressions, identifiers, root) = parse_alone(input).ok()?;
        Some((expressions[usize::from(root)].variant.clone(), identifiers))
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
        ] {
            match root_variant(word) {
                Some((Variant::Identifier(id), identifiers)) => {
                    assert_eq!(identifiers[usize::from(id.target)].as_str(), word)
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
    fn aggregator_names_are_not_reserved() {
        assert!(parses("max + sum"));
        assert!(parses("all max > 0 for max in real end"));
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
}
