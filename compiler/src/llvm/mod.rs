// LLVM Module Structure
// =====================
// This module provides the LLVM IR backend for the GraduaLuau compiler.
// It translates MIR into LLVM Intermediate Representation for code generation.

pub mod error;
pub mod types;
pub mod generator;
pub mod verifier;

// Re-export commonly used types for convenience
pub use error::{LlvmError};
pub use types::{LlvmType, map_mir_type};
pub use generator::LlvmGenerator;
pub use verifier::LlvmVerifier;

use crate::mir::MirModule;

#[derive(Debug, Clone)]
pub struct LlvmModule {
    pub ir: String,
}

#[derive(Debug, Default)]
pub struct LlvmStage;

impl LlvmStage {
    pub fn generate(mir: &MirModule) -> Result<LlvmModule, LlvmError> {
        let mut generator = LlvmGenerator::new(mir.name.clone());
        let ir = generator.generate(mir)?;
        
        // Verify the generated LLVM IR
        LlvmVerifier::verify(&ir)?;
        
        Ok(LlvmModule { ir })
    }
}
