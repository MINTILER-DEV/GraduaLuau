use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind, TableField, TypeExpression};
use crate::semantic::symbol_table::{ScopeId, SymbolId, SymbolNamespace, SymbolTable};
use crate::source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReference {
    pub symbol_id: SymbolId,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub struct NameResolver {
    table: SymbolTable,
    scope_stack: Vec<ScopeId>,
    child_index_stack: Vec<usize>,
    diagnostics: Vec<Diagnostic>,
    references: Vec<ResolvedReference>,
}

impl NameResolver {
    pub fn new(table: SymbolTable) -> Self {
        let root_scope = table.root_scope();
        Self { table, scope_stack: vec![root_scope], child_index_stack: vec![0], diagnostics: Vec::new(), references: Vec::new() }
    }

    pub fn resolve(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>, Vec<ResolvedReference>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        (self.table, self.diagnostics, self.references)
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

    fn resolve_identifier(&mut self, name: &str, span: SourceSpan, namespace: SymbolNamespace) -> Option<super::symbol_table::SymbolId> {
        let current_scope = self.current_scope();
        if let Some(symbol) = self.table.lookup(name, namespace, current_scope) {
            let id = symbol.id;
            self.references.push(ResolvedReference { symbol_id: id, span });
            Some(id)
        } else {
            self.diagnostics.push(
                Diagnostic::error(format!("Undefined {} '{}'.", match namespace {
                    SymbolNamespace::Value => "name",
                    SymbolNamespace::Type => "type",
                }, name))
                .with_span(span),
            );
            None
        }
    }

    fn process_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                self.resolve_identifier(name, expression.span, SymbolNamespace::Value);
            }
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

    fn process_type_expression(&mut self, type_expression: &TypeExpression) {
        use crate::parser::ast_builder::TypeExpressionKind::*;

        match &type_expression.kind {
            Named(name) => {
                self.resolve_identifier(name, type_expression.span, SymbolNamespace::Type);
            }
            Optional(inner)
            | Array(inner)
            | Variadic(inner)
            | Parenthesized(inner) => self.process_type_expression(inner),
            Union(types) | Intersection(types) | Tuple(types) => {
                for typ in types {
                    self.process_type_expression(typ);
                }
            }
            Function { params, return_type } => {
                for param in params {
                    self.process_type_expression(param);
                }
                self.process_type_expression(return_type);
            }
            Table(fields) => {
                for (_, field_type, _) in fields {
                    self.process_type_expression(field_type);
                }
            }
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { names, initializers } => {
                for initializer in initializers {
                    self.process_expression(initializer);
                }
                for (_, annotation) in names {
                    if let Some(annotation) = annotation {
                        self.process_type_expression(annotation);
                    }
                }
            }
            StatementKind::Function { params, return_type, body, .. } => {
                for (_, annotation) in params {
                    if let Some(annotation) = annotation {
                        self.process_type_expression(annotation);
                    }
                }
                if let Some(return_type) = return_type {
                    self.process_type_expression(return_type);
                }
                self.enter_scope();
                for statement in body {
                    self.process_statement(statement);
                }
                self.exit_scope();
            }
            StatementKind::TypeAlias { alias, .. } => {
                self.process_type_expression(alias);
            }
            StatementKind::Return(values) => {
                if let Some(values) = values {
                    for value in values {
                        self.process_expression(value);
                    }
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
            StatementKind::Expression(expression) => self.process_expression(expression),
            StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Program, Statement, StatementKind, TypeExpression, TypeExpressionKind};
    use crate::source::{FileId, SourceSpan};
    use crate::semantic::symbol_table::SymbolTableBuilder;

    #[test]
    fn resolves_local_reference_in_root_scope() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Local {
                    names: vec![("x".to_string(), None)],
                    initializers: vec![Expression {
                        kind: ExpressionKind::Identifier("print".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 5),
                    }],
                },
                span: SourceSpan::new(FileId::new(0), 0, 5),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 5),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, references) = NameResolver::new(table).resolve(&AstNode::Program(program));
        assert!(diagnostics.is_empty());
        assert_eq!(references.len(), 1);
    }

    #[test]
    fn reports_missing_type_reference() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::TypeAlias {
                    name: "T".to_string(),
                    alias: TypeExpression {
                        kind: TypeExpressionKind::Named("MissingType".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 11),
                    },
                },
                span: SourceSpan::new(FileId::new(0), 0, 11),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 11),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics, _) = NameResolver::new(table).resolve(&AstNode::Program(program));
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message().contains("Undefined type 'MissingType'"));
    }
}
