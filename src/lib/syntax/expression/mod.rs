use std::hash::Hash;

use super::entity;
use crate::{
    children::{ChildrenIter, ChildrenIterMut},
    iter::NodeIter,
    key::NodeKey,
};
use parser::Rule;

pub mod parser;

pub(crate) type ExpressionKey<'a> = NodeKey<'a, Expression>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Expression<R = entity::Reference> {
    pub variant: ExpressionVariant<R>,
}

impl Default for Expression {
    fn default() -> Self {
        Expression {
            variant: ExpressionVariant::Undefined,
        }
    }
}

impl<R> ChildrenIter for Expression<R> {
    fn children_iter(&self) -> impl Iterator<Item = &Expression<R>> {
        use ExpressionVariant::*;

        let mut children = Vec::new();
        match &self.variant {
            Branch(branch) => {
                let bo = &branch.otherwise;
                children.push(branch.case.condition.as_ref());
                children.push(branch.case.consequence.as_ref());
                children.extend(bo.iter().map(|c| c.as_ref()));
            }

            When(when) => {
                let cases = when
                    .cases
                    .iter()
                    .map(|c| [c.condition.as_ref(), c.consequence.as_ref()])
                    .flatten();

                children.extend(cases);
                children.extend(when.otherwise.iter().map(|c| c.as_ref()));
            }

            Forall(forall) => {
                let fs = &forall.set;
                children.push(&*fs.domain);
                children.extend(fs.filter.iter().map(|n| n.as_ref()));
                children.push(forall.expression.as_ref());
            }

            Exists(exists) => {
                let es = &exists.set;
                children.push(es.domain.as_ref());
                children.extend(es.filter.iter().map(|n| n.as_ref()));
            }

            Select(select) => {
                let ss = &select.set;
                let so = &select.optimizer;
                children.push(ss.domain.as_ref());
                children.extend(ss.filter.iter().map(|n| n.as_ref()));
                children.extend(so.iter().map(|n| n.expression.as_ref()));
            }

            Aggregation(aggregation) => {
                let ae = &aggregation.expressions;
                children.extend(ae.iter());
            }

            UnaryOp(unary) => {
                children.push(unary.operand.as_ref());
            }

            BinOp(binary) => {
                children.push(binary.left.as_ref());
                children.push(binary.right.as_ref());
            }

            Function(function) => {
                let fa = &function.arguments;
                children.push(function.expression.as_ref());
                children.extend(fa.iter());
            }

            Set(set) => {
                let se = &set.elements;
                children.extend(se.iter());
            }

            Identifier(_) => {}
            Number(_) => {}
            Boolean(_) => {}
            Undefined => {}
        };

        children.into_iter()
    }
}

impl<R> ChildrenIterMut for Expression<R> {
    fn children_iter_mut(&mut self) -> impl Iterator<Item = &mut Expression<R>> {
        use ExpressionVariant::*;

        let mut children = Vec::new();
        match &mut self.variant {
            Branch(branch) => {
                let bo = &mut branch.otherwise;
                children.push(branch.case.condition.as_mut());
                children.push(branch.case.consequence.as_mut());
                children.extend(bo.iter_mut().map(|c| c.as_mut()));
            }
            When(when) => {
                let cases = when
                    .cases
                    .iter_mut()
                    .map(|c| [c.condition.as_mut(), c.consequence.as_mut()])
                    .flatten();
                children.extend(cases);
                children.extend(when.otherwise.iter_mut().map(|c| c.as_mut()));
            }
            Forall(forall) => {
                let fs = &mut forall.set;
                children.push(fs.domain.as_mut());
                children.extend(fs.filter.iter_mut().map(|n| n.as_mut()));
                children.push(forall.expression.as_mut());
            }
            Exists(exists) => {
                let es = &mut exists.set;
                children.push(es.domain.as_mut());
                children.extend(es.filter.iter_mut().map(|n| n.as_mut()));
            }
            Select(select) => {
                let ss = &mut select.set;
                let so = &mut select.optimizer;
                children.push(ss.domain.as_mut());
                children.extend(ss.filter.iter_mut().map(|n| n.as_mut()));
                children.extend(so.iter_mut().map(|n| n.expression.as_mut()));
            }
            Aggregation(aggregation) => {
                let ae = &mut aggregation.expressions;
                children.extend(ae.iter_mut());
            }
            UnaryOp(unary) => {
                children.push(unary.operand.as_mut());
            }
            BinOp(binary) => {
                children.push(binary.left.as_mut());
                children.push(binary.right.as_mut());
            }
            Function(function) => {
                let fa = &mut function.arguments;
                children.push(function.expression.as_mut());
                children.extend(fa.iter_mut());
            }
            Set(set) => {
                let se = &mut set.elements;
                children.extend(se.iter_mut());
            }

            Identifier(_) => {}
            Number(_) => {}
            Boolean(_) => {}
            Undefined => {}
        };

        children.into_iter()
    }
}

impl<R> Expression<R> {
    pub fn new(variant: ExpressionVariant<R>) -> Self {
        Expression { variant }
    }

    pub fn needs_parentheses(&self, parent: &Expression<R>, is_right: bool) -> bool {
        let parent_prec = parent.variant.precedence();
        let child_prec = self.variant.precedence();

        if child_prec < parent_prec {
            return true;
        }

        if child_prec == parent_prec {
            if let (Some(parent_assoc), Some(_)) =
                (parent.variant.associativity(), self.variant.associativity())
            {
                match parent_assoc {
                    Associativity::Right => !is_right,
                    Associativity::Left => is_right,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn set_element(&self) -> Option<&SetElement<R>> {
        use ExpressionVariant::*;

        match &self.variant {
            Forall(forall) => Some(&forall.set),
            Exists(exists) => Some(&exists.set),
            Select(select) => Some(&select.set),
            _ => None,
        }
    }
}

impl Expression {
    pub fn iter(&self) -> impl Iterator<Item = &Self> {
        NodeIter::new(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpressionVariant<R = entity::Reference> {
    Branch(Branch<R>),
    When(When<R>),
    Forall(Forall<R>),
    Exists(Exists<R>),
    Select(Select<R>),
    Aggregation(Aggregation<R>),

    UnaryOp(UnaryOp<R>),
    BinOp(BinOp<R>),
    Function(Function<R>),

    Identifier(Identifier<R>),
    Number(Number),
    Boolean(Boolean),
    Undefined,

    Set(Set<R>),
}

impl<R> ExpressionVariant<R> {
    pub fn precedence(&self) -> i32 {
        match self {
            ExpressionVariant::BinOp(bin_op) => bin_op.operator.precedence(),
            ExpressionVariant::UnaryOp(unary_op) => unary_op.operator.precedence(),
            ExpressionVariant::Function(_) => 160,
            _ => 1000,
        }
    }

    pub fn associativity(&self) -> Option<Associativity> {
        match self {
            ExpressionVariant::BinOp(bin_op) => Some(bin_op.operator.associativity()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Branch<R = entity::Reference> {
    pub case: Case<R>,
    pub otherwise: Option<Box<Expression<R>>>,
}

impl Branch {
    pub fn new(case: Case) -> Self {
        Branch {
            case,
            otherwise: None,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct When<R = entity::Reference> {
    pub cases: Vec<Case<R>>,
    pub otherwise: Option<Box<Expression<R>>>,
}

impl When {
    pub fn new(cases: Vec<Case>) -> Self {
        When {
            cases,
            otherwise: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case<R = entity::Reference> {
    pub condition: Box<Expression<R>>,
    pub consequence: Box<Expression<R>>,
}

impl Case {
    pub fn new(condition: Expression, consequence: Expression) -> Self {
        Case {
            condition: Box::new(condition),
            consequence: Box::new(consequence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Forall<R = entity::Reference> {
    pub set: SetElement<R>,
    pub expression: Box<Expression<R>>,
}

impl Forall {
    pub fn new(set: SetElement, expression: Expression) -> Self {
        Forall {
            set,
            expression: Box::new(expression),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Exists<R = entity::Reference> {
    pub set: SetElement<R>,
}

impl Exists {
    pub fn new(set: SetElement) -> Self {
        Exists { set }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Select<R = entity::Reference> {
    pub set: SetElement<R>,
    pub optimizer: Option<Optimizer<R>>,
}

impl Select {
    pub fn new(set: SetElement) -> Self {
        Select {
            set,
            optimizer: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Optimizer<R = entity::Reference> {
    pub kind: OptimizerKind,
    pub expression: Box<Expression<R>>,
}

impl Optimizer {
    pub fn new(kind: OptimizerKind, expression: Expression) -> Self {
        Optimizer {
            kind,
            expression: Box::new(expression),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OptimizerKind {
    Minimize,
    Maximize,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetElement<R = entity::Reference> {
    pub variable: String,
    pub domain: Box<Expression<R>>,
    pub filter: Option<Box<Expression<R>>>,
}

impl SetElement {
    pub fn new(variable: String, domain: Expression) -> Self {
        SetElement {
            variable,
            domain: Box::new(domain),
            filter: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Aggregation<R = entity::Reference> {
    pub aggregator: Aggregator,
    pub expressions: Vec<Expression<R>>,
}

impl Aggregation {
    pub fn new(aggregator: Aggregator) -> Self {
        Aggregation {
            aggregator,
            expressions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Aggregator {
    All,
    Any,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier<R = entity::Reference> {
    pub target: R,
}

impl Identifier {
    pub fn new(target: entity::Reference) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryOp<R = entity::Reference> {
    pub operator: UnaryOperator,
    pub operand: Box<Expression<R>>,
}

impl UnaryOp {
    pub fn new(operator: UnaryOperator, operand: Expression) -> Self {
        UnaryOp {
            operator,
            operand: Box::new(operand),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Plus,
    Negation,
    Not,
    Factorial,
    Previously,
    Rise,
    Fall,
    Eventually,
    Always,
}

impl UnaryOperator {
    pub fn precedence(&self) -> i32 {
        match self {
            UnaryOperator::Not => 40,
            UnaryOperator::Previously
            | UnaryOperator::Rise
            | UnaryOperator::Fall
            | UnaryOperator::Eventually
            | UnaryOperator::Always => 50,
            UnaryOperator::Negation | UnaryOperator::Plus => 140,
            UnaryOperator::Factorial => 150,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinOp<R = entity::Reference> {
    pub operator: BinaryOperator,
    pub left: Box<Expression<R>>,
    pub right: Box<Expression<R>>,
}

impl BinOp {
    pub fn new(operator: BinaryOperator, left: Expression, right: Expression) -> Self {
        BinOp {
            operator,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Associativity {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulus,
    Power,

    And,
    Or,
    Xor,
    In,
    Implies,
    Iff,

    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    NotEqual,
    Equal,

    Union,
    Intersection,
    Difference,
    Complement,
    Includes,

    Since,
}

impl BinaryOperator {
    pub fn precedence(&self) -> i32 {
        match self {
            BinaryOperator::Iff | BinaryOperator::Implies => 10,
            BinaryOperator::Or | BinaryOperator::Xor => 20,
            BinaryOperator::And => 30,
            BinaryOperator::Since => 60,
            BinaryOperator::Union | BinaryOperator::Difference => 70,
            BinaryOperator::Intersection => 71,
            BinaryOperator::Complement => 72,
            BinaryOperator::In | BinaryOperator::Includes => 80,
            BinaryOperator::Equal | BinaryOperator::NotEqual => 90,
            BinaryOperator::LessThan
            | BinaryOperator::GreaterThan
            | BinaryOperator::LessOrEqual
            | BinaryOperator::GreaterOrEqual => 100,
            BinaryOperator::Plus | BinaryOperator::Minus => 110,
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulus => 120,
            BinaryOperator::Power => 130,
        }
    }

    pub fn associativity(&self) -> Associativity {
        match self {
            BinaryOperator::Power => Associativity::Right,
            _ => Associativity::Left,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Function<R = entity::Reference> {
    pub expression: Box<Expression<R>>,
    pub arguments: Vec<Expression<R>>,
}

impl Function {
    pub fn new(expression: Expression) -> Self {
        Function {
            expression: Box::new(expression),
            arguments: Vec::new(),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Set<R = entity::Reference> {
    pub elements: Vec<Expression<R>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Number {
    pub kind: NumberKind,
    pub value: String,
}

impl Number {
    pub fn new(kind: NumberKind) -> Self {
        Number {
            kind,
            value: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NumberKind {
    Integer,
    Real,
    Infinity,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boolean {
    pub value: bool,
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
