// MIR Module Structure
// =====================
// This module provides the Mid-Level Intermediate Representation (MIR)
// for the GraduaLuau compiler. The MIR is a low-level, language-independent
// representation used for optimization and backend code generation.

pub mod error;
pub mod types;
pub mod instruction;
pub mod block;
pub mod function;
pub mod module;
pub mod builder;
pub mod validator;
pub mod printer;

// Re-export commonly used types for convenience
pub use error::{MirError};
pub use types::{MirValueId, MirBlockId, MirFunctionId, MirType, MirValue};
pub use instruction::{MirInstruction, MirInstructionKind};
pub use block::MirBasicBlock;
pub use function::MirFunction;
pub use module::MirModule;
pub use builder::MirBuilder;
pub use validator::{MirValidator, MirValidationError};
pub use printer::MirPrinter;

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
