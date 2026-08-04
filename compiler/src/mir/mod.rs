// MIR Module Structure
// =====================
// This module provides the Mid-Level Intermediate Representation (MIR)
// for the GraduaLuau compiler. The MIR is a low-level, language-independent
// representation used for optimization and backend code generation.

pub mod block;
pub mod builder;
pub mod cfg;
pub mod error;
pub mod function;
pub mod instruction;
pub mod module;
pub mod operand;
pub mod printer;
pub mod types;
pub mod validator;
pub mod value;

// Re-export commonly used types for convenience
pub use block::MirBasicBlock;
pub use builder::MirBuilder;
pub use cfg::{
    MirCfgEdge, MirCfgEdgeKind, MirCfgNode, MirCfgValidationError, MirControlFlowGraph, MirLoop,
};
pub use error::MirError;
pub use function::MirFunction;
pub use instruction::{MirInstruction, MirInstructionKind, MirTerminator};
pub use module::MirModule;
pub use operand::MirOperand;
pub use printer::MirPrinter;
pub use types::{MirBlockId, MirCompareOperator, MirFunctionId, MirType, MirValue, MirValueId};
pub use validator::{MirValidationError, MirValidator};
pub use value::{MirConstant, MirGlobal, MirLocal, MirParameter, MirValueData, MirValueKind};

use crate::hir::HirModule;

#[derive(Debug, Default)]
pub struct MirStage;

impl MirStage {
    pub fn lower(hir: &HirModule) -> Result<MirModule, MirError> {
        let mut builder = MirBuilder::new();
        let mir_module = builder.build(hir)?;

        // Validate the generated MIR
        let mut validator = MirValidator::new();
        if let Err(validation_errors) = validator.validate(&mir_module) {
            return Err(MirError::ValidationError(format!(
                "MIR validation failed with {} errors: {:?}",
                validation_errors.len(),
                validation_errors
            )));
        }

        Ok(mir_module)
    }
}
