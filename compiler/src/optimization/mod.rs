// Optimization Module Structure
// =====================
// This module provides the optimization pipeline for the GraduaLuau compiler.
// It performs optimization passes on LLVM IR to improve efficiency while preserving behavior.

pub mod error;
pub mod config;
pub mod passes;

// Re-export commonly used types for convenience
pub use error::{OptimizationError};
pub use config::{OptimizationLevel};
pub use passes::OptimizationPasses;

use crate::llvm::LlvmModule;

#[derive(Debug)]
pub struct OptimizationStage {
    level: OptimizationLevel,
}

impl OptimizationStage {
    pub fn new(level: OptimizationLevel) -> Self {
        Self { level }
    }
    
    pub fn optimize(&self, llvm_module: &LlvmModule) -> Result<LlvmModule, OptimizationError> {
        let passes = OptimizationPasses::new(self.level);
        let optimized_ir = passes.run(&llvm_module.ir)?;
        
        Ok(LlvmModule {
            ir: optimized_ir,
        })
    }
}

impl Default for OptimizationStage {
    fn default() -> Self {
        Self::new(OptimizationLevel::default())
    }
}