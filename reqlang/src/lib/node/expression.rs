use crate::{node::arena, utils::children::ChildrenIter};

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
pub type Arena<T> = arena::Arena<T, IdxMarker>;
pub type Selector<'a, T> = arena::selector::Selector<'a, T, IdxMarker>;

#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Node<P, R, I = Idx> {
    pub payload: P,
    pub variant: Variant<I, R>,
}

impl<P, R, I> Node<P, R, I> {
    pub fn empty(variant: Variant<I, R>) -> Self
    where
        P: Default,
    {
        Self {
            payload: Default::default(),
            variant,
        }
    }

    pub fn new(payload: P, variant: Variant<I, R>) -> Self {
        Self { payload, variant }
    }

    pub fn map<F, Q>(self, f: F) -> Node<Q, R, I>
    where
        F: FnOnce(P) -> Q,
    {
        Node {
            payload: f(self.payload),
            variant: self.variant,
        }
    }

    pub fn mapr<F, S>(self, f: F) -> Node<P, S, I>
    where
        F: FnMut(R) -> S,
    {
        Node {
            payload: self.payload,
            variant: self.variant.mapr(f),
        }
    }
}

impl<N, R, I: Clone> ChildrenIter<I> for Node<N, R, I> {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = I> {
        self.variant.children_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Variant<I, R> {
    When(When<I>),
    Aggregation(Aggregation<I>),
    Comprehension(Comprehension<I>),

    UnaryOp(UnaryOp<I>),
    BinOp(BinOp<I>),
    Function(Function<I>),

    Identifier(Identifier<R>),
    Number(Number),
    Boolean(Boolean),
    Undefined,

    Set(Set<I>),
    Interval(Interval<I>),
    Range(Range<I>),
}

pub trait IntoVariable<N, R> {
    fn into_variant(self) -> Variant<N, R>;
}

impl<I: Clone, R> ChildrenIter<I> for Variant<I, R> {
    fn children_iter(&self) -> impl DoubleEndedIterator<Item = I> {
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

impl<N, R> Default for Variant<N, R> {
    fn default() -> Self {
        Variant::Undefined
    }
}

impl<N, R> Variant<N, R> {
    pub fn mapr<F, S>(self, mut f: F) -> Variant<N, S>
    where
        F: FnMut(R) -> S,
    {
        match self {
            Variant::When(when) => Variant::When(when),
            Variant::Aggregation(aggregation) => Variant::Aggregation(aggregation),
            Variant::Comprehension(comprehension) => Variant::Comprehension(comprehension),
            Variant::UnaryOp(unary_op) => Variant::UnaryOp(unary_op),
            Variant::BinOp(bin_op) => Variant::BinOp(bin_op),
            Variant::Function(function) => Variant::Function(function),
            Variant::Identifier(identifier) => {
                Variant::Identifier(Identifier::new(f(identifier.target)))
            }
            Variant::Number(number) => Variant::Number(number),
            Variant::Boolean(boolean) => Variant::Boolean(boolean),
            Variant::Undefined => Variant::Undefined,
            Variant::Set(set) => Variant::Set(set),
            Variant::Interval(interval) => Variant::Interval(interval),
            Variant::Range(range) => Variant::Range(range),
        }
    }

    pub fn mape<F, O>(self, mut f: F) -> Variant<O, R>
    where
        F: FnMut(N) -> O,
    {
        match self {
            Variant::When(when) => Variant::When(When {
                cases: when.cases.into_iter().map(|c| c.map(&mut f)).collect(),
                otherwise: when.otherwise.map(&mut f),
            }),

            Variant::Aggregation(aggregation) => Variant::Aggregation(Aggregation {
                aggregator: aggregation.aggregator,
                expressions: aggregation.expressions.into_iter().map(&mut f).collect(),
            }),

            Variant::Comprehension(comprehension) => {
                Variant::Comprehension(comprehension.mape(&mut f))
            }

            Variant::UnaryOp(unary) => Variant::UnaryOp(UnaryOp {
                operator: unary.operator,
                operand: f(unary.operand),
            }),

            Variant::BinOp(binary) => Variant::BinOp(BinOp {
                operator: binary.operator,
                left: f(binary.left),
                right: f(binary.right),
            }),

            Variant::Function(function) => Variant::Function(Function {
                expression: f(function.expression),
                arguments: function.arguments.into_iter().map(&mut f).collect(),
            }),

            Variant::Set(set) => Variant::Set(Set {
                elements: set.elements.into_iter().map(&mut f).collect(),
            }),

            Variant::Interval(interval) => Variant::Interval(Interval {
                left: interval.left.mape(&mut f),
                right: interval.right.mape(&mut f),
            }),
            Variant::Range(range) => Variant::Range(Range {
                left: f(range.left),
                right: f(range.right),
            }),

            Variant::Identifier(identifier) => Variant::Identifier(Identifier {
                target: identifier.target,
            }),
            Variant::Number(number) => Variant::Number(number),
            Variant::Boolean(boolean) => Variant::Boolean(boolean),
            Variant::Undefined => Variant::Undefined,
        }
    }

    pub fn identifier(&self) -> Option<&Identifier<R>> {
        match self {
            Variant::Identifier(id) => Some(id),
            _ => None,
        }
    }

    pub fn identifier_mut(&mut self) -> Option<&mut Identifier<R>> {
        match self {
            Variant::Identifier(id) => Some(id),
            _ => None,
        }
    }

    pub fn set_element(&self) -> Option<&SetElement<N>> {
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
            Variant::Function(_) => 160,
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct When<N> {
    pub cases: Vec<Case<N>>,
    pub otherwise: Option<N>,
}

impl<N> When<N> {
    pub fn new(cases: Vec<Case<N>>) -> Self {
        When {
            cases,
            otherwise: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Case<N> {
    pub condition: N,
    pub consequence: N,
}

impl<N> Case<N> {
    pub fn new(condition: N, consequence: N) -> Self {
        Case {
            condition,
            consequence,
        }
    }

    pub fn map<F, O>(self, mut f: F) -> Case<O>
    where
        F: FnMut(N) -> O,
    {
        Case {
            condition: f(self.condition),
            consequence: f(self.consequence),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SetElement<N> {
    pub variable: String,
    pub domain: N,
    pub filter: Option<N>,
}

impl<N> SetElement<N> {
    pub fn new(variable: String, domain: N) -> Self {
        SetElement {
            variable,
            domain,
            filter: None,
        }
    }

    pub fn mape<F, M>(self, mut f: F) -> SetElement<M>
    where
        F: FnMut(N) -> M,
    {
        SetElement {
            variable: self.variable,
            domain: f(self.domain),
            filter: self.filter.map(&mut f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Aggregation<N> {
    pub aggregator: Aggregator,
    pub expressions: Vec<N>,
}

impl<N> Aggregation<N> {
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
pub struct Comprehension<N> {
    pub aggregator: Option<Aggregator>,
    pub set: SetElement<N>,
    pub expression: N,
}

impl<N> Comprehension<N> {
    pub fn new(aggregator: Option<Aggregator>, set: SetElement<N>, expression: N) -> Self {
        Comprehension {
            aggregator,
            set,
            expression,
        }
    }

    pub fn mape<F, M>(self, mut f: F) -> Comprehension<M>
    where
        F: FnMut(N) -> M,
    {
        Comprehension {
            aggregator: self.aggregator,
            set: self.set.mape(&mut f),
            expression: f(self.expression),
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
pub struct Identifier<R> {
    pub target: R,
}

impl<R> Identifier<R> {
    pub fn new(target: R) -> Self {
        Self { target }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryOp<N> {
    pub operator: UnaryOperator,
    pub operand: N,
}

impl<N> UnaryOp<N> {
    pub fn new(operator: UnaryOperator, operand: N) -> Self {
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
pub struct BinOp<N> {
    pub operator: BinaryOperator,
    pub left: N,
    pub right: N,
}

impl<N> BinOp<N> {
    pub fn new(operator: BinaryOperator, left: N, right: N) -> Self {
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
pub struct Function<N> {
    pub expression: N,
    pub arguments: Vec<N>,
}

impl<N> Function<N> {
    pub fn new(expression: N) -> Self {
        Function {
            expression,
            arguments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Set<N> {
    pub elements: Vec<N>,
}

impl<N> Default for Set<N> {
    fn default() -> Self {
        Self {
            elements: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Interval<N> {
    pub left: IntervalBound<N>,
    pub right: IntervalBound<N>,
}

impl<N> Interval<N> {
    pub fn new(left: IntervalBound<N>, right: IntervalBound<N>) -> Self {
        Self { left, right }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntervalBound<N> {
    pub expression: N,
    pub is_open: bool,
}

impl<N> IntervalBound<N> {
    pub fn open(expression: N) -> Self {
        Self {
            expression,
            is_open: true,
        }
    }

    pub fn closed(expression: N) -> Self {
        Self {
            expression,
            is_open: false,
        }
    }

    pub fn mape<E>(self, f: impl FnOnce(N) -> E) -> IntervalBound<E> {
        IntervalBound {
            expression: f(self.expression),
            is_open: self.is_open,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Range<N> {
    pub left: N,
    pub right: N,
}

impl<N> Range<N> {
    pub fn new(left: N, right: N) -> Self {
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
