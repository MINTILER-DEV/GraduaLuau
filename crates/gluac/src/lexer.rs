use crate::errors::CompilerResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub lexeme: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Lexer<'source> {
    source: &'source str,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source }
    }

    pub fn tokenize(&self) -> CompilerResult<Vec<Token>> {
        let _ = self.source;
        Ok(Vec::new())
    }
}
