pub mod name_resolution;
pub mod symbol_table;
pub mod type_resolution;

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::AstNode;

pub use symbol_table::*;
pub use name_resolution::*;
pub use type_resolution::*;

#[derive(Debug, Clone)]
pub struct SemanticAnalysisResult {
    pub table: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<ResolvedReference>,
    pub resolved_types: Vec<ResolvedType>,
}

pub fn analyze(program: &AstNode) -> SemanticAnalysisResult {
    let (table, mut diagnostics) = symbol_table::SymbolTableBuilder::new().build(program);
    let (table, name_diagnostics, references) = NameResolver::new(table).resolve(program);
    diagnostics.extend(name_diagnostics);
    let mut type_resolver = TypeResolver::new(table.clone());
    let resolved_types = type_resolver.analyze(program);

    SemanticAnalysisResult {
        table,
        diagnostics,
        references,
        resolved_types,
    }
}
