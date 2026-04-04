use crate::{
    node::markup,
    reference::Reference,
    syntax::tokenizer::markup::{IntoTokens, Token},
};

pub type Node = markup::Node<Reference>;
pub type Item = markup::Item<Reference>;

impl Item {
    pub fn to_string(self) -> String {
        match self {
            Item::Text(s) => s,
            Item::Reference(r) => format!("{{{}}}", r.value),
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
        let value = tokens.map(|token| Item::from_token(token)).collect();
        Self { items: value }
    }
}

impl Node {
    pub fn to_string(self) -> String {
        self.items
            .into_iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn push_str(&mut self, str: &str) {
        let items = str.char_indices().tokens().map(|t| Item::from_token(t));
        self.items.extend(items);
    }
}
