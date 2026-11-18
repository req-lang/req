use crate::{
    node::{arena, identifier},
    utils::children::{ChildrenIter, ChildrenIterMut},
};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
pub type Arena<T> = arena::Arena<T, IdxMarker>;

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Node {
    pub variant: Variant,
}

impl Node {
    pub fn new(variant: Variant) -> Self {
        Self { variant }
    }
}

impl ChildrenIter<Idx> for Node {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Idx> {
        self.variant.children_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Variant {
    When(When),
    Aggregation(Aggregation),
    Comprehension(Comprehension),

    UnaryOp(UnaryOp),
    BinOp(BinOp),
    Function(Function),

    Identifier(Identifier),
    Number(Number),
    Boolean(Boolean),
    Undefined,

    Set(Set),
    Interval(Interval),
    Range(Range),
}

impl ChildrenIter<Idx> for Variant {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = Idx> {
        use Variant::*;

        let mut children = Vec::new();
        match self {
            When(when) => {
                let cases = when
                    .cases
                    .iter()
                    .map(|c| [&c.condition, &c.consequence])
                    .flatten();

                children.extend(cases);
                children.extend(when.otherwise.iter());
            }

            Aggregation(aggregation) => {
                let ae = &aggregation.expressions;
                children.extend(ae.iter());
            }

            Comprehension(comprehension) => {
                let fs = &comprehension.set;
                children.push(&fs.domain);
                children.extend(fs.filter.iter());
                children.push(&comprehension.expression);
            }

            UnaryOp(unary) => {
                children.push(&unary.operand);
            }

            BinOp(binary) => {
                children.push(&binary.left);
                children.push(&binary.right);
            }

            Function(function) => {
                let fa = &function.arguments;
                children.push(&function.expression);
                children.extend(fa.iter());
            }

            Set(set) => {
                let se = &set.elements;
                children.extend(se.iter());
            }

            Interval(interval) => {
                children.push(&interval.left.expression);
                children.push(&interval.right.expression);
            }

            Range(range) => {
                children.push(&range.left);
                children.push(&range.right);
            }
            Identifier(_) => {}
            Number(_) => {}
            Boolean(_) => {}
            Undefined => {}
        };

        children.into_iter().cloned()
    }
}

impl ChildrenIterMut<Idx> for Variant {
    fn children_iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Idx>
    where
        Idx: 'a,
    {
        use Variant::*;

        let mut children = Vec::new();
        match self {
            When(when) => {
                for case in &mut when.cases {
                    children.push(&mut case.condition);
                    children.push(&mut case.consequence);
                }
                children.extend(when.otherwise.iter_mut());
            }

            Aggregation(aggregation) => children.extend(aggregation.expressions.iter_mut()),

            Comprehension(comprehension) => {
                children.push(&mut comprehension.set.domain);
                children.extend(comprehension.set.filter.iter_mut());
                children.push(&mut comprehension.expression);
            }

            UnaryOp(unary) => children.push(&mut unary.operand),

            BinOp(binary) => {
                children.push(&mut binary.left);
                children.push(&mut binary.right);
            }

            Function(function) => {
                children.push(&mut function.expression);
                children.extend(function.arguments.iter_mut());
            }

            Set(set) => children.extend(set.elements.iter_mut()),

            Interval(interval) => {
                children.push(&mut interval.left.expression);
                children.push(&mut interval.right.expression);
            }

            Range(range) => {
                children.push(&mut range.left);
                children.push(&mut range.right);
            }

            Identifier(_) | Number(_) | Boolean(_) | Undefined => {}
        };

        children.into_iter()
    }
}

impl ChildrenIterMut<Idx> for Node {
    fn children_iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Idx>
    where
        Idx: 'a,
    {
        self.variant.children_iter_mut()
    }
}

impl Default for Variant {
    fn default() -> Self {
        Variant::Undefined
    }
}

impl Variant {
    pub fn identifier(&self) -> Option<&Identifier> {
        match self {
            Variant::Identifier(id) => Some(id),
            _ => None,
        }
    }

    pub fn set_element(&self) -> Option<&SetElement> {
        use Variant::*;

        match self {
            Comprehension(comprehension) => Some(&comprehension.set),
            _ => None,
        }
    }

    pub fn needs_end(&self) -> bool {
        use Variant::*;

        match self {
            When(_) | Aggregation(_) => true,
            // `{ e for x in S }` is closed by its brace, `sum e for x in S end` by `end`
            Comprehension(comprehension) => comprehension.aggregator.is_some(),
            _ => false,
        }
    }

    pub fn needs_parentheses(&self, parent: &Self, is_right: bool) -> bool {
        let parent_prec = parent.precedence();
        let child_prec = self.precedence();

        if child_prec < parent_prec {
            return true;
        }

        if child_prec > parent_prec {
            return false;
        }

        match (parent.associativity(), self.associativity()) {
            (Some(parent), Some(_)) => match parent {
                Associativity::Right => !is_right,
                Associativity::Left => is_right,
            },
            _ => false,
        }
    }

    pub fn precedence(&self) -> i32 {
        match self {
            Variant::BinOp(bin_op) => bin_op.operator.precedence(),
            Variant::UnaryOp(unary_op) => unary_op.operator.precedence(),
            Variant::Function(_) => FUNCTION_PRECEDENCE,
            _ => 1000,
        }
    }

    pub fn associativity(&self) -> Option<Associativity> {
        match self {
            Variant::BinOp(bin_op) => Some(bin_op.operator.associativity()),
            _ => None,
        }
    }
}

pub const FUNCTION_PRECEDENCE: i32 = 160;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct When {
    pub cases: Vec<Case>,
    pub otherwise: Option<Idx>,
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
pub struct Case {
    pub condition: Idx,
    pub consequence: Idx,
}

impl Case {
    pub fn new(condition: Idx, consequence: Idx) -> Self {
        Case {
            condition,
            consequence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetElement {
    pub variable: String,
    pub domain: Idx,
    pub filter: Option<Idx>,
}

impl SetElement {
    pub fn new(variable: String, domain: Idx) -> Self {
        SetElement {
            variable,
            domain,
            filter: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Aggregation {
    pub aggregator: Aggregator,
    pub expressions: Vec<Idx>,
}

impl Aggregation {
    pub fn new(aggregator: Aggregator) -> Self {
        Aggregation {
            aggregator,
            expressions: Vec::new(),
        }
    }
}

/// `{ expression for variable in domain such that filter }`, or its aggregation
/// `aggregator expression for variable in domain such that filter end` when an
/// aggregator is given. The aggregator is applied to the family indexed by the variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Comprehension {
    pub aggregator: Option<Aggregator>,
    pub set: SetElement,
    pub expression: Idx,
}

impl Comprehension {
    pub fn new(aggregator: Option<Aggregator>, set: SetElement, expression: Idx) -> Self {
        Comprehension {
            aggregator,
            set,
            expression,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Aggregator {
    All,
    Any,
    One,
    Maximum,
    Minimum,
    Sum,
    Product,
    Union,
    Intersection,
    Choose,
    ArgMaximum,
    ArgMinimum,
    Supremum,
    Infimum,
    Count,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub target: identifier::Idx,
}

impl Identifier {
    pub fn new(target: identifier::Idx) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryOp {
    pub operator: UnaryOperator,
    pub operand: Idx,
}

impl UnaryOp {
    pub fn new(operator: UnaryOperator, operand: Idx) -> Self {
        UnaryOp { operator, operand }
    }

    pub fn is_postfix(&self) -> bool {
        match self.operator {
            UnaryOperator::Factorial => true,
            _ => false,
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
pub struct BinOp {
    pub operator: BinaryOperator,
    pub left: Idx,
    pub right: Idx,
}

impl BinOp {
    pub fn new(operator: BinaryOperator, left: Idx, right: Idx) -> Self {
        BinOp {
            operator,
            left,
            right,
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
    Implies,
    Iff,

    In,
    Tolerance,

    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    NotEqual,
    Equal,
    Fix,
    Initially,

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
            BinaryOperator::Tolerance => 85,
            BinaryOperator::Fix
            | BinaryOperator::Initially
            | BinaryOperator::Equal
            | BinaryOperator::NotEqual => 90,
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
pub struct Function {
    pub expression: Idx,
    pub arguments: Vec<Idx>,
}

impl Function {
    pub fn new(expression: Idx) -> Self {
        Function {
            expression,
            arguments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Set {
    pub elements: Vec<Idx>,
}

impl Default for Set {
    fn default() -> Self {
        Self {
            elements: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Interval {
    pub left: IntervalBound,
    pub right: IntervalBound,
}

impl Interval {
    pub fn new(left: IntervalBound, right: IntervalBound) -> Self {
        Self { left, right }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntervalBound {
    pub expression: Idx,
    pub is_open: bool,
}

impl IntervalBound {
    pub fn open(expression: Idx) -> Self {
        Self {
            expression,
            is_open: true,
        }
    }

    pub fn closed(expression: Idx) -> Self {
        Self {
            expression,
            is_open: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Range {
    pub left: Idx,
    pub right: Idx,
}

impl Range {
    pub fn new(left: Idx, right: Idx) -> Self {
        Self { left, right }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Boolean {
    pub value: bool,
}

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        Boolean { value }
    }
}
