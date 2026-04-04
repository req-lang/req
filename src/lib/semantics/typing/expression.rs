use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

use crate::{
    issue::Severity,
    node::{
        self, NodeRef,
        expression::{Case, SetElement},
    },
    reference::Reference,
    semantics::{
        reference::{
            self,
            expression::{BuiltinKind, Target},
        },
        typing::entity,
    },
    utils::children::ChildrenIter,
};

#[derive(Eq, PartialEq, Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("previously caught error")]
    Poison,
    #[error("expected a set got {0}")]
    NotASet(TypeKind),
    #[error("expected {0} got {1}")]
    UnexpectedType(TypeKind, TypeKind),
    #[error("cannot retrieve the domain for this identifier")]
    DomainNotFound,
}

impl From<&Issue> for Severity {
    fn from(_: &Issue) -> Self {
        use Severity::*;

        Critical
    }
}

pub(super) type Map = HashMap<*const RefCell<reference::expression::Node>, NodeRef<Node>>;

pub type Variant =
    node::expression::Variant<NodeRef<Node>, Result<Target<entity::Node, Node>, Reference>>;

#[derive(Debug)]
pub struct Node {
    pub typ: Result<TypeKind, Issue>,
    pub variant: Variant,
}

impl ChildrenIter<NodeRef<Node>> for NodeRef<Node> {
    fn children_iter(&self) -> impl Iterator<Item = NodeRef<Node>> {
        self.borrow()
            .variant
            .children_iter()
            .map(|n| n.clone())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

pub(super) fn evaluate(node: NodeRef<Node>) {
    use node::entity::Variant::*;
    use node::expression::Variant::*;

    for child in node.borrow().variant.children_iter() {
        evaluate(child);
    }

    if let Identifier(ident) = &node.borrow().variant {
        if let Ok(Target::Entity(n)) = &ident.target {
            if let Some(n) = n.upgrade() {
                if let Attribute(atr) = &n.borrow().variant
                    && atr.domain.borrow().typ == Ok(TypeKind::Undefined)
                {
                    evaluate(Rc::clone(&atr.domain));
                }
            }
        }

        if let Ok(Target::Expression(n)) = &ident.target {
            if let Some(n) = n.upgrade()
                && n.borrow().typ == Ok(TypeKind::Undefined)
            {
                evaluate(n);
            }
        }
    }

    node.borrow_mut().typ = evaluate_node(node.borrow());
}

fn evaluate_case(
    case: &Case<NodeRef<Node>>,
    expected: Option<TypeKind>,
) -> Result<TypeKind, Issue> {
    use Issue::*;
    use TypeKind::*;

    let cond_ref = case.condition.borrow();
    let cond = cond_ref.typ.as_ref().map_err(|_| Poison)?;
    if *cond != Boolean {
        return Err(UnexpectedType(Boolean, cond.clone()));
    }

    let cons_ref = case.consequence.borrow();
    let cons = cons_ref.typ.as_ref().map_err(|_| Poison)?.clone();
    match expected {
        Some(exp) if cons == exp => Ok(cons),
        Some(exp) => Err(UnexpectedType(exp.clone(), cons)),
        None => Ok(cons),
    }
}

fn evaluate_set_element(set: &SetElement<NodeRef<Node>>) -> Result<TypeKind, Issue> {
    use Issue::*;
    use TypeKind::*;

    let domain = &set.domain;
    let domain_ref = domain.borrow();
    let typ = match domain_ref.typ.as_ref().map_err(|_| Poison)? {
        Set(t) => t,
        typ => return Err(NotASet(typ.clone())),
    };

    if let Some(filter) = &set.filter {
        let filter_ref = filter.borrow();
        let typ = filter_ref.typ.as_ref().map_err(|_| Poison)?;
        if *typ != Boolean {
            return Err(UnexpectedType(Boolean, typ.clone()));
        }
    }

    Ok(*typ.clone())
}

fn evaluate_node(node: std::cell::Ref<Node>) -> Result<TypeKind, Issue> {
    use Issue::*;
    use node::entity::Variant::*;
    use node::expression::Variant::*;

    match &node.variant {
        Branch(branch) => {
            let els = match &branch.otherwise {
                Some(expr) => Some(expr.borrow().typ.as_ref().map_err(|_| Poison)?.clone()),
                None => None,
            };

            evaluate_case(&branch.case, els)
        }

        When(when) => {
            let mut cases = when.cases.iter();
            let expected = match &when.otherwise {
                Some(expr) => expr.borrow().typ.as_ref().map_err(|_| Poison)?.clone(),
                None => {
                    let first = cases.next().unwrap();
                    let first_ref = first.consequence.borrow();
                    first_ref.typ.as_ref().map_err(|_| Poison)?.clone()
                }
            };

            for c in cases {
                evaluate_case(c, Some(expected.clone()))?;
            }

            Ok(expected)
        }

        Forall(forall) => {
            use TypeKind::*;

            evaluate_set_element(&forall.set)?;

            let expr = forall.expression.borrow();
            let typ = expr.typ.as_ref().map_err(|_| Poison)?;
            if *typ != Boolean {
                return Err(UnexpectedType(Boolean, typ.clone()));
            }

            Ok(Boolean)
        }

        Exists(exists) => {
            use TypeKind::*;

            evaluate_set_element(&exists.set)?;
            Ok(Boolean)
        }

        Select(select) => {
            use TypeKind::*;

            let typ = evaluate_set_element(&select.set)?;
            match &select.optimizer {
                None => Ok(typ),
                Some(opt) => match opt.expression.borrow().typ.as_ref().map_err(|_| Poison)? {
                    Number => Ok(Number),
                    t => Err(UnexpectedType(Number, t.clone())),
                },
            }
        }

        Aggregation(aggregation) => {
            use TypeKind::*;

            for expression in &aggregation.expressions {
                let expr_ref = expression.borrow();
                let t = expr_ref.typ.as_ref().map_err(|_| Poison)?;
                if *t != Boolean {
                    return Err(UnexpectedType(Boolean, t.clone()));
                }
            }

            Ok(Boolean)
        }

        UnaryOp(una) => {
            use TypeKind::*;
            use node::expression::UnaryOperator::*;

            let op_ref = una.operand.borrow();
            let type_op = op_ref.typ.as_ref().map_err(|_| Poison)?;
            let expected = match una.operator {
                Plus | Negation | Factorial => Number,
                Not | Previously | Rise | Fall | Eventually | Always => Boolean,
            };

            match type_op {
                actual if *actual != expected => {
                    return Err(UnexpectedType(expected, actual.clone()));
                }
                _ => Ok(expected),
            }
        }

        BinOp(bin) => {
            use TypeKind::*;
            use node::expression::BinaryOperator::*;

            let lhs_ref = bin.left.borrow();
            let rhs_ref = bin.right.borrow();

            let lhs = lhs_ref.typ.as_ref().map_err(|_| Poison)?;
            let rhs = rhs_ref.typ.as_ref().map_err(|_| Poison)?;

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
                (In, lhs, _) if *lhs == expected => Ok(result),
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
                (_, lhs, _) if *lhs != expected => Err(UnexpectedType(expected, lhs.clone())),
                (_, _, rhs) if *rhs != expected => Err(UnexpectedType(expected, rhs.clone())),
                (_, _, _) => Ok(result),
            }
        }

        Set(set) => {
            let mut se = set.elements.iter();
            let first_ref = se.next().unwrap().borrow();
            let type_first = first_ref.typ.as_ref().map_err(|_| Poison)?;

            for elem in se {
                let elem_ref = elem.borrow();
                let type_elem = elem_ref.typ.as_ref().map_err(|_| Poison)?;
                if type_elem != type_first {
                    return Err(UnexpectedType(type_first.clone(), type_elem.clone()));
                }
            }

            Ok(TypeKind::Set(Box::new(type_first.clone())))
        }

        Function(_) => todo!(),

        Identifier(ident) => match ident.target.as_ref().map_err(|_| DomainNotFound)? {
            Target::Entity(node) => match node.upgrade().ok_or(Poison)?.borrow().variant {
                Attribute(ref atr) => match atr.domain.borrow().typ.as_ref().map_err(|_| Poison)? {
                    TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                    typ => Err(NotASet(typ.clone())),
                },
                _ => Err(DomainNotFound),
            },
            Target::Expression(node) => {
                let node = node.upgrade().ok_or(Poison)?;
                match node.borrow().typ.as_ref().map_err(|_| Poison)? {
                    TypeKind::Set(inner) => Ok(inner.as_ref().clone()),
                    typ => Err(NotASet(typ.clone())),
                }
            }
            Target::Builtin(kind) => Ok(TypeKind::from(*kind)),
        },

        Number(_) => Ok(TypeKind::Number),

        Boolean(_) => Ok(TypeKind::Boolean),

        Undefined => Ok(TypeKind::Undefined), // Undefined behavior should be studied
    }
}

pub(super) fn init_node(
    node: NodeRef<reference::expression::Node>,
    entities: &entity::Map,
    expressions: &mut Map,
) -> NodeRef<Node> {
    let key = Rc::as_ptr(&node);
    let inner = node::unwrap(node);
    let variant = inner
        .variant
        .mape(|child_ref| init_node(child_ref, entities, expressions))
        .mapr(|r| r.map(|t| convert_target(t, entities, expressions)));
    let node = node::new(Node {
        variant,
        typ: Ok(TypeKind::Undefined),
    });
    expressions.insert(key, Rc::clone(&node));
    node
}

fn convert_target(
    target: Target<reference::entity::Node, reference::expression::Node>,
    entities: &entity::Map,
    expressions: &Map,
) -> Target<entity::Node, Node> {
    use Target::*;

    match target {
        Builtin(kind) => Builtin(kind),
        Entity(weak) => Entity(Rc::downgrade(&entities[&weak.as_ptr()])),
        Expression(weak) => Expression(Rc::downgrade(&expressions[&weak.as_ptr()])),
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
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

impl From<BuiltinKind> for TypeKind {
    fn from(value: BuiltinKind) -> Self {
        match value {
            BuiltinKind::Real | BuiltinKind::Integer => TypeKind::Set(Box::new(TypeKind::Number)),
            BuiltinKind::Boolean => TypeKind::Set(Box::new(TypeKind::Boolean)),
        }
    }
}
