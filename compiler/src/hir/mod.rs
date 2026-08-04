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
pub mod optimizer;
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
pub use optimizer::{HirOptimizationResult, HirOptimizationStats, HirOptimizer};
pub use printer::HirPrinter;
pub use statement::{HirLocalVariable, HirStatement, HirStatementKind};
pub use symbol::{HirScope, HirScopeKind, HirSymbol, HirSymbolKind};
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
        Self::lower_with_optimization(ast).map(|result| result.module)
    }

    pub fn lower_unoptimized(ast: &AstNode) -> Result<HirModule, HirError> {
        let mut builder = HirBuilder::new();
        builder.build(ast)
    }

    pub fn lower_with_optimization(ast: &AstNode) -> Result<HirOptimizationResult, HirError> {
        let module = Self::lower_unoptimized(ast)?;
        // Optimize and validate the generated HIR before MIR lowering.
        let mut optimizer = HirOptimizer::new();
        let optimized = optimizer.optimize(&module);

        let mut validator = HirValidator::new();
        if let Err(validation_errors) = validator.validate(&optimized.module) {
            return Err(HirError::LoweringError(format!(
                "HIR validation failed with {} errors",
                validation_errors.len()
            )));
        }

        Ok(optimized)
    }
}
