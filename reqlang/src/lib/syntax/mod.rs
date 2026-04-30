pub mod location;
pub mod parser;
pub mod tokenizer;

pub mod entity {
    use crate::{node, syntax::parser};

    pub type Node = node::entity::Node<(), String, String, (), ()>;

    impl From<parser::entity::Node> for Node {
        fn from(value: parser::entity::Node) -> Self {
            value.map(|_| ()).mape(|_| ()).mapm(|_| ())
        }
    }
}

pub mod expression {
    use crate::{node, syntax::parser};

    pub type Node = node::expression::Node<(), String>;

    impl From<parser::expression::Node> for Node {
        fn from(value: parser::expression::Node) -> Self {
            value.map(|_| ())
        }
    }
}

pub mod markup {
    use crate::node;

    pub type Node = node::markup::Node<(), String>;
}
