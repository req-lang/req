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
        Ctx {
            indent: self.indent + rhs,
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
}

pub fn format<W: std::io::Write>(node: Ref<Node>, ctx: Ctx, out: &mut W) -> std::io::Result<()> {
    use crate::node::expression::Variant::*;

    let pad = pad(ctx.indent);
    let parens = ctx.parens;
    let ctx = ctx.with_parens(false);

    if parens {
        write!(out, "(")?;
    }

    match &node.variant {
        Branch(branch) => {
            write!(out, "{}if ", pad)?;
            format_case(&branch.case, ctx, out)?;

            if let Some(otherwise) = &branch.otherwise {
                writeln!(out, "{}else", pad)?;
                format(otherwise.borrow(), ctx + 1, out)?;
            }

            writeln!(out, "{}end", pad)?;
        }

        When(when) => {
            writeln!(out, "{}when", pad)?;
            for (idx, case) in when.cases.iter().enumerate() {
                if idx > 0 {
                    write!(out, ", ")?;
                }

                format_case(&case, ctx, out)?;
            }
            if let Some(otherwise) = &when.otherwise {
                write!(out, ",\n")?;
                writeln!(out, "{}otherwise", pad)?;
                format(otherwise.borrow(), ctx + 1, out)?;
            }
            writeln!(out, "{}end", pad)?;
        }

        Forall(forall) => {
            write!(out, "{}forall ", pad)?;
            format_set_elem(&forall.set, ctx, out)?;
            format(forall.expression.borrow(), ctx, out)?;
            writeln!(out, "{}end", pad)?;
        }

        Exists(exists) => {
            write!(out, "{}exists ", pad)?;
            format_set_elem(&exists.set, ctx, out)?;
            writeln!(out, "{}end", pad)?;
        }

        Select(select) => {
            write!(out, "{}select ", pad)?;
            format_set_elem(&select.set, ctx, out)?;

            if let Some(ref optimizer) = select.optimizer {
                write!(out, "\n")?;
                write!(out, "{} ", optimizer_symbol(&optimizer.kind))?;
                format(optimizer.expression.borrow(), ctx, out)?;
            }

            writeln!(out, "{}end", pad)?;
        }

        Aggregation(aggregation) => {
            writeln!(out, "{} ", aggregator_symbol(&aggregation.aggregator))?;

            for (idx, val) in aggregation.expressions.iter().enumerate() {
                if idx > 0 {
                    write!(out, ", ")?;
                }

                writeln!(out, "{}", pad)?;
                format(val.borrow(), ctx, out)?;
            }

            writeln!(out, "{}end", pad)?;
        }

        UnaryOp(unary) => {
            write!(out, "{}", pad)?;
            let operand = unary.operand.borrow();
            let parens = operand.variant.needs_parentheses(&node.variant, false);

            match unary.is_postfix() {
                true => {
                    format(unary.operand.borrow(), ctx.with_parens(parens), out)?;
                    write!(out, "{}", unary_symbol(&unary.operator))?;
                }
                false => {
                    write!(out, "{}", unary_symbol(&unary.operator))?;
                    format(unary.operand.borrow(), ctx.with_parens(parens), out)?;
                }
            }
        }

        BinOp(bin_op) => {
            write!(out, "{}", pad)?;

            let left = bin_op.left.borrow();
            let left_parens = left.variant.needs_parentheses(&node.variant, false);
            format(bin_op.left.borrow(), ctx.with_parens(left_parens), out)?;

            write!(out, " {} ", binary_symbol(&bin_op.operator))?;

            let right = bin_op.right.borrow();
            let right_parens = right.variant.needs_parentheses(&node.variant, true);
            format(bin_op.right.borrow(), ctx.with_parens(right_parens), out)?;
        }

        Function(function) => {
            format(function.expression.borrow(), ctx, out)?;
            write!(out, "(")?;
            for (i, arg) in function.arguments.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ")?;
                }
                format(arg.borrow(), ctx, out)?;
            }
            write!(out, ")")?;
        }

        Set(set) => {
            write!(out, "{{")?;
            for (i, elem) in set.elements.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ")?;
                }
                format(elem.borrow(), ctx, out)?;
            }
            write!(out, "}}")?;
        }

        Identifier(r) => write!(out, "{}", r.target)?,

        Number(x) => write!(out, "{}", x.value)?,

        Boolean(b) => write!(out, "{}", boolean_keyword(b.value))?,

        Undefined => write!(out, "undefined")?,
    }

    if parens {
        write!(out, ")")?;
    }

    Ok(())
}

fn format_case<W: std::io::Write>(
    case: &Case<NodeRef<Node>>,
    ctx: Ctx,
    out: &mut W,
) -> std::io::Result<()> {
    format(case.condition.borrow(), ctx, out)?;
    write!(out, " then\n")?;
    format(case.consequence.borrow(), ctx + 1, out)
}

fn format_set_elem<W: std::io::Write>(
    set: &SetElement<NodeRef<Node>>,
    ctx: Ctx,
    out: &mut W,
) -> std::io::Result<()> {
    write!(out, "{} in ", set.variable)?;
    format(set.domain.borrow(), ctx, out)?;

    if let Some(ref filter) = set.filter {
        write!(out, " such that ")?;
        format(filter.borrow(), ctx, out)?;
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
