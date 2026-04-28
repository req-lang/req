pub mod location;
pub mod parser;
pub mod tokenizer;

pub mod entity {
    use crate::{node, reference::Reference, syntax::parser};

    pub type Node = node::entity::Node<(), Reference, (), ()>;

    impl From<parser::entity::Node> for Node {
        fn from(value: parser::entity::Node) -> Self {
            value.map(|_| ()).mape(|_| ())
        }
    }
}

pub mod expression {
    use crate::{node, reference::Reference, syntax::parser};

    pub type Node = node::expression::Node<(), Reference>;

    impl From<parser::expression::Node> for Node {
        fn from(value: parser::expression::Node) -> Self {
            value.map(|_| ())
        }
    }
}

pub mod markup {
    use crate::{node, reference::Reference};

    pub type Node = node::markup::Node<(), Reference>;
}
