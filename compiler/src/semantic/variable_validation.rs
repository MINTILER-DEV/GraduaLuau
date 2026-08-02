use std::collections::{HashMap, HashSet};

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind, TableField, TypeExpression};
use crate::semantic::symbol_table::{ScopeId, SymbolId, SymbolKind, SymbolNamespace, SymbolTable};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct VariableMetadata {
    pub symbol_id: SymbolId,
    pub name: String,
    pub span: SourceSpan,
    pub declared_type: Option<TypeExpression>,
    pub initialized: bool,
    pub captured: bool,
    pub is_parameter: bool,
    pub is_loop_variable: bool,
    pub is_global: bool,
}

#[derive(Debug)]
pub struct VariableValidator {
    table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    metadata: HashMap<SymbolId, VariableMetadata>,
    scope_stack: Vec<ScopeId>,
    child_index_stack: Vec<usize>,
    initialized: HashSet<SymbolId>,
    function_depth: usize,
}

impl VariableValidator {
    pub fn new(table: SymbolTable) -> Self {
        let root_scope = table.root_scope();
        Self {
            table,
            diagnostics: Vec::new(),
            metadata: HashMap::new(),
            scope_stack: vec![root_scope],
            child_index_stack: vec![0],
            initialized: HashSet::new(),
            function_depth: 0,
        }
    }

    pub fn validate(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>, Vec<VariableMetadata>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        let metadata = self.metadata.into_values().collect();
        (self.table, self.diagnostics, metadata)
    }

    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap()
    }

    fn enter_scope(&mut self) {
        let parent = self.current_scope();
        let index = *self.child_index_stack.last().unwrap();
        let child_id = self.table.scope(parent).children[index];
        self.child_index_stack.last_mut().map(|idx| *idx += 1);
        self.scope_stack.push(child_id);
        self.child_index_stack.push(0);
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        self.child_index_stack.pop();
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { names, initializers } => {
                for initializer in initializers {
                    self.process_expression(initializer, false);
                }

                let local_scope = self.current_scope();
                for (index, (name, annotation)) in names.iter().enumerate() {
                    if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, local_scope) {
                        let symbol_id = symbol.id;
                        let is_global = symbol.declaring_scope == self.table.root_scope();
                        let initialized = initializers.get(index).is_some();
                        self.ensure_metadata(
                            symbol_id,
                            name,
                            statement.span,
                            annotation.clone(),
                            false,
                            false,
                            is_global,
                        );
                        if initialized {
                            self.initialized.insert(symbol_id);
                            if let Some(metadata) = self.metadata.get_mut(&symbol_id) {
                                metadata.initialized = true;
                            }
                        }
                    }
                }
            }
            StatementKind::Assignment { targets, values, .. } => {
                for value in values {
                    self.process_expression(value, false);
                }

                for target in targets {
                    if self.is_valid_assignment_target(target) {
                        self.bind_assignment_target(target);
                    } else {
                        self.diagnostics.push(
                            Diagnostic::error("Invalid assignment target.").with_span(target.span),
                        );
                    }
                }
            }
            StatementKind::Function { params, body, .. } => {
                self.function_depth += 1;
                self.enter_scope();

                let function_scope = self.current_scope();
                for (name, annotation) in params {
                    if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, function_scope) {
                        let symbol_id = symbol.id;
                        self.ensure_metadata(
                            symbol_id,
                            name,
                            statement.span,
                            annotation.clone(),
                            true,
                            false,
                            false,
                        );
                        self.initialized.insert(symbol_id);
                        if let Some(metadata) = self.metadata.get_mut(&symbol_id) {
                            metadata.initialized = true;
                        }
                    }
                }

                self.process_statements(body);
                self.exit_scope();
                self.function_depth -= 1;
            }
            StatementKind::TypeAlias { .. } => {}
            StatementKind::Return(values) => {
                if let Some(values) = values {
                    for value in values {
                        self.process_expression(value, false);
                    }
                }
            }
            StatementKind::Expression(expression) => {
                self.process_expression(expression, false);
            }
            StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn is_valid_assignment_target(&self, expression: &Expression) -> bool {
        matches!(expression.kind, ExpressionKind::Identifier(_)|ExpressionKind::MemberAccess { .. }|ExpressionKind::Index { .. })
    }

    fn bind_assignment_target(&mut self, expression: &Expression) {
        if let ExpressionKind::Identifier(name) = &expression.kind {
            if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, self.current_scope()) {
                self.initialized.insert(symbol.id);
                if let Some(metadata) = self.metadata.get_mut(&symbol.id) {
                    metadata.initialized = true;
                }
            }
        }
    }

    fn process_expression(&mut self, expression: &Expression, is_lhs: bool) {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                let current_scope = self.current_scope();
                if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, current_scope) {
                    let symbol_id = symbol.id;
                    let declaring_scope = symbol.declaring_scope;
                    let declared_type = symbol.declared_type.clone();

                    if !is_lhs
                        && matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter)
                        && !self.initialized.contains(&symbol_id)
                    {
                        self.diagnostics.push(
                            Diagnostic::error(format!("Variable '{}' may be used before initialization.", name))
                                .with_span(expression.span),
                        );
                    }

                    if self.function_depth > 0
                        && declaring_scope != current_scope
                        && matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter)
                    {
                        self.ensure_metadata(
                            symbol_id,
                            name,
                            expression.span,
                            declared_type,
                            symbol.kind == SymbolKind::Parameter,
                            false,
                            declaring_scope == self.table.root_scope(),
                        );
                        if let Some(metadata) = self.metadata.get_mut(&symbol_id) {
                            metadata.captured = true;
                        }
                    }
                }
            }
            ExpressionKind::Unary { operand, .. } => self.process_expression(operand, false),
            ExpressionKind::Binary { left, right, .. } => {
                self.process_expression(left, false);
                self.process_expression(right, false);
            }
            ExpressionKind::Call { callee, arguments } => {
                self.process_expression(callee, false);
                for argument in arguments {
                    self.process_expression(argument, false);
                }
            }
            ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        TableField::Named { value, .. } => self.process_expression(value, false),
                        TableField::Indexed { key, value } => {
                            self.process_expression(key, false);
                            self.process_expression(value, false);
                        }
                        TableField::Expression(expr) => self.process_expression(expr, false),
                    }
                }
            }
            ExpressionKind::MemberAccess { object, .. } => self.process_expression(object, false),
            ExpressionKind::Index { object, index } => {
                self.process_expression(object, false);
                self.process_expression(index, false);
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.process_expression(receiver, false);
                for argument in arguments {
                    self.process_expression(argument, false);
                }
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.process_expression(expr, false);
                    }
                }
            }
            ExpressionKind::NumberLiteral(_) | ExpressionKind::StringLiteral(_) | ExpressionKind::BooleanLiteral(_) | ExpressionKind::Nil | ExpressionKind::Error => {}
        }
    }

    fn ensure_metadata(
        &mut self,
        symbol_id: SymbolId,
        name: &str,
        span: SourceSpan,
        declared_type: Option<TypeExpression>,
        is_parameter: bool,
        is_loop_variable: bool,
        is_global: bool,
    ) {
        self.metadata.entry(symbol_id).or_insert(VariableMetadata {
            symbol_id,
            name: name.to_string(),
            span,
            declared_type,
            initialized: false,
            captured: false,
            is_parameter,
            is_loop_variable,
            is_global,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Program, Statement, StatementKind, TypeExpression, TypeExpressionKind};
    use crate::semantic::symbol_table::SymbolTableBuilder;
    use crate::source::{FileId, SourceSpan};

    #[test]
    fn reports_use_before_initialization() {
        let program = Program {
            statements: vec![
                Statement {
                    kind: StatementKind::Local {
                        names: vec![("x".to_string(), Some(TypeExpression {
                            kind: TypeExpressionKind::Named("number".to_string()),
                            span: SourceSpan::new(FileId::new(0), 0, 6),
                        }))],
                        initializers: vec![],
                    },
                    span: SourceSpan::new(FileId::new(0), 0, 6),
                },
                Statement {
                    kind: StatementKind::Expression(Expression {
                        kind: ExpressionKind::Identifier("x".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 7),
                    }),
                    span: SourceSpan::new(FileId::new(0), 0, 7),
                },
            ],
            span: SourceSpan::new(FileId::new(0), 0, 7),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, _) = VariableValidator::new(table).validate(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("used before initialization")));
    }

    #[test]
    fn reports_invalid_assignment_target() {
        let program = Program {
            statements: vec![
                Statement {
                    kind: StatementKind::Assignment {
                        targets: vec![Expression {
                            kind: ExpressionKind::NumberLiteral("5".to_string()),
                            span: SourceSpan::new(FileId::new(0), 0, 1),
                        }],
                        values: vec![Expression {
                            kind: ExpressionKind::Identifier("x".to_string()),
                            span: SourceSpan::new(FileId::new(0), 0, 3),
                        }],
                        operator: "=".to_string(),
                    },
                    span: SourceSpan::new(FileId::new(0), 0, 3),
                },
            ],
            span: SourceSpan::new(FileId::new(0), 0, 3),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, _) = VariableValidator::new(table).validate(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("Invalid assignment target")));
    }

    #[test]
    fn records_captured_variable() {
        let program = Program {
            statements: vec![
                Statement {
                    kind: StatementKind::Local {
                        names: vec![("x".to_string(), None)],
                        initializers: vec![Expression {
                            kind: ExpressionKind::NumberLiteral("0".to_string()),
                            span: SourceSpan::new(FileId::new(0), 0, 1),
                        }],
                    },
                    span: SourceSpan::new(FileId::new(0), 0, 1),
                },
                Statement {
                    kind: StatementKind::Function {
                        name: "Inner".to_string(),
                        receiver: None,
                        params: vec![],
                        return_type: None,
                        body: vec![Statement {
                            kind: StatementKind::Return(Some(vec![Expression {
                                kind: ExpressionKind::Identifier("x".to_string()),
                                span: SourceSpan::new(FileId::new(0), 0, 2),
                            }])),
                            span: SourceSpan::new(FileId::new(0), 0, 2),
                        }],
                        is_local: false,
                    },
                    span: SourceSpan::new(FileId::new(0), 0, 2),
                },
            ],
            span: SourceSpan::new(FileId::new(0), 0, 2),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, metadata) = VariableValidator::new(table).validate(&AstNode::Program(program));
        assert!(diagnostics.is_empty());
        assert!(metadata.iter().any(|entry| entry.name == "x" && entry.captured));
    }
}
