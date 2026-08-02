use crate::diagnostics::Diagnostic;
use crate::lexer::{Token, TokenKind};
use crate::parser::ast_builder::{make_program, AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind, TableField, TypeExpression, TypeExpressionKind};
use crate::parser::cursor::Cursor;
use crate::parser::precedence::Precedence;
use crate::parser::recovery::{RecoveryState, is_synchronization_token, is_expression_boundary, is_statement_boundary, can_start_type};
use crate::source::{FileId, SourceSpan};

pub struct Parser<'a> {
    cursor: Cursor<'a>,
    diagnostics: Vec<Diagnostic>,
    recovery: RecoveryState,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { 
            cursor: Cursor::new(tokens), 
            diagnostics: Vec::new(),
            recovery: RecoveryState::new(),
        }
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

            let statement = self.parse_statement();
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

    fn emit_error(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.recovery.increment_error_count();
        let diagnostic = Diagnostic::builder(crate::diagnostics::Severity::Error)
            .message(message)
            .span(span)
            .build();
        self.diagnostics.push(diagnostic);
    }

    fn emit_error_with_help(&mut self, message: impl Into<String>, span: SourceSpan, help: impl Into<String>) {
        self.recovery.increment_error_count();
        let diagnostic = Diagnostic::builder(crate::diagnostics::Severity::Error)
            .message(message)
            .span(span)
            .help(help)
            .build();
        self.diagnostics.push(diagnostic);
    }

    fn synchronize_to_statement_boundary(&mut self) {
        self.recovery.set_in_error_recovery(true);
        
        while let Some(token) = self.current() {
            if is_statement_boundary(&token.kind) || is_synchronization_token(&token.kind) {
                break;
            }
            self.advance();
        }
        
        self.recovery.set_in_error_recovery(false);
    }

    fn synchronize_to_expression_boundary(&mut self) {
        self.recovery.set_in_error_recovery(true);
        
        while let Some(token) = self.current() {
            if is_expression_boundary(&token.kind) {
                break;
            }
            self.advance();
        }
        
        self.recovery.set_in_error_recovery(false);
    }

    fn make_error_statement(&mut self, span: SourceSpan) -> Statement {
        Statement {
            kind: StatementKind::Error,
            span,
        }
    }

    fn make_error_expression(&mut self, span: SourceSpan) -> Expression {
        Expression {
            kind: ExpressionKind::Error,
            span,
        }
    }

    fn parse_statement(&mut self) -> Statement {
        // Check for cascading error suppression
        if self.recovery.should_suppress_cascading() {
            let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
            self.advance(); // Always consume at least one token
            return self.make_error_statement(span);
        }

        match self.current().map(|tok| &tok.kind) {
            Some(TokenKind::Local) => {
                if matches!(self.peek().map(|tok| &tok.kind), Some(TokenKind::Function)) {
                    self.parse_local_function_declaration()
                } else {
                    self.parse_local_statement()
                }
            }
            Some(TokenKind::Function) => self.parse_function_declaration(false),
            Some(TokenKind::Type) => self.parse_type_alias_statement(),
            Some(TokenKind::Return) => self.parse_return_statement(),
            Some(TokenKind::Break) => self.parse_break_statement(),
            Some(TokenKind::Continue) => self.parse_continue_statement(),
            _ => self.parse_assignment_or_expression_statement(),
        }
    }

    fn parse_local_statement(&mut self) -> Statement {
        self.advance();
        let mut names = Vec::new();

        // Handle missing identifier after 'local'
        if !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Identifier(_))) {
            let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
            self.emit_error("Expected identifier after 'local'", span);
            self.synchronize_to_statement_boundary();
            return self.make_error_statement(span);
        }

        while let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
            self.advance();
            let annotation = self.parse_type_annotation();
            names.push((name, annotation));
            if !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                break;
            }
            self.advance();
        }

        let mut initializers = Vec::new();
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Equal)) {
            self.advance();
            initializers = self.parse_expression_list();
        }

        let span = if let Some(last) = initializers.last() {
            last.span
        } else {
            self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0))
        };

        Statement {
            kind: StatementKind::Local { names, initializers },
            span,
        }
    }

    fn parse_return_statement(&mut self) -> Statement {
        self.advance();
        let values = if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Semicolon) | Some(TokenKind::EOF)) {
            Vec::new()
        } else {
            self.parse_expression_list()
        };
        let span = if let Some(last) = values.last() {
            last.span
        } else {
            self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0))
        };
        Statement { kind: StatementKind::Return(Some(values)), span }
    }

    fn parse_break_statement(&mut self) -> Statement {
        let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
        self.advance();
        Statement { kind: StatementKind::Break, span }
    }

    fn parse_continue_statement(&mut self) -> Statement {
        let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
        self.advance();
        Statement { kind: StatementKind::Continue, span }
    }

    fn parse_assignment_or_expression_statement(&mut self) -> Statement {
        let first_expr = self.parse_expression();
        let mut targets = vec![first_expr];

        while matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
            self.advance();
            targets.push(self.parse_expression());
        }

        let statement = match self.current().map(|tok| &tok.kind) {
            Some(TokenKind::Equal) | Some(TokenKind::PlusEqual) | Some(TokenKind::MinusEqual)
            | Some(TokenKind::StarEqual) | Some(TokenKind::SlashEqual) | Some(TokenKind::PercentEqual)
            | Some(TokenKind::AmpersandEqual) | Some(TokenKind::PipeEqual) => {
                let operator = self.current().unwrap().kind.clone();
                self.advance();
                let values = self.parse_expression_list();
                let span = values.last().map(|expr| expr.span).unwrap_or(targets.last().unwrap().span);
                Statement {
                    kind: StatementKind::Assignment {
                        targets,
                        values,
                        operator: self.operator_name(&operator),
                    },
                    span,
                }
            }
            _ => {
                let span = targets.last().unwrap().span;
                if targets.len() == 1 {
                    Statement { kind: StatementKind::Expression(targets.into_iter().next().unwrap()), span }
                } else {
                    Statement { kind: StatementKind::Expression(targets.pop().unwrap()), span }
                }
            }
        };

        statement
    }

    fn parse_local_function_declaration(&mut self) -> Statement {
        self.advance();
        let function_statement = self.parse_function_declaration(true);
        function_statement
    }

    fn parse_function_declaration(&mut self, is_local: bool) -> Statement {
        if is_local {
            self.advance();
        }
        self.advance();

        let mut receiver = None;
        let name = if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
            self.advance();
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Colon)) {
                self.advance();
                if let Some(TokenKind::Identifier(method_name)) = self.current().map(|tok| tok.kind.clone()) {
                    receiver = Some(name);
                    self.advance();
                    method_name
                } else {
                    name
                }
            } else {
                name
            }
        } else {
            String::new()
        };

        let params = self.parse_parameter_list();
        let return_type = self.parse_type_annotation();
        let body = self.parse_block();
        let span = body.last().map(|stmt| stmt.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));

        Statement {
            kind: StatementKind::Function { name, receiver, params, return_type, body, is_local },
            span,
        }
    }

    fn parse_parameter_list(&mut self) -> Vec<(String, Option<TypeExpression>)> {
        let mut params = Vec::new();
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::LeftParen)) {
            self.advance();
            while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen) | Some(TokenKind::EOF)) {
                if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
                    self.advance();
                    let annotation = self.parse_type_annotation();
                    params.push((name, annotation));
                    if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    // Error recovery for invalid parameter
                    let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                    self.emit_error("Expected parameter name", span);
                    self.synchronize_to_expression_boundary();
                    break;
                }
            }
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen)) {
                self.advance();
            } else {
                // Missing closing parenthesis
                let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                self.emit_error_with_help("Expected ')' after parameter list", span, "Insert ')' here");
            }
        }
        params
    }

    fn parse_block(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();
        while let Some(kind) = self.current().map(|tok| &tok.kind) {
            if matches!(kind, TokenKind::End) {
                self.advance();
                break;
            }
            let statement = self.parse_statement();
            statements.push(statement);
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Semicolon)) {
                self.advance();
            }
        }
        statements
    }

    fn parse_type_alias_statement(&mut self) -> Statement {
        self.advance();
        let name = if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
            self.advance();
            name
        } else {
            String::new()
        };

        let alias = if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Equal)) {
            self.advance();
            self.parse_type_expression()
        } else {
            TypeExpression { kind: TypeExpressionKind::Named(String::new()), span: SourceSpan::new(FileId::new(0), 0, 0) }
        };

        Statement {
            kind: StatementKind::TypeAlias { name, alias },
            span: self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0)),
        }
    }

    fn parse_type_annotation(&mut self) -> Option<TypeExpression> {
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Colon)) {
            self.advance();
            
            // Handle missing type after ':'
            if !can_start_type(self.current().map(|tok| &tok.kind).unwrap_or(&TokenKind::EOF)) {
                let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                self.emit_error_with_help("Expected type after ':'", span, "Add a type annotation after ':'");
                return None;
            }
            
            return Some(self.parse_type_expression());
        }
        None
    }

    fn parse_type_expression(&mut self) -> TypeExpression {
        self.parse_union_type()
    }

    fn parse_union_type(&mut self) -> TypeExpression {
        let mut types = vec![self.parse_intersection_type()];
        while matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Pipe)) {
            self.advance();
            types.push(self.parse_intersection_type());
        }
        if types.len() == 1 {
            types.into_iter().next().unwrap()
        } else {
            let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
            TypeExpression { kind: TypeExpressionKind::Union(types), span }
        }
    }

    fn parse_intersection_type(&mut self) -> TypeExpression {
        let mut types = vec![self.parse_optional_type()];
        while matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Ampersand)) {
            self.advance();
            types.push(self.parse_optional_type());
        }
        if types.len() == 1 {
            types.into_iter().next().unwrap()
        } else {
            let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
            TypeExpression { kind: TypeExpressionKind::Intersection(types), span }
        }
    }

    fn parse_optional_type(&mut self) -> TypeExpression {
        let mut typ = self.parse_primary_type();
        while matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Question)) {
            let start = typ.span.start();
            let file_id = typ.span.file_id();
            self.advance();
            let span = self.current().map(|tok| tok.span).unwrap_or(typ.span);
            let end = span.end();
            typ = TypeExpression { kind: TypeExpressionKind::Optional(Box::new(typ)), span: SourceSpan::new(file_id, start, end) };
        }
        typ
    }

    fn parse_primary_type(&mut self) -> TypeExpression {
        match self.current().map(|tok| &tok.kind) {
            Some(TokenKind::LeftParen) => self.parse_parenthesized_type(),
            Some(TokenKind::LeftBrace) => self.parse_table_or_array_type(),
            Some(TokenKind::DotDotDot) => self.parse_variadic_type(),
            Some(TokenKind::Identifier(_))
            | Some(TokenKind::Any)
            | Some(TokenKind::Never)
            | Some(TokenKind::Nil) => self.parse_named_type(),
            _ => {
                let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                self.advance();
                TypeExpression { kind: TypeExpressionKind::Named(String::new()), span }
            }
        }
    }

    fn parse_parenthesized_type(&mut self) -> TypeExpression {
        let start = self.current().map(|tok| tok.span.start()).unwrap_or(0);
        self.advance();
        let mut types = Vec::new();

        while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen) | Some(TokenKind::EOF)) {
            types.push(self.parse_type_expression());
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                self.advance();
            } else {
                break;
            }
        }

        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen)) {
            self.advance();
        }

        let end = self.current().map(|tok| tok.span.end()).unwrap_or(start);
        let type_expr = if types.len() == 1 {
            let inner = types.remove(0);
            let file_id = inner.span.file_id();
            TypeExpression { kind: TypeExpressionKind::Parenthesized(Box::new(inner)), span: SourceSpan::new(file_id, start, end) }
        } else {
            let file_id = self.current().map(|tok| tok.span.file_id()).unwrap_or(FileId::new(0));
            TypeExpression { kind: TypeExpressionKind::Tuple(types), span: SourceSpan::new(file_id, start, end) }
        };

        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Arrow)) {
            self.advance();
            let return_type = self.parse_type_expression();
            let params = match type_expr.kind {
                TypeExpressionKind::Tuple(elements) => elements,
                TypeExpressionKind::Parenthesized(inner) => vec![*inner],
                _ => vec![type_expr],
            };
            let file_id = return_type.span.file_id();
            let span = SourceSpan::new(file_id, start, return_type.span.end());
            TypeExpression { kind: TypeExpressionKind::Function { params, return_type: Box::new(return_type) }, span }
        } else {
            type_expr
        }
    }

    fn parse_table_or_array_type(&mut self) -> TypeExpression {
        let start = self.current().map(|tok| tok.span.start()).unwrap_or(0);
        self.advance();

        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace)) {
            self.advance();
            let file_id = self.current().map(|tok| tok.span.file_id()).unwrap_or(FileId::new(0));
            return TypeExpression { kind: TypeExpressionKind::Table(Vec::new()), span: SourceSpan::new(file_id, start, self.current().map(|tok| tok.span.end()).unwrap_or(start)) };
        }

        let peeked = self.peek().map(|tok| &tok.kind);
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Identifier(_))) && matches!(peeked, Some(TokenKind::Colon)) {
            let mut fields = Vec::new();
            while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace) | Some(TokenKind::EOF)) {
                if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
                    let field_start = self.current().map(|tok| tok.span.start()).unwrap_or(start);
                    self.advance();
                    self.advance();
                    let field_type = self.parse_type_expression();
                    let field_span = SourceSpan::new(field_type.span.file_id(), field_start, field_type.span.end());
                    fields.push((name, field_type, field_span));
                    if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace)) {
                self.advance();
            }
            let file_id = self.current().map(|tok| tok.span.file_id()).unwrap_or(FileId::new(0));
            let end = self.current().map(|tok| tok.span.end()).unwrap_or(start);
            TypeExpression { kind: TypeExpressionKind::Table(fields), span: SourceSpan::new(file_id, start, end) }
        } else {
            let element_type = self.parse_type_expression();
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace)) {
                self.advance();
            }
            let file_id = self.current().map(|tok| tok.span.file_id()).unwrap_or(FileId::new(0));
            TypeExpression { kind: TypeExpressionKind::Array(Box::new(element_type)), span: SourceSpan::new(file_id, start, self.current().map(|tok| tok.span.end()).unwrap_or(start)) }
        }
    }

    fn parse_variadic_type(&mut self) -> TypeExpression {
        let start = self.current().map(|tok| tok.span.start()).unwrap_or(0);
        self.advance();
        let element_type = self.parse_type_expression();
        TypeExpression { kind: TypeExpressionKind::Variadic(Box::new(element_type.clone())), span: SourceSpan::new(element_type.span.file_id(), start, element_type.span.end()) }
    }

    fn parse_named_type(&mut self) -> TypeExpression {
        if let Some(kind) = self.current().map(|tok| tok.kind.clone()) {
            let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
            let name = match kind {
                TokenKind::Identifier(name) => name,
                TokenKind::Any => "any".to_string(),
                TokenKind::Never => "never".to_string(),
                TokenKind::Nil => "nil".to_string(),
                _ => String::new(),
            };
            self.advance();
            TypeExpression { kind: TypeExpressionKind::Named(name), span }
        } else {
            TypeExpression { kind: TypeExpressionKind::Named(String::new()), span: SourceSpan::new(FileId::new(0), 0, 0) }
        }
    }

    fn parse_expression_list(&mut self) -> Vec<Expression> {
        let mut expressions = Vec::new();

        expressions.push(self.parse_expression());
        while matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
            self.advance();
            expressions.push(self.parse_expression());
        }

        expressions
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
                } else {
                    // Missing closing parenthesis
                    let span = expression.span;
                    self.emit_error_with_help("Expected ')' after expression", span, "Insert ')' here");
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
            Some(TokenKind::InterpolatedString(_)) => {
                let current = self.current().unwrap();
                if let TokenKind::InterpolatedString(raw) = current.kind.clone() {
                    let span = current.span;
                    self.advance();
                    let parts = self.parse_interpolated_string_parts(&raw);
                    Expression { kind: ExpressionKind::InterpolatedString(parts), span }
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
                // Error recovery for unexpected tokens in expression context
                let span = self.current().map(|tok| tok.span).unwrap_or(SourceSpan::new(FileId::new(0), 0, 0));
                self.emit_error("Expected expression", span);
                self.synchronize_to_expression_boundary();
                self.advance(); // Ensure we consume at least one token
                self.make_error_expression(span)
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
                Some(TokenKind::StringLiteral(_)) | Some(TokenKind::LeftBrace) | Some(TokenKind::InterpolatedString(_)) => {
                    expression = self.parse_shorthand_call(expression);
                }
                Some(TokenKind::Colon) => {
                    expression = self.parse_method_call(expression);
                }
                Some(TokenKind::Dot) => {
                    expression = self.parse_member_access(expression);
                }
                Some(TokenKind::LeftBracket) => {
                    expression = self.parse_index_expression(expression);
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
        } else {
            // Missing closing parenthesis
            let span = SourceSpan::new(file_id, start, end);
            self.emit_error_with_help("Expected ')' after argument list", span, "Insert ')' here");
        }

        Expression {
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn parse_shorthand_call(&mut self, callee: Expression) -> Expression {
        let start = callee.span.start();
        let file_id = callee.span.file_id();
        let mut arguments = Vec::new();

        // Parse the shorthand argument (string literal, table constructor, or interpolated string)
        match self.current().map(|tok| &tok.kind) {
            Some(TokenKind::StringLiteral(_)) | Some(TokenKind::InterpolatedString(_)) => {
                let arg = self.parse_prefix_expression();
                arguments.push(arg);
            }
            Some(TokenKind::LeftBrace) => {
                let arg = self.parse_table_constructor();
                arguments.push(arg);
            }
            _ => {
                // Fallback to regular expression parsing
                let arg = self.parse_expression();
                arguments.push(arg);
            }
        }

        let end = arguments.last().map(|arg| arg.span.end()).unwrap_or(start);
        Expression {
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn parse_method_call(&mut self, receiver: Expression) -> Expression {
        let start = receiver.span.start();
        let file_id = receiver.span.file_id();
        self.advance(); // consume ':'

        let method_name = if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
            self.advance();
            name
        } else {
            String::new()
        };

        let mut arguments = Vec::new();
        
        // Check for parenthesized arguments
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::LeftParen)) {
            self.advance();
            while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen) | Some(TokenKind::EOF)) {
                arguments.push(self.parse_expression());
                if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightParen)) {
                self.advance();
            }
        } else {
            // Shorthand argument (string literal, interpolated string, or table constructor)
            match self.current().map(|tok| &tok.kind) {
                Some(TokenKind::StringLiteral(_)) | Some(TokenKind::InterpolatedString(_)) => {
                    let arg = self.parse_prefix_expression();
                    arguments.push(arg);
                }
                Some(TokenKind::LeftBrace) => {
                    let arg = self.parse_table_constructor();
                    arguments.push(arg);
                }
                _ => {}
            }
        }

        let end = arguments.last().map(|arg| arg.span.end()).unwrap_or(method_name.len() + start);
        Expression {
            kind: ExpressionKind::MethodCall {
                receiver: Box::new(receiver),
                method: method_name,
                arguments,
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn parse_member_access(&mut self, object: Expression) -> Expression {
        let start = object.span.start();
        let file_id = object.span.file_id();
        self.advance(); // consume '.'

        let property = if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
            self.advance();
            name
        } else {
            String::new()
        };

        let end = self.current().map(|tok| tok.span.end()).unwrap_or(property.len() + start);
        Expression {
            kind: ExpressionKind::MemberAccess {
                object: Box::new(object),
                property,
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn parse_index_expression(&mut self, object: Expression) -> Expression {
        let start = object.span.start();
        let file_id = object.span.file_id();
        self.advance(); // consume '['

        let index = self.parse_expression();

        let end = self.current().map(|tok| tok.span.end()).unwrap_or(start);
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBracket)) {
            self.advance();
        }

        Expression {
            kind: ExpressionKind::Index {
                object: Box::new(object),
                index: Box::new(index),
            },
            span: SourceSpan::new(file_id, start, end),
        }
    }

    fn parse_table_constructor(&mut self) -> Expression {
        let start = self.current().map(|tok| tok.span.start()).unwrap_or(0);
        let file_id = self.current().map(|tok| tok.span.file_id()).unwrap_or(FileId::new(0));
        self.advance(); // consume '{'

        let mut fields = Vec::new();

        while !matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace) | Some(TokenKind::EOF)) {
            // Check for named field: Name = value
            if let Some(TokenKind::Identifier(name)) = self.current().map(|tok| tok.kind.clone()) {
                let peeked = self.peek().map(|tok| &tok.kind);
                if matches!(peeked, Some(TokenKind::Equal)) {
                    self.advance(); // consume identifier
                    self.advance(); // consume '='
                    let value = self.parse_expression();
                    fields.push(TableField::Named { key: name, value });
                } else {
                    // Could be expression field
                    let expr = self.parse_expression();
                    fields.push(TableField::Expression(expr));
                }
            }
            // Check for indexed field: [key] = value
            else if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::LeftBracket)) {
                self.advance(); // consume '['
                let key = self.parse_expression();
                if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBracket)) {
                    self.advance(); // consume ']'
                } else {
                    let span = key.span;
                    self.emit_error_with_help("Expected ']' after table index", span, "Insert ']' here");
                }
                if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Equal)) {
                    self.advance(); // consume '='
                    let value = self.parse_expression();
                    fields.push(TableField::Indexed { key, value });
                } else {
                    // Fallback: treat as expression
                    fields.push(TableField::Expression(key));
                }
            }
            // Otherwise, it's an expression field
            else {
                let expr = self.parse_expression();
                fields.push(TableField::Expression(expr));
            }

            // Check for separator
            if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::Comma) | Some(TokenKind::Semicolon)) {
                self.advance();
            } else {
                break;
            }
        }

        let end = self.current().map(|tok| tok.span.end()).unwrap_or(start);
        if matches!(self.current().map(|tok| &tok.kind), Some(TokenKind::RightBrace)) {
            self.advance();
        } else {
            // Missing closing brace
            let span = SourceSpan::new(file_id, start, end);
            self.emit_error_with_help("Expected '}' to close table constructor", span, "Insert '}' here");
        }

        Expression {
            kind: ExpressionKind::TableConstructor(fields),
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
            TokenKind::Equal => "=".to_string(),
            TokenKind::EqualEqual => "==".to_string(),
            TokenKind::NotEqual => "~=".to_string(),
            TokenKind::Less => "<".to_string(),
            TokenKind::LessEqual => "<=".to_string(),
            TokenKind::Greater => ">".to_string(),
            TokenKind::GreaterEqual => ">=".to_string(),
            TokenKind::PlusEqual => "+=".to_string(),
            TokenKind::MinusEqual => "-=".to_string(),
            TokenKind::StarEqual => "*=".to_string(),
            TokenKind::SlashEqual => "/=".to_string(),
            TokenKind::PercentEqual => "%=".to_string(),
            TokenKind::AmpersandEqual => "&=".to_string(),
            TokenKind::PipeEqual => "|=".to_string(),
            TokenKind::And => "and".to_string(),
            TokenKind::Or => "or".to_string(),
            TokenKind::DotDot => "..".to_string(),
            _ => "".to_string(),
        }
    }

    fn parse_interpolated_string_parts(&self, raw: &str) -> Vec<InterpolatedStringPart> {
        let mut parts = Vec::new();
        let mut current_text = String::new();
        let mut chars = raw.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                if !current_text.is_empty() {
                    parts.push(InterpolatedStringPart::Text(current_text.clone()));
                    current_text.clear();
                }

                // Extract the expression inside braces
                let mut expr_content = String::new();
                let mut brace_depth = 1;

                while let Some(inner_ch) = chars.next() {
                    if inner_ch == '{' {
                        brace_depth += 1;
                        expr_content.push(inner_ch);
                    } else if inner_ch == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            break;
                        }
                        expr_content.push(inner_ch);
                    } else {
                        expr_content.push(inner_ch);
                    }
                }

                // For now, create a simple identifier expression
                // In a full implementation, we'd need to re-lex and parse the expression
                parts.push(InterpolatedStringPart::Expression(Expression {
                    kind: ExpressionKind::Identifier(expr_content.trim().to_string()),
                    span: SourceSpan::new(FileId::new(0), 0, 0),
                }));
            } else {
                current_text.push(ch);
            }
        }

        if !current_text.is_empty() {
            parts.push(InterpolatedStringPart::Text(current_text));
        }

        parts
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
