use std::{fmt::Display, ops::Add};

use crate::{
    node::expression::{
        Aggregator, Arena, BinaryOperator, Case, Idx, Node, Operand, SetElement,
        UnaryOperator,
    },
    renderer::formatter::padding::pad,
};

#[derive(Default, Debug, Copy, Clone)]
pub struct Ctx {
    pub indent: usize,
    pub parens: bool,
}

impl From<usize> for Ctx {
    fn from(value: usize) -> Self {
        Self {
            indent: value,
            parens: false,
        }
    }
}

impl Add<usize> for Ctx {
    type Output = Ctx;

    fn add(self, rhs: usize) -> Self::Output {
        let indent = match self.indent == 0 {
            true => 0,
            false => self.indent + rhs,
        };
        Ctx {
            indent,
            parens: self.parens,
        }
    }
}

impl Ctx {
    pub fn with_parens(self, parens: bool) -> Self {
        Ctx {
            indent: self.indent,
            parens,
        }
    }

    pub fn inline(self) -> Self {
        Ctx {
            indent: 0,
            parens: self.parens,
        }
    }
}

pub fn format<W: std::io::Write, E, T: Display>(
    arena: &Arena<Node<E, T>>,
    idx: Idx,
    ctx: Ctx,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::expression::Variant::*;

    let node = &arena[idx];
    let pd = pad(ctx.indent);
    let parens = ctx.parens;
    let sep = if ctx.indent > 0 { "\n" } else { " " };
    let ctx = ctx.with_parens(false);

    write!(out, "{}", pd)?;
    if parens {
        write!(out, "(")?;
    }

    match &node.variant {
        Branch(branch) => {
            write!(out, "if ")?;
            format_case(arena, &branch.case, out)?;

            if let Some(otherwise) = &branch.otherwise {
                write!(out, "{}{}else ", sep, pd)?;
                format(arena, *otherwise, Ctx::default(), out)?;
            }
        }

        When(when) => {
            write!(out, "when{}", sep)?;
            let pd1 = pad(ctx.indent + 1);
            for (idx, case) in when.cases.iter().enumerate() {
                write!(out, "{}", pd1)?;
                format_case(arena, case, out)?;
                if idx < when.cases.len() - 1 || when.otherwise.is_some() {
                    write!(out, ",{}", sep)?;
                }
            }

            if let Some(otherwise) = &when.otherwise {
                write!(out, "{}otherwise ", pd1)?;
                format(arena, *otherwise, Ctx::default(), out)?;
            }
        }

        Forall(forall) => {
            write!(out, "forall ")?;
            format_set_elem(arena, &forall.set, out)?;
            write!(out, "{}", sep)?;
            format(arena, forall.expression, ctx + 1, out)?;
        }

        Exists(exists) => {
            write!(out, "exists ")?;
            format_set_elem(arena, &exists.set, out)?;
        }

        Aggregation(aggregation) => {
            write!(out, "{}{}", aggregator_symbol(&aggregation.aggregator), sep)?;
            match &aggregation.operand {
                Operand::List(expressions) => {
                    for (idx, val) in expressions.iter().enumerate() {
                        format(arena, *val, ctx + 1, out)?;
                        if idx < expressions.len() - 1 {
                            write!(out, ",{}", sep)?;
                        }
                    }
                }

                Operand::Comprehension(comprehension) => {
                    format(arena, comprehension.expression, ctx + 1, out)?;
                    write!(out, "{}{}for ", sep, pad((ctx + 1).indent))?;
                    format_set_elem(arena, &comprehension.set, out)?;
                }
            }
        }

        Comprehension(comprehension) => {
            write!(out, "{{ ")?;
            format(arena, comprehension.expression, Ctx::default(), out)?;
            write!(out, " for ")?;
            format_set_elem(arena, &comprehension.set, out)?;
            write!(out, " }}")?;
        }

        UnaryOp(unary) => {
            let operand = &arena[unary.operand];
            let parens = operand.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(parens).inline();
            let symbol = unary_symbol(&unary.operator);
            match unary.is_postfix() {
                true => {
                    format(arena, unary.operand, sub, out)?;
                    write!(out, " {}", symbol)?;
                }
                false => {
                    write!(out, "{} ", symbol)?;
                    format(arena, unary.operand, sub, out)?;
                }
            }
        }

        BinOp(bin_op) => {
            let left = &arena[bin_op.left];
            let left_parens = left.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(left_parens).inline();
            format(arena, bin_op.left, sub, out)?;

            write!(out, " {} ", binary_symbol(&bin_op.operator))?;

            let right = &arena[bin_op.right];
            let right_parens = right.variant.needs_parentheses(&node.variant, true);
            let sub = ctx.with_parens(right_parens).inline();
            format(arena, bin_op.right, sub, out)?;
        }

        Function(function) => {
            format(arena, function.expression, ctx, out)?;
            write!(out, "( ")?;
            for (idx, arg) in function.arguments.iter().enumerate() {
                format(arena, *arg, Ctx::default(), out)?;
                if idx < function.arguments.len() - 1 {
                    write!(out, ", ")?;
                }
            }
            write!(out, " )")?;
        }

        Set(set) => {
            write!(out, "{{ ")?;
            for (idx, elem) in set.elements.iter().enumerate() {
                format(arena, *elem, Ctx::default(), out)?;
                if idx < set.elements.len() - 1 {
                    write!(out, ", ")?;
                }
            }
            write!(out, " }}")?;
        }

        Interval(interval) => {
            let lsymbol = match interval.left.is_open {
                true => "]",
                false => "[",
            };
            write!(out, "{} ", lsymbol)?;
            let sub = ctx.with_parens(false).inline();
            format(arena, interval.left.expression, sub, out)?;

            write!(out, ", ")?;

            let sub = ctx.with_parens(false).inline();
            format(arena, interval.right.expression, sub, out)?;
            let rsymbol = match interval.right.is_open {
                true => "[",
                false => "]",
            };
            write!(out, " {}", rsymbol)?;
        }

        Range(range) => {
            write!(out, "[[ ")?;

            let sub = ctx.with_parens(false).inline();
            format(arena, range.left, sub, out)?;

            write!(out, ", ")?;

            let sub = ctx.with_parens(false).inline();
            format(arena, range.right, sub, out)?;

            write!(out, " ]]")?;
        }

        Identifier(r) => write!(out, "{}", r.target)?,

        Number(x) => write!(out, "{}", x.value)?,

        Boolean(b) => write!(out, "{}", boolean_keyword(b.value))?,

        Undefined => write!(out, "undefined")?,
    }

    if node.variant.needs_end() {
        write!(out, "{}{}end", sep, pd)?;
    }

    if parens {
        write!(out, ")")?;
    }

    Ok(())
}

fn format_case<W: std::io::Write, E, T: Display>(
    arena: &Arena<Node<E, T>>,
    case: &Case<Idx>,
    out: &mut W,
) -> std::io::Result<()> {
    format(arena, case.condition, Ctx::default(), out)?;
    write!(out, " then ")?;
    format(arena, case.consequence, Ctx::default(), out)
}

fn format_set_elem<W: std::io::Write, E, T: Display>(
    arena: &Arena<Node<E, T>>,
    set: &SetElement<Idx>,
    out: &mut W,
) -> std::io::Result<()> {
    write!(out, "{} in ", set.variable)?;
    format(arena, set.domain, Ctx::default(), out)?;

    if let Some(ref filter) = set.filter {
        write!(out, " such that ")?;
        format(arena, *filter, Ctx::default(), out)?;
    }

    Ok(())
}

fn boolean_keyword(b: bool) -> &'static str {
    match b {
        true => "true",
        false => "false",
    }
}

fn aggregator_symbol(agg: &Aggregator) -> &'static str {
    use crate::node::expression::Aggregator::*;

    match agg {
        Any => "any",
        All => "all",
        One => "one",
        Maximum => "max",
        Minimum => "min",
        Sum => "sum",
        Product => "prod",
        Intersection => "intersection",
        Union => "union",
        Choose => "choose",
        ArgMaximum => "argmax",
        ArgMinimum => "argmin",
        Supremum => "sup",
        Infimum => "inf",
        Count => "count",
    }
}

fn unary_symbol(unary: &UnaryOperator) -> &'static str {
    use crate::node::expression::UnaryOperator::*;

    match unary {
        Not => "not",
        Plus => "+",
        Negation => "-",
        Factorial => "!",
        Previously => "previously",
        Rise => "rising",
        Fall => "falling",
        Eventually => "eventually",
        Always => "always",
    }
}

fn binary_symbol(binary: &BinaryOperator) -> &'static str {
    use crate::node::expression::BinaryOperator::*;

    match binary {
        And => "and",
        Or => "or",
        Xor => "xor",
        Implies => "implies",
        Iff => "iff",
        Fix => ":=",
        Equal => "=",
        Initially => "°=",
        NotEqual => "<>",
        LessThan => "<",
        GreaterThan => ">",
        Plus => "+",
        Minus => "-",
        Multiply => "*",
        Divide => "/",
        Modulus => "%",
        Power => "^",
        In => "in",
        Tolerance => "tol",
        GreaterOrEqual => ">=",
        LessOrEqual => "<=",
        Union => "union",
        Intersection => "intersection",
        Since => "since",
        Difference => "difference",
        Complement => "complement",
        Includes => "includes",
    }
}
