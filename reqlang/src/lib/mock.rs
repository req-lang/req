pub mod generator {
    use super::entity::{
        add, add_child, add_parent, add_root, formal, informal, package, part, reference,
    };
    use super::expression::{binary, identifier, number};
    use crate::node::{
        expression::{Aggregator, BinaryOperator, Case, Comprehension, SetElement, Variant, When},
        markup::{Item, Markup},
        tree::Tree,
    };

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
        fn generate(&self) -> Tree;

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

    impl Generate for Simple {
        fn generate(&self) -> Tree {
            let mut draft = Tree::default();
            let root = add_root(&mut draft, package("root"));

            for p in 0..self.packages {
                let pkg = add_child(&mut draft, root, package(format!("package_{p}")));

                let mut prev = pkg;
                for d in 0..self.depth {
                    let curr = add_child(&mut draft, prev, package(format!("package_{p}_{d}")));
                    for r in 0..self.requirements {
                        let content = repeat_words(self.words as usize);
                        let text = Markup::from(vec![Item::Text(content)]);
                        let label = format!("requirement_{p}_{d}_{r}");
                        let req = informal(&mut draft, label, text);
                        add_child(&mut draft, curr, req);
                    }
                    prev = curr;
                }
            }

            draft.into()
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

    /// Flat draft: one root package, two parts ("my::part", "my::other_part"),
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
        fn generate(&self) -> Tree {
            let mut draft = Tree::default();
            let root = add_root(&mut draft, package("root"));
            add_child(&mut draft, root, part("part"));
            add_child(&mut draft, root, part("other_part"));

            for i in 0..self.requirements {
                let restriction = reference(&mut draft, "root::other_part");
                let part = reference(&mut draft, "root::part");
                let missing = reference(&mut draft, "root::not_existing");
                let text = Markup::from(vec![Item::Reference(part), Item::Reference(missing)]);
                let mut req = informal(&mut draft, format!("req_{i}"), text);
                req.variant.requirement_mut().unwrap().restriction = Some(restriction);
                add_child(&mut draft, root, req);
            }

            draft.into()
        }

        fn size(&self) -> u64 {
            3 + self.requirements
        }
    }

    /// Flat draft: one root package and `requirements` informal requirements. Each text is `lines`
    /// lines of `per_line` references, `words` words before each one, and a path has `segments`
    /// segments. The paths resolve to nothing: meant for tokenizing and parsing the references.
    pub struct ReferenceText {
        pub requirements: u64,
        pub lines: u64,
        pub per_line: u64,
        pub words: u64,
        pub segments: u64,
    }

    impl ReferenceText {
        pub fn new() -> Self {
            Self {
                requirements: 10,
                lines: 5,
                per_line: 4,
                words: 8,
                segments: 3,
            }
        }

        pub fn references(&self) -> u64 {
            self.requirements * self.lines * self.per_line
        }
    }

    impl Generate for ReferenceText {
        fn generate(&self) -> Tree {
            let mut draft = Tree::default();

            let mut children = Vec::new();
            let mut ordinal = 0;
            for r in 0..self.requirements {
                let mut items = Vec::new();
                for line in 0..self.lines {
                    if line > 0 {
                        items.push(Item::Newline);
                    }
                    for _ in 0..self.per_line {
                        let words = repeat_words(self.words as usize);
                        items.push(Item::Text(format!("{words} ")));
                        let path = (0..self.segments)
                            .map(|segment| format!("s{ordinal}_{segment}"))
                            .collect::<Vec<_>>()
                            .join("::");
                        items.push(Item::Reference(reference(&mut draft, path)));
                        ordinal += 1;
                    }
                }
                let text = Markup::from(items);
                let req = informal(&mut draft, format!("requirement_{r}"), text);
                children.push(add(&mut draft, req));
            }
            add_parent(&mut draft, package("root"), &children);

            draft.into()
        }

        fn size(&self) -> u64 {
            1 + self.requirements
        }
    }

    /// Flat draft: root package containing one part "attrs" (with attr_a: real, attr_b: real,
    /// attr_bool: boolean) and N*3 formal requirements cycling through:
    ///   - addition:  attr_a + attr_b
    ///   - all:       all x > 0 for x in real end
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
        fn generate(&self) -> Tree {
            let mut draft = Tree::default();
            let root = add_root(&mut draft, package("root"));
            let attrs = add_child(&mut draft, root, part("attrs"));
            for (label, domain) in [
                ("attr_a", "real"),
                ("attr_b", "real"),
                ("attr_bool", "boolean"),
            ] {
                let attribute = super::entity::attribute(&mut draft, label, domain);
                add_child(&mut draft, attrs, attribute);
            }

            for i in 0..self.cycles {
                let addition = make_addition(&mut draft, i);
                add_child(&mut draft, root, addition);
                let all = make_all(&mut draft, i);
                add_child(&mut draft, root, all);
                let when = make_when(&mut draft, i);
                add_child(&mut draft, root, when);
            }

            draft.into()
        }

        fn size(&self) -> u64 {
            5 + self.cycles * 3 // root + part + 3 attrs + N*3 reqs
        }
    }

    // attr_a + attr_b
    fn make_addition(draft: &mut Tree, i: u64) -> crate::node::entity::Node {
        let left = identifier(draft, "root::attrs::attr_a");
        let right = identifier(draft, "root::attrs::attr_b");
        let root = binary(draft, BinaryOperator::Plus, left, right);
        formal(&format!("addition_{i}"), root)
    }

    // all x > 0 for x in real end
    fn make_all(draft: &mut Tree, i: u64) -> crate::node::entity::Node {
        let domain = identifier(draft, "real");
        let x = identifier(draft, "x");
        let zero = number(draft, 0);
        let body = binary(draft, BinaryOperator::GreaterThan, x, zero);
        let set = SetElement::new("x".to_string(), domain);
        let all = Comprehension::new(Some(Aggregator::All), set, body);
        let root = super::expression::insert(draft, Variant::Comprehension(all));
        formal(&format!("all_{i}"), root)
    }

    // when attr_bool then attr_a + 1
    fn make_when(draft: &mut Tree, i: u64) -> crate::node::entity::Node {
        let condition = identifier(draft, "root::attrs::attr_bool");
        let attr = identifier(draft, "root::attrs::attr_a");
        let one = number(draft, 1);
        let consequence = binary(draft, BinaryOperator::Plus, attr, one);
        let when = When {
            cases: vec![Case::new(condition, consequence)],
            otherwise: None,
        };
        let root = super::expression::insert(draft, Variant::When(when));
        formal(&format!("when_{i}"), root)
    }
}

pub mod entity {
    use crate::{
        mock::expression,
        node::{
            entity::{
                Attribute, Idx, Meta, Node, Package, Part, Requirement, RequirementVariant, Type,
                Variant,
            },
            markup::{self, Markup},
            path::Path,
            reference,
            tree::Tree,
        },
        utils::children::ChildrenMut,
    };

    pub fn package<S: ToString>(label: S) -> Node {
        let variant = Variant::Package(Package::new(vec![], vec![]));
        Node::new(Meta::from(label.to_string()), variant)
    }

    pub fn part<S: ToString>(label: S) -> Node {
        let variant = Variant::Part(Part::new(vec![], vec![]));
        Node::new(Meta::from(label.to_string()), variant)
    }

    /// An attribute whose domain is the identifier `domain`, which is added to the draft
    pub fn attribute<S: ToString>(draft: &mut Tree, label: S, domain: S) -> Node {
        let domain = expression::identifier(draft, domain);
        let variant = Variant::Attribute(Attribute::new(domain));
        Node::new(Meta::from(label.to_string()), variant)
    }

    /// Adds a text to the markups of the draft
    pub fn markup(draft: &mut Tree, text: Markup) -> markup::Idx {
        let idx = draft.markups.len().into();
        draft.markups.push(text);
        idx
    }

    /// An informal requirement with no text, whose markup is added to the draft
    pub fn requirement<S: ToString>(draft: &mut Tree, label: S) -> Node {
        informal(draft, label, Markup::default())
    }

    /// An informal requirement with `text`, which is added to the draft
    pub fn informal<S: ToString>(draft: &mut Tree, label: S, text: Markup) -> Node {
        let text = markup(draft, text);
        let variant = RequirementVariant::Informal(text);
        let variant = Variant::Requirement(Requirement::from(variant));
        Node::new(Meta::from(label.to_string()), variant)
    }

    pub fn formal(label: &str, root: crate::node::expression::Idx) -> Node {
        let variant = RequirementVariant::Formal(root);
        let variant = Variant::Requirement(Requirement::from(variant));
        Node::new(Meta::from(label.to_string()), variant)
    }

    pub fn typ<S: ToString>(label: S, members: &[S]) -> Node {
        let variant = Variant::Type(Type::from(
            members
                .into_iter()
                .map(|m| m.to_string())
                .collect::<Vec<_>>(),
        ));
        Node::new(Meta::from(label.to_string()), variant)
    }

    /// Adds a path to the references of the draft
    pub fn reference<S: ToString>(draft: &mut Tree, path: S) -> reference::Idx {
        let idx = draft.references.len().into();
        draft.references.push(Path::from(path.to_string()));
        idx
    }

    pub fn add(draft: &mut Tree, node: Node) -> Idx {
        let idx = draft.entities.len().into();
        draft.entities.push(node);
        idx
    }

    /// Stores `node` after its `children`, the order in which the parser stores entities
    pub fn add_parent(draft: &mut Tree, mut node: Node, children: &[Idx]) -> Idx {
        node.variant
            .children_mut()
            .expect("node has no children")
            .extend(children);
        add(draft, node)
    }

    pub fn add_root(draft: &mut Tree, node: Node) -> Idx {
        add(draft, node)
    }

    pub fn add_child(draft: &mut Tree, parent: Idx, node: Node) -> Idx {
        let idx = add_root(draft, node);
        draft.entities[parent]
            .variant
            .children_mut()
            .expect("parent has no children")
            .push(idx);
        idx
    }

    /// The child of `parent` named `label`
    pub fn find(tree: &Tree, parent: Idx, label: &str) -> Idx {
        use crate::utils::children::Children;

        tree.entities[parent]
            .variant
            .children()
            .expect("node has no children")
            .iter()
            .copied()
            .find(|idx| tree.entities[*idx].meta.label == label)
            .unwrap_or_else(|| panic!("child '{}' not found", label))
    }
}

pub mod expression {
    use crate::node::{
        self,
        expression::{Identifier, Idx, Node, Variant},
        path::Path,
        tree::Tree,
    };

    pub fn insert(draft: &mut Tree, variant: Variant) -> Idx {
        let idx = draft.expressions.len().into();
        draft.expressions.push(Node::new(variant));
        idx
    }

    pub fn identifier<S: ToString>(draft: &mut Tree, location: S) -> Idx {
        let target = draft.identifiers.len().into();
        draft.identifiers.push(Path::from(location.to_string()));
        insert(draft, Variant::Identifier(Identifier::new(target)))
    }

    pub fn number(draft: &mut Tree, n: i64) -> Idx {
        let num = node::expression::Number {
            kind: node::expression::NumberKind::Integer,
            value: n.to_string(),
        };
        insert(draft, Variant::Number(num))
    }

    pub fn boolean(draft: &mut Tree, b: bool) -> Idx {
        insert(draft, Variant::Boolean(node::expression::Boolean::from(b)))
    }

    pub fn binary(
        draft: &mut Tree,
        op: node::expression::BinaryOperator,
        left: Idx,
        right: Idx,
    ) -> Idx {
        insert(
            draft,
            Variant::BinOp(node::expression::BinOp::new(op, left, right)),
        )
    }
}
