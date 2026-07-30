use crate::diagnostics::Diagnostic;
use crate::lexer::Token;
use crate::parser::cursor::Cursor;

pub struct Parser<'a> {
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { cursor: Cursor::new(tokens), diagnostics: Vec::new() }
    }

    pub fn current(&self) -> Option<&'a Token> {
        self.cursor.current()
    }

    pub fn peek(&self) -> Option<&'a Token> {
        self.cursor.peek()
    }

    pub fn advance(&mut self) {
        self.cursor.advance();
    }

    pub fn parse_program(&mut self) {
        // consume tokens until EOF — real implementation will build AST
        while let Some(tok) = self.current() {
            if matches!(tok.kind, crate::lexer::TokenKind::EOF) {
                break;
            }
            self.advance();
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] { &self.diagnostics }
}
