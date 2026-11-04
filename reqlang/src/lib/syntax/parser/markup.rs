use crate::{
    node::{
        markup::{Item, Markup},
        path::Path,
    },
    syntax::tokenizer::markup::{IntoTokens, Token},
};

/// Parses the text, the paths it references are appended to `references`
pub fn parse<S: ToString>(s: &S, references: &mut Vec<Path>) -> Markup {
    let unindented = unindent::unindent(&s.to_string());
    let tokens = unindented.char_indices().tokens();
    let items: Vec<_> = tokens
        .map(|token| match token {
            Token::Text(text) => Item::Text(text.to_string()),
            Token::Reference(ident) => {
                let idx = references.len().into();
                references.push(Path::from(ident));
                Item::Reference(idx)
            }
            Token::Newline => Item::Newline,
        })
        .collect();
    Markup::from(items)
}
