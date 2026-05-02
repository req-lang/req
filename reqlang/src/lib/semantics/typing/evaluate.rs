use std::cell::{Ref, RefCell};

use crate::{
    node::{
        self,
        expression::{Arena, Case, Idx, SetElement},
    },
    semantics::{
        link::{
            self,
            expression::{BuiltinKind, Target},
        },
        typing::{
            TypeResult,
            expression::{Issue, TypeKind},
        },
    },
};

pub(super) fn evaluate_node<P>(
    types: &node::entity::Arena<Option<RefCell<Arena<Option<TypeResult>>>>>,
    entity: node::entity::Idx,
    arena: &Arena<&link::expression::Node<P>>,
    idx: Idx,
) -> TypeResult {
    use Issue::*;
    use node::expression::Variant::*;

    let local = &types[entity].as_ref().unwrap().borrow();
    match &arena[idx].variant {
        Branch(branch) => {
            let els = match &branch.otherwise {
                Some(expr) => Some(type_or_poison(&local, *expr)?),
                None => None,
            };

            evaluate_case(&local, &branch.case, els)
        }

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

        Forall(forall) => {
            use TypeKind::*;

            evaluate_set_element(&local, &forall.set)?;

            let expr = type_or_poison(&local, forall.expression)?;
            if expr != Boolean {
                return Err(UnexpectedType(Boolean, expr.clone()));
            }

            Ok(Boolean)
        }

        Exists(exists) => {
            use TypeKind::*;

            evaluate_set_element(&local, &exists.set)?;
            Ok(Boolean)
        }

        Select(select) => {
            use TypeKind::*;

            let domain = evaluate_set_element(&local, &select.set)?;
            match &select.optimizer {
                None => Ok(domain),
                Some(opt) => match type_or_poison(&local, opt.expression)? {
                    Number => Ok(Number),
                    t => Err(UnexpectedType(Number, t.clone())),
                },
            }
        }

        Aggregation(aggregation) => {
            use TypeKind::*;

            for expression in &aggregation.expressions {
                let expr = type_or_poison(&local, *expression)?;
                if expr != Boolean {
                    return Err(UnexpectedType(Boolean, expr.clone()));
                }
            }

            Ok(Boolean)
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
                NotEqual | Equal => (lhs.clone(), Boolean),
                Union | Intersection | Difference | Complement => (lhs.clone(), lhs.clone()),
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
            let first = se.next().unwrap();
            let first = type_or_poison(&local, *first)?;
            for elem in se {
                let elem = type_or_poison(&local, *elem)?;
                if elem != first {
                    return Err(UnexpectedType(first.clone(), elem.clone()));
                }
            }

            Ok(TypeKind::Set(Box::new(first.clone())))
        }

        Function(_) => {
            eprintln!("not yet implemented");
            Ok(TypeKind::Undefined)
        }

        Identifier(ident) => match &ident.target.target().ok_or(Poison)? {
            Target::Entity(tgt) => match &types[*tgt] {
                Some(typs) => match type_or_poison(&typs.borrow(), 0.into())? {
                    TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                    typ => Err(NotASet(typ.clone())),
                },
                None => Err(NotEvaluable),
            },
            Target::Expression(tgt) => match type_or_poison(&local, *tgt)? {
                TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                typ => Err(NotASet(typ.clone())),
            },
            Target::Builtin(kind) => Ok(TypeKind::from(*kind)),
        },

        Number(_) => Ok(TypeKind::Number),

        Boolean(_) => Ok(TypeKind::Boolean),

        Undefined => Ok(TypeKind::Undefined), // Undefined behavior should be studied
    }
}

fn evaluate_case(
    types: &Ref<'_, Arena<Option<TypeResult>>>,
    case: &Case<Idx>,
    expected: Option<TypeKind>,
) -> TypeResult {
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

fn evaluate_set_element(
    types: &Ref<'_, Arena<Option<TypeResult>>>,
    set: &SetElement<Idx>,
) -> TypeResult {
    use Issue::*;
    use TypeKind::*;

    let typ = match type_or_poison(types, set.domain)? {
        Set(t) => t,
        typ => return Err(NotASet(typ.clone())),
    };

    if let Some(filter) = &set.filter {
        let typ = type_or_poison(types, *filter)?;
        if typ != Boolean {
            return Err(UnexpectedType(Boolean, typ.clone()));
        }
    }

    Ok(*typ.clone())
}

fn type_or_poison(types: &Ref<'_, Arena<Option<TypeResult>>>, idx: Idx) -> TypeResult {
    types[idx]
        .clone()
        .unwrap()
        .map_err(|_| Issue::Poison)
        .clone()
}

impl From<BuiltinKind> for TypeKind {
    fn from(value: BuiltinKind) -> Self {
        match value {
            BuiltinKind::Real | BuiltinKind::Integer => TypeKind::Set(Box::new(TypeKind::Number)),
            BuiltinKind::Boolean => TypeKind::Set(Box::new(TypeKind::Boolean)),
        }
    }
}
