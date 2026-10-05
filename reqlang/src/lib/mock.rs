pub mod generator {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::entity::{package, part, requirement};
    use crate::node;
    use crate::node::entity::Arena;
    use crate::syntax::entity::Node;
    use crate::syntax::markup;
    use crate::utils::children::ChildrenMut;

    pub struct ExponentialSizeIterator {
        value: f64,
    }

    impl ExponentialSizeIterator {
        pub fn new() -> Self {
            Self { value: 1.0 }
        }
    }

    impl Iterator for ExponentialSizeIterator {
        type Item = u64;

        fn next(&mut self) -> Option<Self::Item> {
            let ret = self.value;
            self.value *= (10.0 as f64).sqrt();
            Some(ret as u64)
        }
    }

    pub trait Generate {
        fn generate(&self) -> Arena<Node>;

        fn size(&self) -> u64;
    }

    // Count of nodes: packages * depth * (requirement + 1)
    pub struct Simple {
        pub packages: u64,
        pub depth: u64,
        pub requirements: u64,
        pub words: u64,
    }

    fn repeat_words(count: usize) -> String {
        vec!["word"]
            .into_iter()
            .cycle()
            .take(count)
            .collect::<Vec<&str>>()
            .join(" ")
    }

    fn push_child(nd: &Rc<RefCell<Node>>, child: usize) {
        nd.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(child.into());
    }

    impl Generate for Simple {
        fn generate(&self) -> Arena<Node> {
            let arena = {
                let root = Rc::new(RefCell::new(package("root")));
                let mut arena = vec![root.clone()];

                for p in 0..self.packages {
                    let pkg = Rc::new(RefCell::new(package(format!("package_{p}"))));
                    arena.push(pkg.clone());
                    push_child(&root, arena.len() - 1);

                    let mut prev = pkg;
                    for d in 0..self.depth {
                        let curr = Rc::new(RefCell::new(package(format!("package_{p}_{d}"))));
                        arena.push(curr.clone());
                        let len = arena.len() - 1;
                        for r in 0..self.requirements {
                            let mut req = requirement(format!("requirement_{p}_{d}_{r}"));
                            let content = repeat_words(self.words as usize);
                            let mut markup = node::markup::Container::default();
                            markup.items.push(node::markup::Item::Text(content));

                            let informal = req.variant.requirement_mut().unwrap().informal_mut();
                            *informal.unwrap() = markup::Node::empty(markup);

                            arena.push(Rc::new(RefCell::new(req)));
                            push_child(&curr, arena.len() - 1);
                        }

                        push_child(&prev, len);
                        prev = curr;
                    }
                }
                arena
            };
            arena
                .into_iter()
                .map(|n| RefCell::into_inner(Rc::try_unwrap(n).unwrap()))
                .collect::<Vec<_>>()
                .into()
        }

        fn size(&self) -> u64 {
            self.packages * (self.depth * (self.requirements + 1) + 1) + 1
        }
    }

    impl Simple {
        pub fn new() -> Self {
            Self {
                packages: 10,
                depth: 5,
                requirements: 5,
                words: 200,
            }
        }
    }

    /// Flat tree: one root package, two parts ("my::part", "my::other_part"),
    /// and N informal requirements each linking to one part and restricting to the other.
    /// Intended for benchmarking reference resolution under increasing requirement counts.
    pub struct ReferenceResolution {
        pub requirements: u64,
    }

    impl ReferenceResolution {
        pub fn new(requirements: u64) -> Self {
            Self { requirements }
        }
    }

    impl Generate for ReferenceResolution {
        fn generate(&self) -> Arena<Node> {
            let arena = {
                let root = Rc::new(RefCell::new(package("root")));
                let mut arena: Vec<Rc<RefCell<Node>>> = vec![
                    root.clone(),
                    Rc::new(RefCell::new(part("part"))), // 1
                    Rc::new(RefCell::new(part("other_part"))), // 2
                ];

                push_child(&root, 1);
                push_child(&root, 2);

                for i in 0..self.requirements {
                    let mut req = requirement(format!("req_{i}"));
                    req.variant.requirement_mut().unwrap().restriction =
                        Some("root::other_part".to_string());
                    let informal = req
                        .variant
                        .requirement_mut()
                        .unwrap()
                        .informal_mut()
                        .unwrap();
                    let mut container = node::markup::Container::default();
                    container
                        .items
                        .push(node::markup::Item::Reference("root::part".to_string()));
                    container.items.push(node::markup::Item::Reference(
                        "root::not_existing".to_string(),
                    ));
                    *informal = markup::Node::empty(container);

                    let idx = arena.len();
                    arena.push(Rc::new(RefCell::new(req)));
                    push_child(&root, idx);
                }
                arena
            };

            arena
                .into_iter()
                .map(|n| RefCell::into_inner(Rc::try_unwrap(n).unwrap()))
                .collect::<Vec<_>>()
                .into()
        }

        fn size(&self) -> u64 {
            3 + self.requirements
        }
    }

    /// Flat tree: root package containing one part "attrs" (with attr_a: real, attr_b: real,
    /// attr_bool: boolean) and N*3 formal requirements cycling through:
    ///   - addition:  attr_a + attr_b
    ///   - forall:    forall x in real, x > 0
    ///   - when:      when attr_bool then attr_a + 1
    pub struct ExpressionTyping {
        pub cycles: u64,
    }

    impl ExpressionTyping {
        pub fn new(cycles: u64) -> Self {
            Self { cycles }
        }
    }

    impl Generate for ExpressionTyping {
        fn generate(&self) -> Arena<Node> {
            let arena: Vec<Rc<RefCell<Node>>> = {
                let root = Rc::new(RefCell::new(package("root")));
                let attrs = Rc::new(RefCell::new(part("attrs")));
                let mut arena = vec![
                    root.clone(),                                                      // 0
                    attrs.clone(),                                                     // 1
                    Rc::new(RefCell::new(super::entity::attribute("attr_a", "real"))), // 2
                    Rc::new(RefCell::new(super::entity::attribute("attr_b", "real"))), // 3
                    Rc::new(RefCell::new(super::entity::attribute(
                        "attr_bool",
                        "boolean",
                    ))), // 4
                ];
                push_child(&root, 1);
                push_child(&attrs, 2);
                push_child(&attrs, 3);
                push_child(&attrs, 4);

                for i in 0..self.cycles {
                    arena.push(Rc::new(RefCell::new(make_addition(i))));
                    push_child(&root, arena.len() - 1);
                    arena.push(Rc::new(RefCell::new(make_forall(i))));
                    push_child(&root, arena.len() - 1);
                    arena.push(Rc::new(RefCell::new(make_when(i))));
                    push_child(&root, arena.len() - 1);
                }
                arena
            };

            arena
                .into_iter()
                .map(|n| RefCell::into_inner(Rc::try_unwrap(n).unwrap()))
                .collect::<Vec<_>>()
                .into()
        }

        fn size(&self) -> u64 {
            5 + self.cycles * 3 // root + part + 3 attrs + N*3 reqs
        }
    }

    fn make_addition(i: u64) -> Node {
        use crate::node::expression::{BinOp, BinaryOperator, Variant};
        use crate::syntax::expression::Node as ExprNode;
        // 0: attr_a + attr_b
        // 1: identifier("root::attrs::attr_a")
        // 2: identifier("root::attrs::attr_b")
        let exprs = vec![
            ExprNode::empty(Variant::BinOp(BinOp::new(
                BinaryOperator::Plus,
                1.into(),
                2.into(),
            ))),
            super::expression::identifier("root::attrs::attr_a"),
            super::expression::identifier("root::attrs::attr_b"),
        ];
        super::entity::formal(&format!("addition_{i}"), exprs)
    }

    fn make_forall(i: u64) -> Node {
        use crate::node::expression::{BinOp, BinaryOperator, Forall, SetElement, Variant};
        use crate::syntax::expression::Node as ExprNode;
        // 0: forall x in real, x > 0
        // 1: identifier("real")
        // 2: x > 0
        // 3: identifier("x")
        // 4: number(0)
        let set = SetElement::new("x".to_string(), 1.into());
        let forall_node = Forall::new(set, 2.into());
        let exprs = vec![
            ExprNode::empty(Variant::Forall(forall_node)),
            super::expression::identifier("real"),
            ExprNode::empty(Variant::BinOp(BinOp::new(
                BinaryOperator::GreaterThan,
                3.into(),
                4.into(),
            ))),
            super::expression::identifier("x"),
            super::expression::number(0),
        ];
        super::entity::formal(&format!("forall_{i}"), exprs)
    }

    fn make_when(i: u64) -> Node {
        use crate::node::expression::{BinOp, BinaryOperator, Case, Variant, When};
        use crate::syntax::expression::Node as ExprNode;
        // 0: when attr_bool then attr_a + 1
        // 1: identifier("root::attrs::attr_bool")
        // 2: attr_a + 1
        // 3: identifier("root::attrs::attr_a")
        // 4: number(1)
        let case = Case {
            condition: 1.into(),
            consequence: 2.into(),
        };
        let when_node = When {
            cases: vec![case],
            otherwise: None,
        };
        let exprs = vec![
            ExprNode::empty(Variant::When(when_node)),
            super::expression::identifier("root::attrs::attr_bool"),
            ExprNode::empty(Variant::BinOp(BinOp::new(
                BinaryOperator::Plus,
                3.into(),
                4.into(),
            ))),
            super::expression::identifier("root::attrs::attr_a"),
            super::expression::number(1),
        ];
        super::entity::formal(&format!("when_{i}"), exprs)
    }
}

pub mod entity {
    use crate::{
        mock::expression,
        node::entity::{
            Arena, Attribute, Meta, Package, Part, Requirement, RequirementVariant, Type, Variant,
        },
        syntax::{self, entity::Node},
    };

    pub fn package<S: ToString>(label: S) -> Node {
        let variant = Variant::Package(Package::new(vec![], vec![]));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn part<S: ToString>(label: S) -> Node {
        let variant = Variant::Part(Part::new(vec![], vec![]));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn attribute<S: ToString>(label: S, domain: S) -> Node {
        let domain = expression::identifier(domain);
        let variant = Variant::Attribute(Attribute::new(vec![domain].into()));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn requirement<S: ToString>(label: S) -> Node {
        let variant = RequirementVariant::default();
        let variant = Variant::Requirement(Requirement::from(variant));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn formal(label: &str, expr: Vec<syntax::expression::Node>) -> Node {
        let mut req = requirement(label);
        req.variant.requirement_mut().unwrap().variant = RequirementVariant::Formal(expr.into());
        req
    }

    pub fn typ<S: ToString>(label: S, members: &[S]) -> Node {
        let variant = Variant::Type(Type::from(
            members
                .into_iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>(),
        ));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn simple_tree() -> Arena<Node> {
        let mut arena = vec![
            package("a"),
            package("b"),
            package("c"),
            package("d"),
            package("e"),
        ];

        let variant = Variant::Package(Package::new(vec![], vec![3.into()]));
        arena[1].variant = variant;

        let variant = Variant::Package(Package::new(vec![], vec![4.into()]));
        arena[2].variant = variant;

        let variant = Variant::Package(Package::new(vec![], vec![1.into(), 2.into()]));
        arena[0].variant = variant;
        arena.reverse();

        return arena.into();
    }
}

pub mod expression {
    use crate::{
        node::{
            self,
            expression::{Identifier, Variant},
        },
        syntax::expression::Node,
    };

    pub fn identifier<S: ToString>(location: S) -> Node {
        let variant = Variant::Identifier(Identifier::new(location.to_string()));
        Node::empty(variant)
    }

    pub fn number(n: i64) -> Node {
        let num = node::expression::Number {
            kind: node::expression::NumberKind::Integer,
            value: n.to_string(),
        };
        Node::empty(Variant::Number(num))
    }

    pub fn boolean(b: bool) -> Node {
        Node::empty(Variant::Boolean(node::expression::Boolean::from(b)))
    }

    pub fn binary(op: node::expression::BinaryOperator, left: usize, right: usize) -> Node {
        Node::empty(node::expression::Variant::BinOp(
            node::expression::BinOp::new(op, left.into(), right.into()),
        ))
    }
}

//
// #[cfg(test)]
// mod tests {
//     use crate::mock::generator::{Generate, Simple};
//
//     #[test]
//     fn simple_has_expected_size() {
//         let mut generator = Simple::new();
//
//         let root = generator.generate();
//         assert_eq!(root.iter().count() as u64, generator.size());
//
//         generator.packages = 5;
//         let root = generator.generate();
//         assert_eq!(root.iter().count() as u64, generator.size());
//     }
// }
