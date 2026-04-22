use std::{cell::Ref, ops::Add};

use crate::{
    node::{
        NodeRef,
        expression::{Aggregator, BinaryOperator, Case, OptimizerKind, SetElement, UnaryOperator},
    },
    renderer::formatter::padding::pad,
    syntax::expression::Node,
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

pub fn format<W: std::io::Write>(node: Ref<Node>, ctx: Ctx, out: &mut W) -> std::io::Result<()> {
    use crate::node::expression::Variant::*;

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
            format_case(&branch.case, out)?;

            if let Some(otherwise) = &branch.otherwise {
                write!(out, "{}{}else ", sep, pd)?;
                format(otherwise.borrow(), Ctx::default(), out)?;
            }
        }

        When(when) => {
            write!(out, "when{}", sep)?;
            let pd1 = pad(ctx.indent + 1);
            for (idx, case) in when.cases.iter().enumerate() {
                write!(out, "{}", pd1)?;
                format_case(&case, out)?;
                if idx < when.cases.len() - 1 || when.otherwise.is_some() {
                    write!(out, ",{}", sep)?;
                }
            }

            if let Some(otherwise) = &when.otherwise {
                write!(out, "{}otherwise ", pd1)?;
                format(otherwise.borrow(), Ctx::default(), out)?;
            }
        }

        Forall(forall) => {
            write!(out, "forall ")?;
            format_set_elem(&forall.set, out)?;
            write!(out, "{}", sep)?;
            format(forall.expression.borrow(), ctx + 1, out)?;
        }

        Exists(exists) => {
            write!(out, "exists ")?;
            format_set_elem(&exists.set, out)?;
        }

        Select(select) => {
            write!(out, "select ")?;
            format_set_elem(&select.set, out)?;
            if let Some(ref optimizer) = select.optimizer {
                let symbol = optimizer_symbol(&optimizer.kind);
                write!(out, "{}{}{}{}", sep, pd, symbol, sep)?;
                format(optimizer.expression.borrow(), ctx + 1, out)?;
            }
        }

        Aggregation(aggregation) => {
            write!(out, "{}{}", aggregator_symbol(&aggregation.aggregator), sep)?;
            for (idx, val) in aggregation.expressions.iter().enumerate() {
                format(val.borrow(), ctx + 1, out)?;
                if idx < aggregation.expressions.len() - 1 {
                    write!(out, ",{}", sep)?;
                }
            }
        }

        UnaryOp(unary) => {
            let operand = unary.operand.borrow();
            let parens = operand.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(parens).inline();
            let symbol = unary_symbol(&unary.operator);
            match unary.is_postfix() {
                true => {
                    format(unary.operand.borrow(), sub, out)?;
                    write!(out, " {}", symbol)?;
                }
                false => {
                    write!(out, "{} ", symbol)?;
                    format(unary.operand.borrow(), sub, out)?;
                }
            }
        }

        BinOp(bin_op) => {
            let left = bin_op.left.borrow();
            let left_parens = left.variant.needs_parentheses(&node.variant, false);
            let sub = ctx.with_parens(left_parens).inline();
            format(bin_op.left.borrow(), sub, out)?;

            write!(out, " {} ", binary_symbol(&bin_op.operator))?;

            let right = bin_op.right.borrow();
            let right_parens = right.variant.needs_parentheses(&node.variant, true);
            let sub = ctx.with_parens(right_parens).inline();
            format(bin_op.right.borrow(), sub, out)?;
        }

        Function(function) => {
            format(function.expression.borrow(), ctx, out)?;
            write!(out, "( ")?;
            for (idx, arg) in function.arguments.iter().enumerate() {
                format(arg.borrow(), Ctx::default(), out)?;
                if idx < function.arguments.len() - 1 {
                    write!(out, ", ")?;
                }
            }
            write!(out, " )")?;
        }

        Set(set) => {
            write!(out, "{{ ")?;
            for (idx, elem) in set.elements.iter().enumerate() {
                format(elem.borrow(), Ctx::default(), out)?;
                if idx < set.elements.len() - 1 {
                    write!(out, ", ")?;
                }
            }
            write!(out, " }}")?;
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

fn format_case<W: std::io::Write>(case: &Case<NodeRef<Node>>, out: &mut W) -> std::io::Result<()> {
    format(case.condition.borrow(), Ctx::default(), out)?;
    write!(out, " then ")?;
    format(case.consequence.borrow(), Ctx::default(), out)
}

fn format_set_elem<W: std::io::Write>(
    set: &SetElement<NodeRef<Node>>,
    out: &mut W,
) -> std::io::Result<()> {
    write!(out, "{} in ", set.variable)?;
    format(set.domain.borrow(), Ctx::default(), out)?;

    if let Some(ref filter) = set.filter {
        write!(out, " such that ")?;
        format(filter.borrow(), Ctx::default(), out)?;
    }

    Ok(())
}

fn boolean_keyword(b: bool) -> &'static str {
    match b {
        true => "true",
        false => "false",
    }
}

fn optimizer_symbol(opt: &OptimizerKind) -> &'static str {
    use crate::node::expression::OptimizerKind::*;

    match opt {
        Maximize => "maximize",
        Minimize => "minimize",
    }
}

fn aggregator_symbol(agg: &Aggregator) -> &'static str {
    use crate::node::expression::Aggregator::*;

    match agg {
        Any => "any",
        All => "all",
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
        Equal => "=",
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
