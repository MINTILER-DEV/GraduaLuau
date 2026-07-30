use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::parser::ast_builder::{make_program, AstNode};
use crate::parser::cursor::Cursor;
use crate::source::{FileId, SourceSpan};

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

    pub fn parse_program(&mut self) -> AstNode {
        let first_span = self.current().map(|tok| tok.span);
        let start = first_span.map(|span| span.start()).unwrap_or(0);
        let file_id = first_span.map(|span| span.file_id()).unwrap_or(FileId::new(0));
        let mut end = start;

        while let Some(tok) = self.current() {
            end = tok.span.end();
            if matches!(tok.kind, TokenKind::EOF) {
                break;
            }
            self.advance();
        }

        make_program(SourceSpan::new(file_id, start, end))
    }

    pub fn diagnostics(&self) -> &[Diagnostic] { &self.diagnostics }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::source::SourceManager;
    use std::path::PathBuf;

    #[test]
    fn parse_program_returns_program_ast() {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("test.glu"), String::from(""));
        let file = sources.get(file_id).unwrap();

        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token();
            tokens.push(token.clone());
            if matches!(token.kind, crate::lexer::TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_program();

        assert!(matches!(ast, AstNode::Program(_)));
    }
}
