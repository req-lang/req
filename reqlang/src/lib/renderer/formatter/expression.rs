use std::ops::Add;

use crate::{
    node::{
        expression::{Aggregator, BinaryOperator, Case, Idx, UnaryOperator},
        tree::Tree,
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

pub fn format<W: std::io::Write>(
    tree: &Tree,
    idx: Idx,
    ctx: Ctx,
    out: &mut W,
) -> std::io::Result<()> {
    use crate::node::expression::Variant::*;

    let node = &tree.expressions[idx];
    let pd = pad(ctx.indent);
    let parens = ctx.parens;
    let sep = if ctx.indent > 0 { "\n" } else { " " };
    let ctx = ctx.with_parens(false);

    write!(out, "{}", pd)?;
    if parens {
        write!(out, "(")?;
    }

    match &node.variant {
        When(when) => {
            write!(out, "when{}", sep)?;
            let pd1 = pad(ctx.indent + 1);
            for (idx, case) in when.cases.iter().enumerate() {
                write!(out, "{}", pd1)?;
                format_case(tree, case, out)?;
                if idx < when.cases.len() - 1 || when.otherwise.is_some() {
                    write!(out, ",{}", sep)?;
                }
            }

            if let Some(otherwise) = &when.otherwise {
                write!(out, "{}otherwise ", pd1)?;
                format(tree, *otherwise, Ctx::default(), out)?;
            }
        }

        Aggregation(aggregation) => {
            write!(out, "{}{}", aggregator_symbol(&aggregation.aggregator), sep)?;
            let n = aggregation.expressions.len();
            for (idx, val) in aggregation.expressions.iter().enumerate() {
                format(tree, *val, ctx + 1, out)?;
                if idx < n - 1 {
                    write!(out, ",{}", sep)?;
                }
            }
        }

        Comprehension(comprehension) => {
            // `{ e for x in S }` is inline, `sum e for x in S end` follows the aggregation layout
            match &comprehension.aggregator {
                None => {
                    write!(out, "{{ ")?;
                    format(tree, comprehension.expression, Ctx::default(), out)?;
                    write!(out, " for ")?;
                }
                Some(aggregator) => {
                    write!(out, "{}{}", aggregator_symbol(aggregator), sep)?;
                    format(tree, comprehension.expression, ctx + 1, out)?;
                    write!(out, "{}{}for ", sep, pad((ctx + 1).indent))?;
                }
            }

            let set = &comprehension.set;
            write!(out, "{} in ", set.variable)?;
            format(tree, set.domain, Ctx::default(), out)?;
            if let Some(filter) = &set.filter {
                write!(out, " such that ")?;
                format(tree, *filter, Ctx::default(), out)?;
            }

            if comprehension.aggregator.is_none() {
                write!(out, " }}")?;
            }
        }

        UnaryOp(unary) => {
            let operand = &tree.expressions[unary.operand];
            let parens = operand.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(parens).inline();
            let symbol = unary_symbol(&unary.operator);
            match unary.is_postfix() {
                true => {
                    format(tree, unary.operand, sub, out)?;
                    write!(out, " {}", symbol)?;
                }
                false => {
                    write!(out, "{} ", symbol)?;
                    format(tree, unary.operand, sub, out)?;
                }
            }
        }

        BinOp(bin_op) => {
            let left = &tree.expressions[bin_op.left];
            let left_parens = left.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(left_parens).inline();
            format(tree, bin_op.left, sub, out)?;

            write!(out, " {} ", binary_symbol(&bin_op.operator))?;

            let right = &tree.expressions[bin_op.right];
            let right_parens = right.variant.needs_parentheses(&node.variant, true);
            let sub = ctx.with_parens(right_parens).inline();
            format(tree, bin_op.right, sub, out)?;
        }

        Function(function) => {
            format(tree, function.expression, ctx, out)?;
            write!(out, "( ")?;
            for (idx, arg) in function.arguments.iter().enumerate() {
                format(tree, *arg, Ctx::default(), out)?;
                if idx < function.arguments.len() - 1 {
                    write!(out, ", ")?;
                }
            }
            write!(out, " )")?;
        }

        Set(set) => {
            write!(out, "{{ ")?;
            for (idx, elem) in set.elements.iter().enumerate() {
                format(tree, *elem, Ctx::default(), out)?;
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
            format(tree, interval.left.expression, sub, out)?;

            write!(out, ", ")?;

            let sub = ctx.with_parens(false).inline();
            format(tree, interval.right.expression, sub, out)?;
            let rsymbol = match interval.right.is_open {
                true => "[",
                false => "]",
            };
            write!(out, " {}", rsymbol)?;
        }

        Range(range) => {
            write!(out, "[[ ")?;

            let sub = ctx.with_parens(false).inline();
            format(tree, range.left, sub, out)?;

            write!(out, ", ")?;

            let sub = ctx.with_parens(false).inline();
            format(tree, range.right, sub, out)?;

            write!(out, " ]]")?;
        }

        Identifier(r) => write!(out, "{}", tree.identifiers[r.target])?,

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

fn format_case<W: std::io::Write>(tree: &Tree, case: &Case, out: &mut W) -> std::io::Result<()> {
    format(tree, case.condition, Ctx::default(), out)?;
    write!(out, " then ")?;
    format(tree, case.consequence, Ctx::default(), out)
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
