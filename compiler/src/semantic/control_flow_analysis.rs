use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind, TableField, TypeExpression};
use crate::semantic::symbol_table::SymbolTable;
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct ControlFlowMetadata {
    pub function_name: String,
    pub span: SourceSpan,
    pub has_return: bool,
    pub all_paths_return: bool,
    pub unreachable_spans: Vec<SourceSpan>,
    pub invalid_breaks: Vec<SourceSpan>,
    pub invalid_continues: Vec<SourceSpan>,
}

#[derive(Debug)]
pub struct ControlFlowAnalyzer {
    table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    metadata: Vec<ControlFlowMetadata>,
}

impl ControlFlowAnalyzer {
    pub fn new(table: SymbolTable) -> Self {
        Self {
            table,
            diagnostics: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn analyze(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>, Vec<ControlFlowMetadata>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        (self.table, self.diagnostics, self.metadata)
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Function { name, return_type, body, .. } => {
                let mut function_metadata = ControlFlowMetadata {
                    function_name: name.clone(),
                    span: statement.span,
                    has_return: false,
                    all_paths_return: false,
                    unreachable_spans: Vec::new(),
                    invalid_breaks: Vec::new(),
                    invalid_continues: Vec::new(),
                };

                let mut analyzer = FunctionFlowAnalyzer::new(return_type.clone());
                analyzer.process_statements(body);

                if analyzer.needs_explicit_return() && !analyzer.all_paths_return() {
                    analyzer.diagnostics.push(
                        Diagnostic::error("Not all execution paths return a value.").with_span(statement.span),
                    );
                }

                function_metadata.has_return = analyzer.has_return;
                function_metadata.all_paths_return = analyzer.all_paths_return();
                function_metadata.unreachable_spans = analyzer.unreachable_spans;
                function_metadata.invalid_breaks = analyzer.invalid_breaks;
                function_metadata.invalid_continues = analyzer.invalid_continues;

                self.diagnostics.extend(analyzer.diagnostics);
                self.metadata.push(function_metadata);
            }
            _ => {}
        }
    }
}

struct FunctionFlowAnalyzer {
    return_type: Option<TypeExpression>,
    has_return: bool,
    current_terminated: bool,
    unreachable_spans: Vec<SourceSpan>,
    invalid_breaks: Vec<SourceSpan>,
    invalid_continues: Vec<SourceSpan>,
    diagnostics: Vec<Diagnostic>,
    loop_depth: usize,
}

impl FunctionFlowAnalyzer {
    fn new(return_type: Option<TypeExpression>) -> Self {
        Self {
            return_type,
            has_return: false,
            current_terminated: false,
            unreachable_spans: Vec::new(),
            invalid_breaks: Vec::new(),
            invalid_continues: Vec::new(),
            diagnostics: Vec::new(),
            loop_depth: 0,
        }
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            if self.current_terminated {
                self.unreachable_spans.push(statement.span);
                self.diagnostics.push(
                    Diagnostic::error("Unreachable code.").with_span(statement.span),
                );
                continue;
            }
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Return(values) => {
                self.has_return = true;
                self.current_terminated = true;
                if let Some(values) = values {
                    for value in values {
                        self.process_expression(value);
                    }
                }
            }
            StatementKind::Break => {
                if self.loop_depth == 0 {
                    self.invalid_breaks.push(statement.span);
                    self.diagnostics.push(
                        Diagnostic::error("'break' may only appear inside a loop.").with_span(statement.span),
                    );
                }
                self.current_terminated = true;
            }
            StatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.invalid_continues.push(statement.span);
                    self.diagnostics.push(
                        Diagnostic::error("'continue' may only appear inside a loop.").with_span(statement.span),
                    );
                }
                self.current_terminated = true;
            }
            StatementKind::Local { names: _, initializers } => {
                for initializer in initializers {
                    self.process_expression(initializer);
                }
            }
            StatementKind::Assignment { targets, values, .. } => {
                for target in targets {
                    self.process_expression(target);
                }
                for value in values {
                    self.process_expression(value);
                }
            }
            StatementKind::Expression(expression) => {
                self.process_expression(expression);
            }
            StatementKind::Function { .. } => {
                // Nested functions are handled independently by outer analyzer.
            }
            StatementKind::TypeAlias { .. } => {}
            StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn process_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(_) => {}
            ExpressionKind::Unary { operand, .. } => self.process_expression(operand),
            ExpressionKind::Binary { left, right, .. } => {
                self.process_expression(left);
                self.process_expression(right);
            }
            ExpressionKind::Call { callee, arguments } => {
                self.process_expression(callee);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        TableField::Named { value, .. } => self.process_expression(value),
                        TableField::Indexed { key, value } => {
                            self.process_expression(key);
                            self.process_expression(value);
                        }
                        TableField::Expression(expr) => self.process_expression(expr),
                    }
                }
            }
            ExpressionKind::MemberAccess { object, .. } => self.process_expression(object),
            ExpressionKind::Index { object, index } => {
                self.process_expression(object);
                self.process_expression(index);
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.process_expression(receiver);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.process_expression(expr);
                    }
                }
            }
            ExpressionKind::NumberLiteral(_) | ExpressionKind::StringLiteral(_) | ExpressionKind::BooleanLiteral(_) | ExpressionKind::Nil | ExpressionKind::Error => {}
        }
    }

    fn needs_explicit_return(&self) -> bool {
        self.return_type.is_some()
    }

    fn all_paths_return(&self) -> bool {
        self.current_terminated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Program, Statement, StatementKind, TypeExpression, TypeExpressionKind};
    use crate::semantic::symbol_table::SymbolTableBuilder;
    use crate::source::{FileId, SourceSpan};

    #[test]
    fn reports_unreachable_code_after_return() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Function {
                    name: "Test".to_string(),
                    receiver: None,
                    params: vec![],
                    return_type: Some(TypeExpression {
                        kind: TypeExpressionKind::Named("number".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 6),
                    }),
                    body: vec![
                        Statement {
                            kind: StatementKind::Return(Some(vec![Expression {
                                kind: ExpressionKind::NumberLiteral("1".to_string()),
                                span: SourceSpan::new(FileId::new(0), 0, 7),
                            }])),
                            span: SourceSpan::new(FileId::new(0), 0, 7),
                        },
                        Statement {
                            kind: StatementKind::Expression(Expression {
                                kind: ExpressionKind::Identifier("x".to_string()),
                                span: SourceSpan::new(FileId::new(0), 0, 8),
                            }),
                            span: SourceSpan::new(FileId::new(0), 0, 8),
                        },
                    ],
                    is_local: false,
                },
                span: SourceSpan::new(FileId::new(0), 0, 7),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 8),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, metadata) = ControlFlowAnalyzer::new(table).analyze(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("Unreachable code")));
        assert_eq!(metadata.len(), 1);
    }

    #[test]
    fn reports_break_outside_loop() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Function {
                    name: "Test".to_string(),
                    receiver: None,
                    params: vec![],
                    return_type: None,
                    body: vec![Statement {
                        kind: StatementKind::Break,
                        span: SourceSpan::new(FileId::new(0), 0, 1),
                    }],
                    is_local: false,
                },
                span: SourceSpan::new(FileId::new(0), 0, 1),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 1),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, _) = ControlFlowAnalyzer::new(table).analyze(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("may only appear inside a loop")));
    }

    #[test]
    fn reports_missing_return_for_non_nil_function() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Function {
                    name: "Test".to_string(),
                    receiver: None,
                    params: vec![],
                    return_type: Some(TypeExpression {
                        kind: TypeExpressionKind::Named("number".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 6),
                    }),
                    body: vec![Statement {
                        kind: StatementKind::Expression(Expression {
                            kind: ExpressionKind::NumberLiteral("1".to_string()),
                            span: SourceSpan::new(FileId::new(0), 0, 7),
                        }),
                        span: SourceSpan::new(FileId::new(0), 0, 7),
                    }],
                    is_local: false,
                },
                span: SourceSpan::new(FileId::new(0), 0, 7),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 7),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, metadata) = ControlFlowAnalyzer::new(table).analyze(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("Not all execution paths return")));
        assert_eq!(metadata[0].all_paths_return, false);
    }
}
