use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::parser::ast_builder::{make_program, AstNode, Expression, ExpressionKind, Statement, StatementKind};
use crate::parser::cursor::Cursor;
use crate::parser::precedence::Precedence;
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
        let mut statements = Vec::new();
        let mut end = start;

        while let Some(tok) = self.current() {
            end = tok.span.end();
            if matches!(tok.kind, TokenKind::EOF) {
                break;
            }

            let statement = self.parse_expression_statement();
            end = statement.span.end();
            statements.push(statement);

            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Semicolon)) {
                self.advance();
            }
        }

        make_program(statements, SourceSpan::new(file_id, start, end))
    }

    pub fn parse_expression(&mut self) -> Expression {
        self.parse_precedence(Precedence::Lowest)
    }

    pub fn diagnostics(&self) -> &[Diagnostic] { &self.diagnostics }

    fn parse_expression_statement(&mut self) -> Statement {
        let expression = self.parse_expression();
        let span = expression.span;
        Statement { kind: StatementKind::Expression(expression), span }
    }

    fn parse_prefix_expression(&mut self) -> Expression {
        let token = self.current().cloned();

        match token.as_ref().map(|tok| &tok.kind) {
            Some(TokenKind::Minus) | Some(TokenKind::Not) => {
                let operator = match token.as_ref().unwrap().kind {
                    TokenKind::Minus => "-".to_string(),
                    TokenKind::Not => "not".to_string(),
                    _ => unreachable!(),
                };
                let start = token.as_ref().unwrap().span.start();
                self.advance();
                let operand = self.parse_precedence(Precedence::Unary);
                let span = SourceSpan::new(operand.span.file_id(), start, operand.span.end());
                Expression { kind: ExpressionKind::Unary { operator, operand: Box::new(operand) }, span }
            }
            Some(TokenKind::LeftParen) => {
                self.advance();
                let expression = self.parse_expression();
                if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen)) {
                    self.advance();
                }
                expression
            }
            Some(TokenKind::Identifier(_)) => {
                let current = self.current().unwrap();
                if let TokenKind::Identifier(name) = current.kind.clone() {
                    let span = current.span;
                    self.advance();
                    Expression { kind: ExpressionKind::Identifier(name), span }
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::NumberLiteral(_)) => {
                let current = self.current().unwrap();
                if let TokenKind::NumberLiteral(value) = current.kind.clone() {
                    let span = current.span;
                    self.advance();
                    Expression { kind: ExpressionKind::NumberLiteral(value), span }
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::StringLiteral(_)) => {
                let current = self.current().unwrap();
                if let TokenKind::StringLiteral(value) = current.kind.clone() {
                    let span = current.span;
                    self.advance();
                    Expression { kind: ExpressionKind::StringLiteral(value), span }
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::True) | Some(TokenKind::False) | Some(TokenKind::Nil) => {
                let current = self.current().unwrap();
                let kind = match current.kind {
                    TokenKind::True => ExpressionKind::BooleanLiteral(true),
                    TokenKind::False => ExpressionKind::BooleanLiteral(false),
                    TokenKind::Nil => ExpressionKind::Nil,
                    _ => unreachable!(),
                };
                let span = current.span;
                self.advance();
                Expression { kind, span }
            }
            _ => {
                let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                self.advance();
                Expression { kind: ExpressionKind::Nil, span }
            }
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Expression {
        let mut left = self.parse_prefix_expression();
        left = self.parse_postfix_expression(left);

        while let Some(tok) = self.current() {
            let next_prec = match Precedence::of_token(&tok.kind) {
                Some(p) => p,
                None => break,
            };
            if next_prec < precedence {
                break;
            }

            let operator = tok.kind.clone();
            self.advance();
            let rhs_precedence = if Precedence::is_right_associative(&operator) {
                next_prec
            } else {
                next_prec.next()
            };
            let right = self.parse_precedence(rhs_precedence);
            let span = SourceSpan::new(left.span.file_id(), left.span.start(), right.span.end());
            left = Expression {
                kind: ExpressionKind::Binary {
                    left: Box::new(left),
                    operator: self.operator_name(&operator),
                    right: Box::new(right),
                },
                span,
            };
        }

        left
    }

    fn parse_postfix_expression(&mut self, mut expression: Expression) -> Expression {
        loop {
            match self.current().map(|tok| &tok.kind) {
                Some(TokenKind::LeftParen) => {
                    expression = self.parse_call_expression(expression);
                }
                _ => break,
            }
        }

        expression
    }

    fn parse_call_expression(&mut self, callee: Expression) -> Expression {
        let start = callee.span.start();
        let file_id = callee.span.file_id();
        self.advance();
        let mut arguments = Vec::new();

        while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen) | Some(TokenKind::EOF)) {
            arguments.push(self.parse_expression());
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                self.advance();
            } else {
                break;
            }
        }

        let end = self.current().map(|tok| tok.span.end()).unwrap_or(start);
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen)) {
            self.advance();
        }

        Expression {
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn operator_name(&self, kind: &TokenKind) -> String {
        match kind {
            TokenKind::Plus => "+".to_string(),
            TokenKind::Minus => "-".to_string(),
            TokenKind::Star => "*".to_string(),
            TokenKind::Slash => "/".to_string(),
            TokenKind::Percent => "%".to_string(),
            TokenKind::Caret => "^".to_string(),
            TokenKind::EqualEqual => "==".to_string(),
            TokenKind::NotEqual => "~=".to_string(),
            TokenKind::Less => "<".to_string(),
            TokenKind::LessEqual => "<=".to_string(),
            TokenKind::Greater => ">".to_string(),
            TokenKind::GreaterEqual => ">=".to_string(),
            TokenKind::And => "and".to_string(),
            TokenKind::Or => "or".to_string(),
            TokenKind::DotDot => "..".to_string(),
            _ => "".to_string(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::ast_builder::ExpressionKind;
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
            if matches!(token.kind, TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_program();

        assert!(matches!(ast, AstNode::Program(_)));
    }

    #[test]
    fn parse_binary_expression_respects_precedence() {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("test.glu"), String::from("1 + 2 * 3"));
        let file = sources.get(file_id).unwrap();

        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            tokens.push(token.clone());
            if matches!(token.kind, TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let expr = parser.parse_expression();

        match expr.kind {
            ExpressionKind::Binary { left, operator, right } => {
                assert_eq!(operator, "+");
                assert!(matches!(left.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
                match right.kind {
                    ExpressionKind::Binary { left: inner_left, operator: inner_op, right: inner_right } => {
                        assert_eq!(inner_op, "*");
                        assert!(matches!(inner_left.kind, ExpressionKind::NumberLiteral(ref s) if s == "2"));
                        assert!(matches!(inner_right.kind, ExpressionKind::NumberLiteral(ref s) if s == "3"));
                    }
                    _ => panic!("expected multiplication on right-hand side"),
                }
            }
            _ => panic!("expected top-level binary expression"),
        }
    }

    #[test]
    fn parse_call_expression_parses_arguments() {
        let mut sources = SourceManager::new();
        let src = String::from("print(1, 2)");
        let file_id = sources.add_file(PathBuf::from("test.glu"), src);
        let file = sources.get(file_id).unwrap();

        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            tokens.push(token.clone());
            if matches!(token.kind, TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let expr = parser.parse_expression();

        match expr.kind {
            ExpressionKind::Call { callee, arguments } => {
                assert!(matches!(callee.kind, ExpressionKind::Identifier(ref s) if s == "print"));
                assert_eq!(arguments.len(), 2);
                assert!(matches!(arguments[0].kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
                assert!(matches!(arguments[1].kind, ExpressionKind::NumberLiteral(ref s) if s == "2"));
            }
            _ => panic!("expected call expression"),
        }
    }
}
