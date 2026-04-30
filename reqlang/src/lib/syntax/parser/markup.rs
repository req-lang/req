use crate::{
    node::{
        self,
        markup::{Container, Item},
    },
    syntax::tokenizer::markup::{IntoTokens, Token},
};

pub(in crate::syntax) type Node = node::markup::Node<(), String>;

pub fn parse<S: ToString>(s: &S) -> Node {
    Node::from(s.to_string())
}

impl Item<String> {
    pub fn from_token(token: Token<'_>) -> Self {
        match token {
            Token::Text(text) => Item::Text(text.to_string()),
            Token::Reference(ident) => Item::Reference(ident.to_string()),
            Token::Newline => Item::Newline,
        }
    }
}

impl From<String> for Node {
    fn from(value: String) -> Self {
        let unindented = unindent::unindent(&value);
        let tokens = unindented.char_indices().tokens();
        let value: Vec<_> = tokens.map(|token| Item::from_token(token)).collect();
        Node::empty(Container::from(value))
    }
}
