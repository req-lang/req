pub struct Cursor<'t, T> {
    tokens: &'t [T],
    position: usize,
}

impl<'t, T> Cursor<'t, T> {
    pub fn new(tokens: &'t [T]) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn peek_nth(&self, n: usize) -> Option<&'t T> {
        self.tokens.get(self.position + n)
    }

    pub fn peek(&self) -> Option<&'t T> {
        self.peek_nth(0)
    }

    pub fn bump(&mut self) -> Option<&'t T> {
        let token = self.tokens.get(self.position)?;
        self.position += 1;
        Some(token)
    }
}
