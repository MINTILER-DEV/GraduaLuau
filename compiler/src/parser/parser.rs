use crate::lexer::TokenKind;
use crate::source::SourceSpan;

pub struct Parser<'a> {
    // placeholder fields
    tokens: Vec<TokenKind>,
    index: usize,
    diagnostics: Vec<String>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<TokenKind>) -> Self {
        Self { tokens, index: 0, diagnostics: Vec::new(), _marker: Default::default() }
    }

    pub fn parse_program(&mut self) {
        // placeholder: consume tokens to exercise scaffolding
        while self.index < self.tokens.len() {
            self.index += 1;
        }
    }

    pub fn diagnostics(&self) -> &[String] { &self.diagnostics }
}
