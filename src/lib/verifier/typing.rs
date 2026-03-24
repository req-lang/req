use std::{collections::HashMap, fmt::Display, ops::ControlFlow};

use serde::Serialize;

use crate::{
    children::ChildrenIter,
    issue::expression::{ExpressionIssue, ExpressionIssueKind},
    mapper::id::NodeMap,
    syntax::{
        self,
        entity::{Entity, EntityVariant},
        expression::{Case, Expression, ExpressionKey, ExpressionVariant, SetElement},
    },
    verifier::reference::ReferenceMap,
    visitor::{Visitor, Walk, WalkCustom},
};

pub struct Verifier<'expr> {
    nodes_by_id: &'expr NodeMap<'expr>,
    references: &'expr ReferenceMap<'expr>,
    pub results: HashMap<ExpressionKey<'expr>, Result<TypeKind, Option<ExpressionIssue<'expr>>>>,
}

impl<'expr> Walk<&'expr Entity> for Verifier<'expr> {
    fn walk(&mut self, node: &'expr Entity) -> ControlFlow<()> {
        use EntityVariant::*;

        for child in node.children_iter() {
            match child.variant {
                Attribute(_) => continue,
                _ => self.walk(child)?,
            }
        }

        self.visit(node)
    }
}

impl<'expr> Visitor<&'expr Entity> for Verifier<'expr> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'expr Entity) -> ControlFlow<()> {
        for expression in node.expressions() {
            self.walk(expression)?;
        }
        ControlFlow::Continue(())
    }
}

impl<'expr> Walk<&'expr Expression> for Verifier<'expr> {
    fn walk(&mut self, node: &'expr Expression) -> ControlFlow<()> {
        if let Some(set) = node.set_element() {
            self.walk(set.domain.as_ref())?;
        }

        for child in node.children_iter() {
            self.walk(child)?;
        }

        self.visit(node)?;
        ControlFlow::Continue(())
    }
}

impl<'expr> Visitor<&'expr Expression> for Verifier<'expr> {
    type WalkKind = WalkCustom;

    fn visit(&mut self, node: &'expr Expression) -> ControlFlow<()> {
        let result = self.eval(node);
        self.results.insert(node.into(), result);
        ControlFlow::Continue(())
    }
}

impl<'expr> Verifier<'expr> {
    #[inline]
    fn get_result(
        &self,
        node: &'expr Expression,
    ) -> Result<TypeKind, Option<ExpressionIssue<'expr>>> {
        match self.results.get(&node.into()).unwrap() {
            Ok(t) => Ok(t.clone()),
            Err(_) => Err(None),
        }
    }

    fn check_set_element(
        &self,
        set: &'expr SetElement,
    ) -> Result<TypeKind, Option<ExpressionIssue<'expr>>> {
        use ExpressionIssueKind::NotASet;
        use ExpressionIssueKind::UnexpectedType;
        use TypeKind::*;

        let domain = &set.domain;
        let typ = match self.get_result(domain)? {
            Set(t) => t,
            typ => {
                let id = domain.as_ref().into();
                let kind = NotASet(typ);
                return Err(Some(ExpressionIssue::new(id, kind)));
            }
        };

        if let Some(filter) = &set.filter {
            let typ = self.get_result(filter)?;
            if typ != Boolean {
                let id = filter.as_ref().into();
                let kind = UnexpectedType(Boolean, typ);
                return Err(Some(ExpressionIssue::new(id, kind)));
            }
        }

        Ok(*typ.clone())
    }

    fn check_case(
        &self,
        expected: &Option<TypeKind>,
        case: &'expr Case,
    ) -> Result<TypeKind, Option<ExpressionIssue<'expr>>> {
        use ExpressionIssueKind::UnexpectedType;
        use TypeKind::*;

        let cond = self.get_result(&case.condition)?;
        if cond != Boolean {
            let id = case.condition.as_ref().into();
            let kind = UnexpectedType(Boolean, cond);
            return Err(Some(ExpressionIssue::new(id, kind)));
        }

        let cons = self.get_result(&case.consequence)?;
        match expected {
            Some(t) if cons == *t => Ok(cons),
            Some(t) => {
                let id = case.consequence.as_ref().into();
                let kind = UnexpectedType(t.clone(), cons);
                Err(Some(ExpressionIssue::new(id, kind)))
            }
            None => Ok(cons),
        }
    }

    fn eval(
        &mut self,
        node: &'expr Expression,
    ) -> Result<TypeKind, Option<ExpressionIssue<'expr>>> {
        {
            use EntityVariant::*;
            use ExpressionIssueKind::NotASet;
            use ExpressionIssueKind::NotAnAttribute;
            use ExpressionIssueKind::UnexpectedType;
            use ExpressionVariant::*;

            match &node.variant {
                Branch(branch) => {
                    let els = match &branch.otherwise {
                        Some(expr) => Some(self.get_result(&expr)?),
                        None => None,
                    };

                    self.check_case(&els, &branch.case)
                }

                When(when) => {
                    let mut cases = when.cases.iter();
                    let expected = match &when.otherwise {
                        Some(expr) => Some(self.get_result(&expr)?),
                        None => Some(self.get_result(&cases.next().unwrap().consequence)?),
                    };

                    for c in cases {
                        self.check_case(&expected, c)?;
                    }

                    Ok(expected.unwrap())
                }

                Forall(forall) => {
                    use TypeKind::*;

                    self.check_set_element(&forall.set)?;

                    let expr = forall.expression.as_ref();
                    let typ = self.get_result(expr)?;
                    if typ != Boolean {
                        let id = expr.into();
                        let kind = UnexpectedType(Boolean, typ);
                        return Err(Some(ExpressionIssue::new(id, kind)));
                    }

                    Ok(Boolean)
                }

                Exists(exists) => {
                    use TypeKind::*;

                    self.check_set_element(&exists.set)?;
                    Ok(Boolean)
                }

                Select(select) => {
                    use TypeKind::*;

                    let typ = self.check_set_element(&select.set)?;
                    match &select.optimizer {
                        None => Ok(typ),
                        Some(opt) => match self.get_result(&opt.expression)? {
                            Number => Ok(Number),
                            t => {
                                let id = opt.expression.as_ref().into();
                                let kind = UnexpectedType(Number, t);
                                Err(Some(ExpressionIssue::new(id, kind)))
                            }
                        },
                    }
                }

                Aggregation(aggregation) => {
                    use TypeKind::*;

                    for expression in &aggregation.expressions {
                        let type_expr = self.get_result(expression)?;
                        if type_expr != Boolean {
                            let id = expression.into();
                            return Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(Boolean, type_expr),
                            )));
                        }
                    }

                    Ok(Boolean)
                }

                UnaryOp(unary_op) => {
                    use TypeKind::*;
                    use syntax::expression::UnaryOperator::*;

                    let type_op = self.get_result(&unary_op.operand)?;

                    let expected = match unary_op.operator {
                        Plus | Negation | Factorial => Number,
                        Not | Previously | Rise | Fall | Eventually | Always => Boolean,
                    };

                    match type_op {
                        actual if actual != expected => {
                            let id = unary_op.operand.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, actual),
                            )))
                        }
                        _ => Ok(expected),
                    }
                }

                BinOp(bin_op) => {
                    use TypeKind::*;
                    use syntax::expression::BinaryOperator::*;

                    let lhs = self.get_result(&bin_op.left)?;
                    let rhs = self.get_result(&bin_op.right)?;

                    let (expected, result) = match &bin_op.operator {
                        Plus | Minus | Multiply | Divide | Modulus | Power => (Number, Number),
                        And | Or | Xor | Implies | Iff | Since => (Boolean, Boolean),
                        GreaterThan | LessThan | GreaterOrEqual | LessOrEqual => (Number, Boolean),
                        NotEqual | Equal => (lhs.clone(), Boolean),
                        Union | Intersection | Difference | Complement => {
                            (lhs.clone(), lhs.clone())
                        }
                        In => match &rhs {
                            TypeKind::Set(rhs) => (*rhs.clone(), Boolean),
                            _ => {
                                return Err(Some(ExpressionIssue::new(
                                    bin_op.right.as_ref().into(),
                                    NotASet(rhs),
                                )));
                            }
                        },
                        Includes => match &rhs {
                            TypeKind::Set(rhs) => (TypeKind::Set(rhs.clone()), Boolean),
                            _ => {
                                return Err(Some(ExpressionIssue::new(
                                    bin_op.right.as_ref().into(),
                                    NotASet(rhs),
                                )));
                            }
                        },
                    };

                    match (&bin_op.operator, lhs, rhs) {
                        (In, lhs, _) if lhs == expected => Ok(result),
                        (In, lhs, _) => {
                            let id = bin_op.left.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, lhs),
                            )))
                        }
                        (Union | Intersection | Includes | Complement | Difference, lhs, _)
                            if !lhs.is_set() =>
                        {
                            let id = bin_op.left.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, lhs),
                            )))
                        }
                        (Union | Intersection | Includes | Complement | Difference, _, rhs)
                            if !rhs.is_set() =>
                        {
                            let id = bin_op.right.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, rhs),
                            )))
                        }
                        (_, lhs, _) if lhs != expected => {
                            let id = bin_op.left.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, lhs),
                            )))
                        }
                        (_, _, rhs) if rhs != expected => {
                            let id = bin_op.right.as_ref().into();
                            Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(expected, rhs),
                            )))
                        }
                        (_, _, _) => Ok(result),
                    }
                }

                Set(set) => {
                    let mut se = set.elements.iter();
                    let type_first = self.get_result(se.next().unwrap())?;

                    for elem in se {
                        let type_elem = self.get_result(elem)?;
                        if type_elem != type_first {
                            let id = elem.into();
                            return Err(Some(ExpressionIssue::new(
                                id,
                                UnexpectedType(type_first, type_elem),
                            )));
                        }
                    }

                    Ok(TypeKind::Set(Box::new(type_first)))
                }

                Function(_) => todo!(),

                Identifier(ident) if is_builtin_number(&ident.target.value) => {
                    Ok(TypeKind::Set(Box::new(TypeKind::Number)))
                }

                Identifier(ident) if is_builtin_boolean(&ident.target.value) => {
                    Ok(TypeKind::Set(Box::new(TypeKind::Boolean)))
                }

                Identifier(ident) => match self.references.get(ident.target.into()) {
                    Some(id) => match &self.nodes_by_id.entities.get(id) {
                        Some(entity) => match &entity.variant {
                            Attribute(attribute) => {
                                match self.results.get(*attribute.domain.into()) {
                                    Some(Ok(TypeKind::Set(typ))) => Ok(*typ.clone()),
                                    _ => Err(None),
                                }
                            }

                            _ => Err(Some(ExpressionIssue::new(
                                *node.into(),
                                NotAnAttribute(*entity.into()),
                            ))),
                        },
                        None => Err(None), // TODO: Restore behavior with local variables (set
                                           // variable)
                    },
                    None => Err(None),
                },

                Number(_) => Ok(TypeKind::Number),
                Boolean(_) => Ok(TypeKind::Boolean),
                Undefined => Ok(TypeKind::Undefined),
            }
        }
    }
}

pub type TypeMap<'expr> = HashMap<ExpressionKey<'expr>, TypeKind>;

impl<'expr> Verifier<'expr> {
    pub fn new(references: &'expr ReferenceMap, nodes_by_id: &'expr NodeMap) -> Self {
        Self {
            references,
            nodes_by_id,
            results: HashMap::new(),
        }
    }

    pub fn prewalk(&mut self, node: &'expr Entity) {
        use EntityVariant::*;
        let attributes: Vec<_> = node
            .iter()
            .filter(|n| match &n.variant {
                Attribute(_) => true,
                _ => false,
            })
            .collect();

        for attr in attributes {
            let _ = self.visit(attr);
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum TypeKind {
    Number,
    Boolean,
    Undefined,
    Set(Box<TypeKind>),
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TypeKind {
    pub fn is_set(&self) -> bool {
        match self {
            TypeKind::Set(_) => true,
            _ => false,
        }
    }
}

#[inline]
pub fn is_builtin_number(val: &str) -> bool {
    match val {
        "real" | "integer" => true,
        _ => false,
    }
}

#[inline]
pub fn is_builtin_boolean(val: &str) -> bool {
    match val {
        "boolean" => true,
        _ => false,
    }
}
