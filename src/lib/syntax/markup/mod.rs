use tokenizer::{IntoTokens, Token};

use crate::key::NodeKey;

use super::{NodeParser, Position, Span, entity::Reference};

pub mod tokenizer;

pub(crate) type MarkupKey<'a> = NodeKey<'a, Markup>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Item<R = Reference> {
    Text(String),
    Reference(R),
}

impl Item {
    pub fn to_string(self) -> String {
        match self {
            Item::Text(s) => s,
            Item::Reference(r) => format!("{{{}}}", r.value),
        }
    }

    pub fn from_token(token: Token<'_>) -> Self {
        match token {
            Token::Text(text) => Item::Text(text.to_string()),
            Token::Reference(ident) => Item::Reference(Reference::new(ident.to_string())),
        }
    }
}

/// This container owns the markdown text associated with markup tokens.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Markup<R = Reference> {
    pub value: Vec<Item<R>>,
}

impl From<String> for Markup {
    fn from(value: String) -> Self {
        let tokens = value.char_indices().tokens();
        let value = tokens.map(|token| Item::from_token(token)).collect();
        Self { value }
    }
}

impl Markup {
    pub fn to_string(self) -> String {
        self.value
            .into_iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn references(&self) -> impl Iterator<Item = &Reference> {
        self.value.iter().filter_map(|item| match item {
            Item::Reference(r) => Some(r),
            _ => None,
        })
    }

    pub fn references_mut(&mut self) -> impl Iterator<Item = &mut Reference> {
        self.value.iter_mut().filter_map(|item| match item {
            Item::Reference(r) => Some(r),
            _ => None,
        })
    }

    pub fn push_str(&mut self, str: &str) {
        let items = str.char_indices().tokens().map(|t| Item::from_token(t));
        self.value.extend(items);
    }
}

impl NodeParser<'_> {
    pub fn parse_markup(&mut self, value: String, end: Position) -> Markup {
        let container = Markup::from(value);
        let span = Span::new(self.offset, end);

        self.spans.push(span);
        container
    }
}
