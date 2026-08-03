// HIR Module Structure
// =====================
// This module provides the High-Level Intermediate Representation (HIR)
// for the GraduaLuau compiler. The HIR is a clean, language-independent
// intermediate representation after semantic analysis.

pub mod error;
pub mod types;
pub mod ids;
pub mod module;
pub mod statement;
pub mod expression;
pub mod function;
pub mod builder;
pub mod validator;
pub mod printer;

// Re-export commonly used types for convenience
pub use error::{HirError};
pub use types::{HirType, HirUnaryOperator, HirBinaryOperator, HirBuiltinFunction};
pub use ids::{HirFunctionId, HirVariableId};
pub use module::{HirModule, HirGlobalVariable};
pub use statement::{HirStatement, HirStatementKind, HirLocalVariable};
pub use expression::{HirExpression, HirExpressionKind, HirTableField};
pub use function::{HirFunction, HirParameter};
pub use builder::HirBuilder;
pub use validator::{HirValidator, HirValidationError};
pub use printer::HirPrinter;

use crate::parser::ast_builder::AstNode;

#[derive(Debug, Default)]
pub struct HirStage;

impl HirStage {
    pub fn lower(ast: &AstNode) -> Result<HirModule, HirError> {
        let mut builder = HirBuilder::new();
        let module = builder.build(ast)?;
        
        // Validate the generated HIR
        let mut validator = HirValidator::new();
        if let Err(validation_errors) = validator.validate(&module) {
            return Err(HirError::LoweringError(format!(
                "HIR validation failed with {} errors",
                validation_errors.len()
            )));
        }
        
        Ok(module)
    }
}