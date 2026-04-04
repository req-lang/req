use crate::utils::children::ChildrenIter;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Variant<N, R> {
    Branch(Branch<N>),
    When(When<N>),
    Forall(Forall<N>),
    Exists(Exists<N>),
    Select(Select<N>),
    Aggregation(Aggregation<N>),

    UnaryOp(UnaryOp<N>),
    BinOp(BinOp<N>),
    Function(Function<N>),

    Identifier(Identifier<R>),
    Number(Number),
    Boolean(Boolean),
    Undefined,

    Set(Set<N>),
}

pub trait IntoVariable<N, R> {
    fn into_variant(self) -> Variant<N, R>;
}

impl<N: Clone, R> ChildrenIter<N> for Variant<N, R> {
    fn children_iter(&self) -> impl Iterator<Item = N> {
        use Variant::*;

        let mut children = Vec::new();
        match self {
            Branch(branch) => {
                let bo = &branch.otherwise;
                children.push(&branch.case.condition);
                children.push(&branch.case.consequence);
                children.extend(bo.iter());
            }

            When(when) => {
                let cases = when
                    .cases
                    .iter()
                    .map(|c| [&c.condition, &c.consequence])
                    .flatten();

                children.extend(cases);
                children.extend(when.otherwise.iter());
            }

            Forall(forall) => {
                let fs = &forall.set;
                children.push(&fs.domain);
                children.extend(fs.filter.iter());
                children.push(&forall.expression);
            }

            Exists(exists) => {
                let es = &exists.set;
                children.push(&es.domain);
                children.extend(es.filter.iter());
            }

            Select(select) => {
                let ss = &select.set;
                let so = &select.optimizer;
                children.push(&ss.domain);
                children.extend(ss.filter.iter());
                children.extend(so.iter().map(|n| &n.expression));
            }

            Aggregation(aggregation) => {
                let ae = &aggregation.expressions;
                children.extend(ae.iter());
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
            Variant::Branch(branch) => Variant::Branch(branch),
            Variant::When(when) => Variant::When(when),
            Variant::Forall(forall) => Variant::Forall(forall),
            Variant::Exists(exists) => Variant::Exists(exists),
            Variant::Select(select) => Variant::Select(select),
            Variant::Aggregation(aggregation) => Variant::Aggregation(aggregation),
            Variant::UnaryOp(unary_op) => Variant::UnaryOp(unary_op),
            Variant::BinOp(bin_op) => Variant::BinOp(bin_op),
            Variant::Function(function) => Variant::Function(function),
            Variant::Identifier(identifier) => Variant::Identifier(Identifier {
                target: f(identifier.target),
            }),
            Variant::Number(number) => Variant::Number(number),
            Variant::Boolean(boolean) => Variant::Boolean(boolean),
            Variant::Undefined => Variant::Undefined,
            Variant::Set(set) => Variant::Set(set),
        }
    }

    pub fn mape<F, O>(self, mut f: F) -> Variant<O, R>
    where
        F: FnMut(N) -> O,
    {
        match self {
            Variant::Branch(branch) => Variant::Branch(Branch {
                case: branch.case.map(&mut f),
                otherwise: branch.otherwise.map(&mut f),
            }),

            Variant::When(when) => Variant::When(When {
                cases: when.cases.into_iter().map(|c| c.map(&mut f)).collect(),
                otherwise: when.otherwise.map(&mut f),
            }),

            Variant::Forall(forall) => Variant::Forall(Forall {
                set: SetElement {
                    variable: forall.set.variable,
                    domain: f(forall.set.domain),
                    filter: forall.set.filter.map(&mut f),
                },
                expression: f(forall.expression),
            }),

            Variant::Exists(exists) => Variant::Exists(Exists {
                set: SetElement {
                    variable: exists.set.variable,
                    domain: f(exists.set.domain),
                    filter: exists.set.filter.map(&mut f),
                },
            }),

            Variant::Select(select) => Variant::Select(Select {
                set: SetElement {
                    variable: select.set.variable,
                    domain: f(select.set.domain),
                    filter: select.set.filter.map(&mut f),
                },
                optimizer: select.optimizer.map(|o| Optimizer {
                    kind: o.kind,
                    expression: f(o.expression),
                }),
            }),

            Variant::Aggregation(aggregation) => Variant::Aggregation(Aggregation {
                aggregator: aggregation.aggregator,
                expressions: aggregation.expressions.into_iter().map(&mut f).collect(),
            }),

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

            Variant::Identifier(identifier) => Variant::Identifier(Identifier {
                target: identifier.target,
            }),
            Variant::Number(number) => Variant::Number(number),
            Variant::Boolean(boolean) => Variant::Boolean(boolean),
            Variant::Undefined => Variant::Undefined,
        }
    }

    pub fn set_element(&self) -> Option<&SetElement<N>> {
        use Variant::*;

        match self {
            Forall(forall) => Some(&forall.set),
            Exists(exists) => Some(&exists.set),
            Select(select) => Some(&select.set),
            _ => None,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Branch<N> {
    pub case: Case<N>,
    pub otherwise: Option<N>,
}

impl<N> Branch<N> {
    pub fn new(case: Case<N>) -> Self {
        Branch {
            case,
            otherwise: None,
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
pub struct Forall<N> {
    pub set: SetElement<N>,
    pub expression: N,
}

impl<N> Forall<N> {
    pub fn new(set: SetElement<N>, expression: N) -> Self {
        Forall { set, expression }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Exists<N> {
    pub set: SetElement<N>,
}

impl<N> Exists<N> {
    pub fn new(set: SetElement<N>) -> Self {
        Exists { set }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Select<N> {
    pub set: SetElement<N>,
    pub optimizer: Option<Optimizer<N>>,
}

impl<N> Select<N> {
    pub fn new(set: SetElement<N>) -> Self {
        Select {
            set,
            optimizer: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Optimizer<N> {
    pub kind: OptimizerKind,
    pub expression: N,
}

impl<N> Optimizer<N> {
    pub fn new(kind: OptimizerKind, expression: N) -> Self {
        Optimizer { kind, expression }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OptimizerKind {
    Minimize,
    Maximize,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Aggregator {
    All,
    Any,
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
