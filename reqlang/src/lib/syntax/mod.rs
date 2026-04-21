pub mod location;
pub mod parser;
pub mod tokenizer;

pub mod entity {

    use reqlang_derive::ChildrenIter;

    use crate::{
        node::{self, NodeRef, entity::Variant},
        reference::Reference,
        syntax::{
            expression,
            parser::{self, markup},
        },
        utils::children::ChildrenIter,
    };

    #[derive(Default, Clone, Debug, PartialEq, Eq, ChildrenIter)]
    pub struct Node {
        pub variant: Variant<NodeRef<Node>, Reference, NodeRef<expression::Node>, markup::Node>,
    }

    impl Node {
        pub fn new(
            variant: Variant<NodeRef<Node>, Reference, NodeRef<expression::Node>, markup::Node>,
        ) -> Self {
            Self { variant }
        }
    }

    impl From<parser::entity::Node> for Node {
        fn from(node: parser::entity::Node) -> Self {
            let variant = node.variant.mapn(|n| node::map(n, Node::from));
            let variant = variant.mape(|e| node::map(e, expression::Node::from));
            Node::new(variant)
        }
    }
}

pub mod expression {
    use crate::{
        node::{self, NodeRef, expression},
        reference::Reference,
        syntax::parser,
    };

    pub type Variant = expression::Variant<NodeRef<Node>, Reference>;

    #[derive(Debug, Default, PartialEq, Eq)]
    pub struct Node {
        pub variant: Variant,
    }

    impl Node {
        pub fn new(variant: Variant) -> Self {
            Self { variant }
        }
    }

    impl From<parser::expression::Node> for Node {
        fn from(value: parser::expression::Node) -> Self {
            let variant = value.variant.mape(|e| node::map(e, Node::from));
            Node::new(variant)
        }
    }
}
