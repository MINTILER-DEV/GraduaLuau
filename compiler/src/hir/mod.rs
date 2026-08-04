// HIR Module Structure
// =====================
// This module provides the High-Level Intermediate Representation (HIR)
// for the GraduaLuau compiler. The HIR is a clean, language-independent
// intermediate representation after semantic analysis.

pub mod builder;
pub mod error;
pub mod expression;
pub mod function;
pub mod ids;
pub mod module;
pub mod printer;
pub mod statement;
pub mod symbol;
pub mod types;
pub mod validator;

// Re-export commonly used types for convenience
pub use builder::HirBuilder;
pub use error::HirError;
pub use expression::{HirExpression, HirExpressionKind, HirInterpolatedStringPart, HirTableField};
pub use function::{HirFunction, HirParameter};
pub use ids::{HirFunctionId, HirScopeId, HirSymbolId, HirVariableId};
pub use module::{HirGlobalVariable, HirModule, HirTypeAlias};
pub use printer::HirPrinter;
pub use statement::{HirLocalVariable, HirStatement, HirStatementKind};
pub use symbol::{HirScope, HirSymbol, HirSymbolKind};
pub use types::{
    HirBinaryOperator, HirBuiltinFunction, HirCallingConvention, HirFunctionSignature, HirType,
    HirUnaryOperator,
};
pub use validator::{HirValidationError, HirValidator};

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
