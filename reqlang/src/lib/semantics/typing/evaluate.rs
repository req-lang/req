use crate::{
    node::{
        self, entity,
        expression::{Aggregator, Arena, Case, Idx},
        tree::Tree,
    },
    semantics::{
        link::{BuiltinKind, Links, Target::*},
        typing::{
            Result,
            expression::{Issue, TypeKind},
        },
    },
};

/// The type of an entity is the one of its first expression, a type is a set of its own values
fn entity_type(tree: &Tree, types: &Arena<Option<Result>>, idx: entity::Idx) -> Option<Result> {
    match &tree.entities[idx].variant {
        entity::Variant::Type(_) => Some(Ok(TypeKind::Set(Box::new(TypeKind::Custom(idx))))),
        variant => variant
            .expressions()
            .next()
            .map(|root| type_or_poison(types, root)),
    }
}

// INVARIANT: the types of the children, and of the targets of the identifiers, are already known
pub(super) fn evaluate_node(
    tree: &Tree,
    links: &Links,
    local: &Arena<Option<Result>>,
    expression: Idx,
) -> Result {
    use Issue::*;
    use node::expression::Variant::*;
    match &tree.expressions[expression].variant {
        When(when) => {
            let mut cases = when.cases.iter();
            let expected = match &when.otherwise {
                Some(expr) => *expr,
                None => cases.next().unwrap().consequence,
            };

            let expected = type_or_poison(&local, expected)?;
            for c in cases {
                evaluate_case(&local, c, Some(expected.clone()))?;
            }

            Ok(expected)
        }

        Aggregation(aggregation) => {
            aggregation
                .aggregator
                .check(&local, &aggregation.expressions, None)
        }

        Comprehension(comprehension) => {
            use TypeKind::*;

            let set = &comprehension.set;
            let element = match type_or_poison(&local, set.domain)? {
                Set(t) => *t,
                typ => return Err(NotASet(typ)),
            };

            if let Some(filter) = &set.filter {
                let typ = type_or_poison(&local, *filter)?;
                if typ != Boolean {
                    return Err(UnexpectedType(Boolean, typ));
                }
            }

            match &comprehension.aggregator {
                None => Ok(Set(Box::new(type_or_poison(
                    &local,
                    comprehension.expression,
                )?))),
                Some(aggregator) => {
                    aggregator.check(&local, &[comprehension.expression], Some(element))
                }
            }
        }

        UnaryOp(una) => {
            use TypeKind::*;
            use node::expression::UnaryOperator::*;

            let operand = type_or_poison(&local, una.operand)?;
            let expected = match una.operator {
                Plus | Negation | Factorial => Number,
                Not | Previously | Rise | Fall | Eventually | Always => Boolean,
            };

            match operand {
                actual if actual != expected => {
                    return Err(UnexpectedType(expected, actual.clone()));
                }
                _ => Ok(expected),
            }
        }

        BinOp(bin) => {
            use TypeKind::*;
            use node::expression::BinaryOperator::*;

            let lhs = type_or_poison(&local, bin.left)?;
            let rhs = type_or_poison(&local, bin.right)?;

            let (expected, result) = match &bin.operator {
                Plus | Minus | Multiply | Divide | Modulus | Power => (Number, Number),
                And | Or | Xor | Implies | Iff | Since => (Boolean, Boolean),
                GreaterThan | LessThan | GreaterOrEqual | LessOrEqual => (Number, Boolean),
                NotEqual | Equal | Fix | Initially => (lhs.clone(), Boolean),
                Union | Intersection | Difference | Complement => (lhs.clone(), lhs.clone()),
                Tolerance => (Number, Set(Box::new(Number))),
                In => match &rhs {
                    TypeKind::Set(rhs) => (*rhs.clone(), Boolean),
                    _ => return Err(NotASet(rhs.clone())),
                },
                Includes => match &rhs {
                    TypeKind::Set(rhs) => (TypeKind::Set(rhs.clone()), Boolean),
                    _ => return Err(NotASet(rhs.clone())),
                },
            };

            match (&bin.operator, lhs, rhs) {
                (In, lhs, _) if lhs == expected => Ok(result),
                (In, lhs, _) => Err(UnexpectedType(expected, lhs.clone())),
                (Union | Intersection | Includes | Complement | Difference, lhs, _)
                    if !lhs.is_set() =>
                {
                    Err(UnexpectedType(expected, lhs.clone()))
                }

                (Union | Intersection | Includes | Complement | Difference, _, rhs)
                    if !rhs.is_set() =>
                {
                    Err(UnexpectedType(expected, rhs.clone()))
                }
                (_, lhs, _) if lhs != expected => Err(UnexpectedType(expected, lhs.clone())),
                (_, _, rhs) if rhs != expected => Err(UnexpectedType(expected, rhs.clone())),
                (_, _, _) => Ok(result),
            }
        }

        Set(set) => {
            let mut se = set.elements.iter();
            let first = se.next().ok_or(NotEvaluable)?;
            let first = type_or_poison(&local, *first)?;
            for elem in se {
                let elem = type_or_poison(&local, *elem)?;
                if elem != first {
                    return Err(UnexpectedType(first.clone(), elem.clone()));
                }
            }

            Ok(TypeKind::Set(Box::new(first.clone())))
        }

        Interval(interval) => {
            let left = type_or_poison(&local, interval.left.expression)?;
            let right = type_or_poison(&local, interval.right.expression)?;
            match (left, right) {
                (TypeKind::Number, TypeKind::Number) => {
                    Ok(TypeKind::Set(Box::new(TypeKind::Number)))
                }
                (TypeKind::Number, other) => Err(UnexpectedType(TypeKind::Number, other)),
                (other, _) => Err(UnexpectedType(TypeKind::Number, other)),
            }
        }

        Range(range) => {
            let left = type_or_poison(&local, range.left)?;
            let right = type_or_poison(&local, range.right)?;
            match (left, right) {
                (TypeKind::Number, TypeKind::Number) => {
                    Ok(TypeKind::Set(Box::new(TypeKind::Number)))
                }
                (TypeKind::Number, other) => Err(UnexpectedType(TypeKind::Number, other)),
                (other, _) => Err(UnexpectedType(TypeKind::Number, other)),
            }
        }

        Function(_) => {
            eprintln!("not yet implemented");
            Ok(TypeKind::Undefined)
        }

        Identifier(ident) => match &links.identifiers[ident.target].ok_or(Poison)? {
            Entity(tgt) | Enum(tgt, _) => match entity_type(tree, local, *tgt) {
                Some(result) => match result? {
                    TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                    TypeKind::Custom(idx) => Ok(TypeKind::Custom(idx)),
                    typ => Err(NotASet(typ.clone())),
                },
                None => Err(NotEvaluable),
            },
            Local(tgt) => {
                let domain = tree.expressions[*tgt].variant.set_element().unwrap().domain;
                match type_or_poison(&local, domain)? {
                    TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                    TypeKind::Custom(idx) => Ok(TypeKind::Custom(idx)),
                    typ => Err(NotASet(typ.clone())),
                }
            }
            Builtin(kind) => Ok(TypeKind::from(*kind)),
        },

        Number(_) => Ok(TypeKind::Number),

        Boolean(_) => Ok(TypeKind::Boolean),

        Undefined => Ok(TypeKind::Undefined), // Undefined behavior should be studied
    }
}

fn evaluate_case(types: &Arena<Option<Result>>, case: &Case, expected: Option<TypeKind>) -> Result {
    use Issue::*;
    use TypeKind::*;

    let cond = type_or_poison(types, case.condition)?;
    if cond != Boolean {
        return Err(UnexpectedType(Boolean, cond.clone()));
    }

    let cons = type_or_poison(types, case.consequence)?;
    match expected {
        Some(exp) if cons == exp => Ok(cons),
        Some(exp) => Err(UnexpectedType(exp.clone(), cons)),
        None => Ok(cons),
    }
}

impl Aggregator {
    /// Type of the aggregate. The operands are the items of the list, or the body of
    /// a comprehension, in which case `element` is the type of its variable.
    fn check(
        &self,
        types: &Arena<Option<Result>>,
        operands: &[Idx],
        element: Option<TypeKind>,
    ) -> Result {
        use Aggregator::*;
        use Issue::*;
        use TypeKind::*;

        let first = operands.first().ok_or(NotEvaluable);
        let (expected, result) = match self {
            All | Any | One => (Boolean, Boolean),
            Maximum | Minimum | Supremum | Infimum | Sum | Product => (Number, Number),
            // The set of maximizers: variable values for `for`, 1-based positions for a list
            ArgMaximum | ArgMinimum => (Number, Set(Box::new(element.unwrap_or(Number)))),
            Union | Intersection => {
                let first = type_or_poison(types, *first?)?;
                match first {
                    Set(x) => (Set(x.clone()), Set(x)),
                    t => return Err(NotASet(t)),
                }
            }
            Choose => {
                let typ = type_or_poison(types, *first?)?;
                (typ.clone(), typ)
            }
            Count => {
                let typ = type_or_poison(types, *first?)?;
                (typ, Number)
            }
        };

        for operand in operands {
            let typ = type_or_poison(types, *operand)?;
            if typ != expected {
                return Err(UnexpectedType(expected, typ));
            }
        }

        Ok(result)
    }
}

fn type_or_poison(types: &Arena<Option<Result>>, idx: Idx) -> Result {
    types[idx]
        .clone()
        .unwrap()
        .map_err(|_| Issue::Poison)
        .clone()
}

impl From<BuiltinKind> for TypeKind {
    fn from(value: BuiltinKind) -> Self {
        use BuiltinKind::*;

        match value {
            Real | Integer => TypeKind::Set(Box::new(TypeKind::Number)),
            Boolean => TypeKind::Set(Box::new(TypeKind::Boolean)),
        }
    }
}
