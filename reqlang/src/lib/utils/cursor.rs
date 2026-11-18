use crate::syntax::location::{Span, SpanLike};

/// Reads a sequence of tokens one at a time. It owns the tokens, so that it can live in the
/// struct that consumes them; [Self::into_tokens] gives them back.
#[derive(Debug, Clone)]
pub struct Cursor<T> {
    tokens: Vec<T>,
    position: usize,
}

impl<T> Default for Cursor<T> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<T> Cursor<T> {
    pub fn new(tokens: Vec<T>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn into_tokens(self) -> Vec<T> {
        self.tokens
    }

    /// Index of the next token to be bumped.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Number of tokens, read or not.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// The last token, read or not.
    pub fn last(&self) -> Option<&T> {
        self.tokens.last()
    }

    pub fn extend(&mut self, tokens: impl Iterator<Item = T>) {
        self.tokens.extend(tokens);
    }

    /// Appends a token, for the layers that produce their tokens while parsing.
    pub fn push(&mut self, token: T) {
        self.tokens.push(token);
    }

    /// The tokens from the `from`th on, read or not.
    pub fn since_mut(&mut self, from: usize) -> &mut [T] {
        &mut self.tokens[from..]
    }
}

/// A token that is made of a kind and of other data, such as where it is in the text.
pub trait Kinded {
    type Kind: PartialEq;

    fn kind(&self) -> Self::Kind;
}

impl<T: Copy + Kinded + SpanLike> Cursor<T> {
    /// Bumps the next token if it is of this kind.
    pub fn eat(&mut self, kind: T::Kind) -> bool {
        let found = self.peek().is_some_and(|token| token.kind() == kind);
        if found {
            self.position += 1;
        }
        found
    }

    /// Bumps and returns the next token if it is of this kind. Otherwise the token is left, and
    /// the error is its span, or `None` at the end of the tokens.
    pub fn expect(&mut self, kind: T::Kind) -> Result<T, Option<Span>> {
        let token = self.peek().ok_or(None)?;
        match token.kind() == kind {
            true => {
                self.position += 1;
                Ok(token)
            }
            false => Err(Some(token.span())),
        }
    }
}

impl<T: Copy> Cursor<T> {
    pub fn peek_nth(&self, n: usize) -> Option<T> {
        self.tokens.get(self.position + n).copied()
    }

    pub fn peek(&self) -> Option<T> {
        self.peek_nth(0)
    }

    pub fn bump(&mut self) -> Option<T> {
        let token = self.peek()?;
        self.position += 1;
        Some(token)
    }
}
