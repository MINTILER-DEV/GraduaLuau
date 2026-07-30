use crate::lexer::Token;

pub struct Cursor<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(tokens: &'a [Token]) -> Self { Self { tokens, index: 0 } }
    pub fn current(&self) -> Option<&'a Token> { self.tokens.get(self.index) }
    pub fn peek(&self) -> Option<&'a Token> { self.tokens.get(self.index + 1) }
    pub fn advance(&mut self) { if self.index < self.tokens.len() { self.index += 1; } }
    pub fn previous(&self) -> Option<&'a Token> { self.tokens.get(self.index.saturating_sub(1)) }
}
