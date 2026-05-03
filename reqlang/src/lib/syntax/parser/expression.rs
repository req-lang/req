use std::{cell::RefCell, rc::Rc, sync::OnceLock};

use itertools::Itertools;
use pest::{
    Parser,
    iterators::Pair,
    pratt_parser::{Assoc, Op, PrattParser},
};

use crate::{
    node::{self, expression::*},
    syntax::location::{Position, Span},
    utils::children::ChildrenIter,
};

use pest_derive;
#[derive(pest_derive::Parser)]
#[grammar = "grammars/expression.pest"]
struct PairParser;

pub(in crate::syntax) type Node = node::expression::Node<Span, String>;

pub fn parse(input: &str, offset: &Position) -> Result<Arena<Node>, Span> {
    let mut arena = Vec::new();
    match PairParser::parse(Rule::expr, input) {
        Ok(mut pair) => {
            Builder::new(offset).build(pair.next().unwrap(), &mut arena);
            let len = arena.len();
            let mut arena: Vec<_> = arena
                .into_iter()
                .map(|node| {
                    let variant = node.variant.mape(|idx| (len - 1 - usize::from(idx)).into());
                    Node::new(node.payload, variant)
                })
                .collect();

            arena.reverse();
            Ok(arena.into())
        }
        Err(err) => Err(Span::from((err.line_col, err.location)) + *offset),
    }
}

struct Builder<'s> {
    pub offset: &'s Position,
}

impl<'s> Builder<'s> {
    fn new(offset: &'s Position) -> Self {
        Self { offset }
    }

    fn build(&self, pair: Pair<Rule>, arena: &mut Vec<Node>) -> Idx {
        let ar = Rc::new(RefCell::new(arena));
        let node = pratt_parser()
            .map_primary(|primary| {
                let mut arena = ar.borrow_mut();
                let span = Span::from(primary.as_span()) + *self.offset;
                match primary.as_rule() {
                    Rule::ident => {
                        let value = primary.as_str();
                        let reference = value.to_string();
                        let ident = Identifier::new(reference);
                        let variant = Variant::Identifier(ident);
                        Node::new(span, variant)
                    }

                    Rule::num => {
                        let value = primary.as_str().to_string();
                        let kind = primary.into_inner().next().unwrap().as_rule().into();
                        let mut number = Number::new(kind);
                        number.value = value;
                        let variant = Variant::Number(number);
                        Node::new(span, variant)
                    }

                    Rule::bool => {
                        let value = primary.into_inner().next().unwrap().as_rule().into();
                        let variant = Variant::Boolean(value);
                        Node::new(span, variant)
                    }

                    Rule::undefined => {
                        let variant = Variant::Undefined;
                        Node::new(span, variant)
                    }

                    Rule::branch => {
                        let mut inner = primary.into_inner();
                        let condition = self.build(inner.next().unwrap(), *arena);
                        let consequence = self.build(inner.next().unwrap(), *arena);
                        let case = Case::new(condition, consequence);
                        let mut branch = Branch::new(case);
                        if let Some(p) = inner.next() {
                            branch.otherwise = Some(self.build(p, *arena));
                        }
                        let variant = Variant::Branch(branch);
                        Node::new(span, variant)
                    }

                    Rule::when => {
                        let inner = primary.into_inner();
                        let mut when = When::new(Vec::new());
                        for p in inner {
                            match p.as_rule() {
                                Rule::case => {
                                    let mut inner = p.into_inner();
                                    let condition = self.build(inner.next().unwrap(), *arena);
                                    let consequence = self.build(inner.next().unwrap(), *arena);
                                    when.cases.push(Case::new(condition, consequence));
                                }

                                Rule::otherwise => {
                                    let node = self.build(p.into_inner().next().unwrap(), *arena);
                                    when.otherwise = Some(node);
                                }

                                _ => unreachable!(),
                            }
                        }

                        let variant = Variant::When(when);
                        Node::new(span, variant)
                    }

                    Rule::forall => {
                        let mut inner = primary.into_inner();
                        let expression = self.build(inner.next_back().unwrap(), *arena);
                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self.build(inner.next().unwrap(), *arena);
                        let mut set = SetElement::new(variable, domain);
                        if let Some(p) = inner.next() {
                            set.filter = Some(self.build(p, *arena));
                        }

                        let variant = Variant::Forall(Forall::new(set, expression));
                        Node::new(span, variant)
                    }

                    Rule::exists => {
                        let mut inner = primary.into_inner();

                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self.build(inner.next().unwrap(), *arena);
                        let mut set = SetElement::new(variable, domain);
                        if let Some(p) = inner.next() {
                            set.filter = Some(self.build(p, *arena));
                        }

                        let variant = Variant::Exists(Exists::new(set));
                        Node::new(span, variant)
                    }

                    Rule::select => {
                        let mut inner = primary.into_inner();

                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self.build(inner.next().unwrap(), *arena);
                        let mut select = Select::new(SetElement::new(variable, domain));
                        for p in inner {
                            match p.as_rule() {
                                Rule::optimizer => {
                                    let mut inner = p.into_inner();
                                    let kind = inner.next().unwrap().as_rule().into();
                                    let expression = self.build(inner.next().unwrap(), *arena);
                                    select.optimizer = Some(Optimizer::new(kind, expression))
                                }

                                Rule::expr => {
                                    select.set.filter = Some(self.build(p, *arena));
                                }

                                _ => {}
                            }
                        }

                        let variant = Variant::Select(select);
                        Node::new(span, variant)
                    }

                    Rule::aggregation => {
                        let mut inner = primary.into_inner();
                        let aggregator = inner.next().unwrap().as_rule().into();
                        let mut aggregation = Aggregation::new(aggregator);

                        for p in inner {
                            aggregation.expressions.push(self.build(p, *arena));
                        }

                        let variant = Variant::Aggregation(aggregation);
                        Node::new(span, variant)
                    }

                    Rule::set => {
                        let mut set = Set::default();
                        let inner = primary.into_inner();

                        for p in inner {
                            set.elements.push(self.build(p, *arena));
                        }

                        let variant = Variant::Set(set);
                        Node::new(span, variant)
                    }

                    Rule::expr => {
                        self.build(primary, *arena);
                        arena.pop().unwrap()
                    }

                    _ => unreachable!(),
                }
            })
            .map_infix(|lhs, op, rhs| {
                let n = ar.borrow().len();
                ar.borrow_mut().extend([lhs, rhs]);
                let span = Span::from(op.as_span()) + *self.offset;
                let operator = op.as_rule().into();
                let variant = Variant::BinOp(BinOp::new(operator, n.into(), (n + 1).into()));

                Node::new(span, variant)
            })
            .map_prefix(|op, operand| {
                let n = ar.borrow().len();
                ar.borrow_mut().push(operand);
                let span = Span::from(op.as_span()) + *self.offset;
                let operator = op.as_rule().into();
                let variant = Variant::UnaryOp(UnaryOp::new(operator, n.into()));

                Node::new(span, variant)
            })
            .map_postfix(|operand, op| {
                let mut ar = ar.borrow_mut();
                let n = ar.len();
                ar.push(operand);
                let span = Span::from(op.as_span()) + *self.offset;
                let variant = match op.as_rule() {
                    Rule::call => {
                        let mut function = Function::new(n.into());
                        for p in op.into_inner() {
                            function.arguments.push(self.build(p, *ar));
                        }
                        Variant::Function(function)
                    }
                    Rule::fac => Variant::UnaryOp(UnaryOp::new(UnaryOperator::Factorial, n.into())),
                    _ => unreachable!(),
                };

                Node::new(span, variant)
            })
            .parse(pair.into_inner());

        let n = ar.borrow().len();
        ar.borrow_mut().push(node);
        n.into()
    }
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
            .op(Op::infix(eq, Left) | Op::infix(neq, Left))
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
            Rule::ins => BinaryOperator::In,
            Rule::implies => BinaryOperator::Implies,
            Rule::iff => BinaryOperator::Iff,

            Rule::gt => BinaryOperator::GreaterThan,
            Rule::lt => BinaryOperator::LessThan,
            Rule::ge => BinaryOperator::GreaterOrEqual,
            Rule::le => BinaryOperator::LessOrEqual,
            Rule::neq => BinaryOperator::NotEqual,
            Rule::eq => BinaryOperator::Equal,

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

impl From<Rule> for OptimizerKind {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::mini => OptimizerKind::Minimize,
            Rule::maxi => OptimizerKind::Maximize,
            _ => unreachable!(),
        }
    }
}

impl From<Rule> for Aggregator {
    fn from(rule: Rule) -> Self {
        match rule {
            Rule::all => Aggregator::All,
            Rule::any => Aggregator::Any,
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
