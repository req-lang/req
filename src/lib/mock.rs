pub mod entity {
    use crate::{
        mock::expression,
        node::{
            self, NodeRef,
            entity::{Attribute, Package, Part, Requirement, RequirementVariant, Variant},
        },
        syntax::{self, entity::Node},
    };

    pub fn package(label: &str) -> NodeRef<Node> {
        let mut variant = Variant::Package(Package::new(vec![], vec![]));
        variant.meta_mut().label = label.to_string();
        node::new(Node::new(variant))
    }

    pub fn part(label: &str) -> NodeRef<Node> {
        let mut variant = Variant::Part(Part::new(vec![], vec![]));
        variant.meta_mut().label = label.to_string();
        node::new(Node::new(variant))
    }

    pub fn attribute(label: &str, domain: &str) -> NodeRef<Node> {
        let domain = expression::identifier(domain);
        let mut variant = Variant::Attribute(Attribute::new(domain));
        variant.meta_mut().label = label.to_string();
        node::new(Node::new(variant))
    }

    pub fn requirement(label: &str) -> NodeRef<Node> {
        let variant = RequirementVariant::default();
        let mut variant = Variant::Requirement(Requirement::from(variant));
        variant.meta_mut().label = label.to_string();
        node::new(Node::new(variant))
    }

    pub fn formal(label: &str, expr: NodeRef<syntax::expression::Node>) -> NodeRef<Node> {
        let req = requirement(label);
        req.borrow_mut().variant.requirement_mut().unwrap().variant =
            RequirementVariant::Formal(expr);
        req
    }

    pub fn simple_tree() -> NodeRef<Node> {
        let mut arena = vec![
            package("a"),
            package("b"),
            package("c"),
            package("d"),
            package("e"),
        ];

        let variant = Variant::Package(Package::new(vec![], vec![arena[3].clone()]));
        arena[1].borrow_mut().variant = variant;

        let variant = Variant::Package(Package::new(vec![], vec![arena[4].clone()]));
        arena[2].borrow_mut().variant = variant;

        let variant = Variant::Package(Package::new(
            vec![],
            vec![arena[1].clone(), arena[2].clone()],
        ));
        arena[0].borrow_mut().variant = variant;
        arena.reverse();

        return arena.pop().unwrap();
    }
}

pub mod generator {
    use std::rc::Rc;

    use super::entity::package;
    use crate::mock::entity::requirement;
    use crate::node::{NodeRef, markup};
    use crate::syntax::entity::Node;
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
        fn generate(&self) -> NodeRef<Node>;

        fn size(&self) -> u64;
    }

    // Count of nodes: packages * depth * (requirement + 1)
    pub struct Simple {
        pub packages: u64,
        pub depth: u64,
        pub requirements: u64,
        pub words: u64,
    }

    impl Generate for Simple {
        fn generate(&self) -> NodeRef<Node> {
            let root = package("root");

            for p in 0..self.packages {
                let pkg = package(format!("package_{p}").as_str());
                root.borrow_mut()
                    .variant
                    .children_mut()
                    .unwrap()
                    .push(Rc::clone(&pkg));

                let mut prev = pkg;
                for d in 0..self.depth {
                    let curr = package(format!("package_{p}_{d}").as_str());
                    for r in 0..self.requirements {
                        let req = requirement(format!("requirement_{p}_{d}_{r}").as_str());
                        let content = vec!["word"]
                            .into_iter()
                            .cycle()
                            .take(self.words as usize)
                            .collect::<Vec<&str>>()
                            .join(" ");

                        let mut markup = markup::Node::default();
                        markup.items.push(markup::Item::Text(content));

                        *req.borrow_mut()
                            .variant
                            .requirement_mut()
                            .unwrap()
                            .informal_mut()
                            .unwrap() = markup;

                        curr.borrow_mut().variant.children_mut().unwrap().push(req);
                    }

                    prev.borrow_mut()
                        .variant
                        .children_mut()
                        .unwrap()
                        .push(Rc::clone(&curr));
                    prev = curr;
                }
            }

            root
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
}

pub mod expression {
    use crate::{
        node::{
            self, NodeRef,
            expression::{Identifier, Variant},
        },
        reference::Reference,
        syntax::expression::Node,
    };

    pub fn identifier(location: &str) -> NodeRef<Node> {
        let reference = Reference::new(location.to_string());
        let variant = Variant::Identifier(Identifier::new(reference));
        node::new(Node::new(variant))
    }

    pub fn number(n: i64) -> NodeRef<Node> {
        let num = node::expression::Number {
            kind: node::expression::NumberKind::Integer,
            value: n.to_string(),
        };
        node::new(Node::new(node::expression::Variant::Number(num)))
    }

    pub fn boolean(b: bool) -> NodeRef<Node> {
        node::new(Node::new(node::expression::Variant::Boolean(
            node::expression::Boolean { value: b },
        )))
    }

    pub fn binary(
        op: node::expression::BinaryOperator,
        left: NodeRef<Node>,
        right: NodeRef<Node>,
    ) -> NodeRef<Node> {
        node::new(Node::new(node::expression::Variant::BinOp(
            node::expression::BinOp::new(op, left, right),
        )))
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
