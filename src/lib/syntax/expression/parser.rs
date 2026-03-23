use std::{cell::RefCell, ops::ControlFlow, sync::OnceLock};

use pest::{
    Parser,
    iterators::Pair,
    pratt_parser::{Assoc, Op, PrattParser},
};

use crate::{
    syntax::{Context, NodeParser, error::Error},
    visitor::Walk,
};

use super::*;

impl NodeParser<'_> {
    pub fn parse_expr(&mut self, input: &str) -> ControlFlow<Error, Expression> {
        match PairParser::parse(Rule::expr, input) {
            Ok(mut pairs) => self.walk(pairs.next().unwrap()).map_break(|mut err| {
                err.span = err.span + self.offset;
                err
            }),
            Err(err) => ControlFlow::Break(err.into()),
        }
    }
}

use pest_derive;
#[derive(pest_derive::Parser)]
#[grammar = "grammars/expression.pest"]
pub struct PairParser;

impl Walk<Pair<'_, Rule>, Error, Expression> for NodeParser<'_> {
    fn walk(&mut self, pair: Pair<Rule>) -> ControlFlow<Error, Expression> {
        let offset = self.offset;
        let rc = RefCell::new(&mut *self);
        let node = pratt_parser()
            .map_primary(|primary| {
                let context = Context::parse(&primary, offset);
                let mut self_mut = rc.borrow_mut();
                self_mut.spans.push(context.span);
                match primary.as_rule() {
                    Rule::ident => {
                        let value = primary.as_str().to_string();
                        let reference = entity::Reference::new(value);
                        let ident = Identifier::new(reference);
                        let variant = ExpressionVariant::Identifier(ident);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::num => {
                        let value = primary.as_str().to_string();
                        let kind = primary.into_inner().next().unwrap().as_rule().into();
                        let mut number = Number::new(kind);
                        number.value = value;
                        let variant = ExpressionVariant::Number(number);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::bool => {
                        let value = primary.into_inner().next().unwrap().as_rule().into();
                        let variant = ExpressionVariant::Boolean(value);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::undefined => {
                        let variant = ExpressionVariant::Undefined;
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::branch => {
                        let mut inner = primary.into_inner();
                        let condition = self_mut.walk(inner.next().unwrap())?;
                        let consequence = self_mut.walk(inner.next().unwrap())?;
                        let case = Case::new(condition, consequence);
                        let mut branch = Branch::new(case);
                        if let Some(p) = inner.next() {
                            branch.otherwise = Some(Box::new(self_mut.walk(p)?));
                        }
                        let variant = ExpressionVariant::Branch(branch);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::when => {
                        let inner = primary.into_inner();
                        let mut when = When::new(Vec::new());
                        for p in inner {
                            match p.as_rule() {
                                Rule::case => {
                                    let mut inner = p.into_inner();
                                    let condition = self_mut.walk(inner.next().unwrap())?;
                                    let consequence = self_mut.walk(inner.next().unwrap())?;
                                    when.cases.push(Case::new(condition, consequence));
                                }

                                Rule::otherwise => {
                                    let node = self_mut.walk(p.into_inner().next().unwrap())?;
                                    when.otherwise = Some(Box::new(node));
                                }

                                _ => unreachable!(),
                            }
                        }

                        let variant = ExpressionVariant::When(when);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::forall => {
                        let mut inner = primary.into_inner();
                        let expression = self_mut.walk(inner.next_back().unwrap())?;
                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self_mut.walk(inner.next().unwrap())?;
                        let mut set = SetElement::new(variable, domain);
                        if let Some(p) = inner.next() {
                            set.filter = Some(Box::new(self_mut.walk(p)?));
                        }

                        let variant = ExpressionVariant::Forall(Forall::new(set, expression));
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::exists => {
                        let mut inner = primary.into_inner();

                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self_mut.walk(inner.next().unwrap())?;
                        let mut set = SetElement::new(variable, domain);
                        if let Some(p) = inner.next() {
                            set.filter = Some(Box::new(self_mut.walk(p)?));
                        }

                        let variant = ExpressionVariant::Exists(Exists::new(set));
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::select => {
                        let mut inner = primary.into_inner();

                        let variable = inner.next().unwrap().as_str().to_string();
                        let domain = self_mut.walk(inner.next().unwrap())?;
                        let mut select = Select::new(SetElement::new(variable, domain));
                        for p in inner {
                            match p.as_rule() {
                                Rule::optimizer => {
                                    let mut inner = p.into_inner();
                                    let kind = inner.next().unwrap().as_rule().into();
                                    let expression = self_mut.walk(inner.next().unwrap())?;
                                    select.optimizer = Some(Optimizer::new(kind, expression))
                                }

                                Rule::expr => {
                                    select.set.filter = Some(Box::new(self_mut.walk(p)?));
                                }

                                _ => {}
                            }
                        }

                        let variant = ExpressionVariant::Select(select);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::aggregation => {
                        let mut inner = primary.into_inner();
                        let aggregator = inner.next().unwrap().as_rule().into();
                        let mut aggregation = Aggregation::new(aggregator);

                        for p in inner {
                            aggregation.expressions.push(self_mut.walk(p)?);
                        }

                        let variant = ExpressionVariant::Aggregation(aggregation);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::set => {
                        let mut set = Set::default();
                        let inner = primary.into_inner();

                        for p in inner {
                            set.elements.push(self_mut.walk(p)?);
                        }

                        let variant = ExpressionVariant::Set(set);
                        ControlFlow::Continue(Expression::new(variant))
                    }

                    Rule::expr => self_mut.walk(primary),

                    _ => unreachable!(),
                }
            })
            .map_infix(|lhs, op, rhs| {
                let lhs = lhs?;
                let rhs = rhs?;

                rc.borrow_mut().spans.push(op.as_span().into());
                let operator = op.as_rule().into();
                let variant = ExpressionVariant::BinOp(BinOp::new(operator, lhs, rhs));

                ControlFlow::Continue(Expression::new(variant))
            })
            .map_prefix(|op, operand| {
                let operand = operand?;
                rc.borrow_mut().spans.push(op.as_span().into());
                let operator = op.as_rule().into();
                let variant = ExpressionVariant::UnaryOp(UnaryOp::new(operator, operand));

                ControlFlow::Continue(Expression::new(variant))
            })
            .map_postfix(|operand, op| {
                let operand = operand?;
                let mut self_mut = rc.borrow_mut();

                self_mut.spans.push(op.as_span().into());
                let variant = match op.as_rule() {
                    Rule::call => {
                        let mut function = Function::new(operand);
                        for p in op.into_inner() {
                            function.arguments.push(self_mut.walk(p)?);
                        }
                        ExpressionVariant::Function(function)
                    }
                    Rule::fac => {
                        ExpressionVariant::UnaryOp(UnaryOp::new(UnaryOperator::Factorial, operand))
                    }
                    _ => unreachable!(),
                };

                ControlFlow::Continue(Expression::new(variant))
            })
            .parse(pair.into_inner());
        node
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
