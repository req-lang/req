pub mod entity {
    use crate::{
        mock::expression,
        node::entity::{Attribute, Meta, Package, Part, Requirement, RequirementVariant, Variant},
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
        let variant = Variant::Attribute(Attribute::new(vec![domain]));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn requirement<S: ToString>(label: S) -> Node {
        let variant = RequirementVariant::default();
        let variant = Variant::Requirement(Requirement::from(variant));
        Node::empty(Meta::from(label.to_string()), variant)
    }

    pub fn formal(label: &str, expr: Vec<syntax::expression::Node>) -> Node {
        let mut req = requirement(label);
        req.variant.requirement_mut().unwrap().variant = RequirementVariant::Formal(expr);
        req
    }

    pub fn simple_tree() -> Vec<Node> {
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

        return arena;
    }
}

pub mod generator {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::entity::package;
    use crate::mock::entity::requirement;
    use crate::node;
    use crate::syntax::entity::Node;
    use crate::syntax::markup;

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
        fn generate(&self) -> Vec<Node>;

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
        fn generate(&self) -> Vec<Node> {
            let arena = {
                let root = Rc::new(RefCell::new(package("root")));
                let mut arena = vec![root.clone()];

                for p in 0..self.packages {
                    let pkg = Rc::new(RefCell::new(package(format!("package_{p}"))));
                    arena.push(pkg.clone());
                    push_child(&root, arena.len());

                    let mut prev = pkg;
                    for d in 0..self.depth {
                        let curr = Rc::new(RefCell::new(package(format!("package_{p}_{d}"))));
                        arena.push(curr.clone());
                        let len = arena.len();
                        for r in 0..self.requirements {
                            let mut req = requirement(format!("requirement_{p}_{d}_{r}"));
                            let content = repeat_words(self.words as usize);
                            let mut markup = node::markup::Container::default();
                            markup.items.push(node::markup::Item::Text(content));

                            let informal = req.variant.requirement_mut().unwrap().informal_mut();
                            *informal.unwrap() = markup::Node::empty(markup);

                            arena.push(Rc::new(RefCell::new(req)));
                            push_child(&curr, arena.len());
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
                .collect()
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
            self,
            expression::{Identifier, Variant},
        },
        reference::Reference,
        syntax::expression::Node,
    };

    pub fn identifier<S: ToString>(location: S) -> Node {
        let reference = Reference::new(location.to_string());
        let variant = Variant::Identifier(Identifier::new(reference));
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
