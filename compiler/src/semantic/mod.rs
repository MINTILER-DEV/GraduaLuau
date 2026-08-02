pub mod control_flow_analysis;
pub mod function_validation;
pub mod name_resolution;
pub mod symbol_table;
pub mod type_checking;
pub mod type_resolution;
pub mod variable_validation;

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::AstNode;

pub use control_flow_analysis::*;
pub use function_validation::*;
pub use symbol_table::*;
pub use name_resolution::*;
pub use type_checking::*;
pub use type_resolution::*;
pub use variable_validation::*;

#[derive(Debug, Clone)]
pub struct SemanticAnalysisResult {
    pub table: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
    pub references: Vec<ResolvedReference>,
    pub resolved_types: Vec<ResolvedType>,
    pub function_metadata: Vec<FunctionMetadata>,
    pub variable_metadata: Vec<VariableMetadata>,
    pub control_flow_metadata: Vec<ControlFlowMetadata>,
}

pub fn analyze(program: &AstNode) -> SemanticAnalysisResult {
    let (table, mut diagnostics) = symbol_table::SymbolTableBuilder::new().build(program);
    let (table, name_diagnostics, references) = NameResolver::new(table).resolve(program);
    diagnostics.extend(name_diagnostics);

    let resolved_types = TypeResolver::new(table.clone()).analyze(program);

    let (table, type_diagnostics) = TypeChecker::new(table).check(program);
    diagnostics.extend(type_diagnostics);

    let (table, function_diagnostics, function_metadata) = FunctionValidator::new(table).validate(program);
    diagnostics.extend(function_diagnostics);

    let (table, variable_diagnostics, variable_metadata) = VariableValidator::new(table).validate(program);
    diagnostics.extend(variable_diagnostics);

    let (_table, control_flow_diagnostics, control_flow_metadata) = ControlFlowAnalyzer::new(table.clone()).analyze(program);
    diagnostics.extend(control_flow_diagnostics);

    SemanticAnalysisResult {
        table,
        diagnostics,
        references,
        resolved_types,
        function_metadata,
        variable_metadata,
        control_flow_metadata,
    }
}
