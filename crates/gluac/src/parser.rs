use crate::ast::Program;
use crate::errors::CompilerResult;
use crate::lexer::Token;

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    pub fn parse_program(&self) -> CompilerResult<Program> {
        let _ = &self.tokens;
        Ok(Program::empty())
    }
}
