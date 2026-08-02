use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Statement, StatementKind};
use crate::semantic::symbol_table::SymbolTable;

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: String,
    pub span: crate::source::SourceSpan,
}

#[derive(Debug)]
pub struct FunctionValidator {
    table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    function_metadata: Vec<FunctionMetadata>,
}

impl FunctionValidator {
    pub fn new(table: SymbolTable) -> Self {
        Self { table, diagnostics: Vec::new(), function_metadata: Vec::new() }
    }

    pub fn validate(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>, Vec<FunctionMetadata>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        (self.table, self.diagnostics, self.function_metadata)
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Function { name, receiver, params, body, .. } => {
                self.function_metadata.push(FunctionMetadata { name: name.clone(), span: statement.span });

                let mut param_names = std::collections::HashSet::new();
                for (index, (param_name, _)) in params.iter().enumerate() {
                    if !param_names.insert(param_name) {
                        self.diagnostics.push(
                            Diagnostic::error(format!("Duplicate parameter name '{param_name}'."))
                                .with_span(statement.span),
                        );
                    }

                    if param_name == "..." && index != params.len() - 1 {
                        self.diagnostics.push(
                            Diagnostic::error("Variadic parameter must be the final parameter.")
                                .with_span(statement.span),
                        );
                    }
                }

                if let Some(receiver) = receiver {
                    if receiver.is_empty() {
                        self.diagnostics.push(Diagnostic::error("Invalid method receiver.").with_span(statement.span));
                    }
                }

                self.process_statements(body);
            }
            StatementKind::Local { .. }
            | StatementKind::TypeAlias { .. }
            | StatementKind::Return(_)
            | StatementKind::Break
            | StatementKind::Continue
            | StatementKind::Assignment { .. }
            | StatementKind::Expression(_)
            | StatementKind::Empty
            | StatementKind::Error => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Program, Statement, StatementKind};
    use crate::semantic::symbol_table::SymbolTableBuilder;
    use crate::source::{FileId, SourceSpan};

    #[test]
    fn records_function_metadata_for_named_function() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Function {
                    name: "Add".to_string(),
                    receiver: None,
                    params: vec![],
                    return_type: None,
                    body: vec![],
                    is_local: false,
                },
                span: SourceSpan::new(FileId::new(0), 0, 0),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 0),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, _, metadata) = FunctionValidator::new(table).validate(&AstNode::Program(program));
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].name, "Add");
    }

    #[test]
    fn reports_duplicate_parameter_names() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Function {
                    name: "Test".to_string(),
                    receiver: None,
                    params: vec![
                        ("a".to_string(), None),
                        ("a".to_string(), None),
                    ],
                    return_type: None,
                    body: vec![],
                    is_local: false,
                },
                span: SourceSpan::new(FileId::new(0), 0, 0),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 0),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(!diagnostics.is_empty());

        let (_, diagnostics, _) = FunctionValidator::new(table).validate(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("Duplicate parameter name")));
    }
}
