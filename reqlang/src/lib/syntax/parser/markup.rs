use crate::{
    node::{
        self,
        markup::{Container, Item},
    },
    reference::Reference,
    syntax::tokenizer::markup::{IntoTokens, Token},
};

pub type Node = node::markup::Node<(), Reference>;

impl Item<Reference> {
    pub fn to_string(self) -> String {
        match self {
            Item::Text(s) => s,
            Item::Reference(r) => format!("{{{}}}", r),
            Item::Newline => "\n".to_string(),
        }
    }

    pub fn from_token(token: Token<'_>) -> Self {
        match token {
            Token::Text(text) => Item::Text(text.to_string()),
            Token::Reference(ident) => Item::Reference(Reference::new(ident.to_string())),
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

impl Node {
    pub fn to_string(self) -> String {
        self.container
            .items
            .into_iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn push_str(&mut self, str: &str) {
        let items = str.char_indices().tokens().map(|t| Item::from_token(t));
        self.container.items.extend(items);
    }
}
