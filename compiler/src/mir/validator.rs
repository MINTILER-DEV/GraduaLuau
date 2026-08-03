use super::module::MirModule;
use super::function::MirFunction;
use super::block::MirBasicBlock;
use super::instruction::{MirInstruction, MirInstructionKind};

#[derive(Debug, Clone)]
pub struct MirValidator {
    errors: Vec<MirValidationError>,
}

#[derive(Debug, Clone)]
pub enum MirValidationError {
    UnreachableBlock {
        block_id: usize,
    },
    InvalidInstruction {
        message: String,
    },
    MissingExitPath {
        function: String,
    },
    InvalidBranchTarget {
        target: usize,
    },
}

impl MirValidator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }
    
    pub fn validate(&mut self, module: &MirModule) -> Result<(), Vec<MirValidationError>> {
        for function in &module.functions {
            self.validate_function(function);
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }
    
    fn validate_function(&mut self, function: &MirFunction) {
        // Check if function has at least one block
        if function.blocks.is_empty() {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!("Function '{}' has no blocks", function.name),
            });
            return;
        }
        
        // Check if function has an entry block
        if function.entry_block.is_none() {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!("Function '{}' has no entry block", function.name),
            });
        }
        
        // Validate each block
        for block in &function.blocks {
            self.validate_block(block, function);
        }
        
        // Check if function has an exit path
        let has_exit = function.blocks.iter().any(|block| {
            block.instructions.iter().any(|instr| matches!(instr.kind, MirInstructionKind::Return { .. }))
        });
        
        if !has_exit {
            self.errors.push(MirValidationError::MissingExitPath {
                function: function.name.clone(),
            });
        }
    }
    
    fn validate_block(&mut self, block: &MirBasicBlock, function: &MirFunction) {
        // Validate each instruction
        for instruction in &block.instructions {
            self.validate_instruction(instruction, function);
        }
    }
    
    fn validate_instruction(&mut self, instruction: &MirInstruction, function: &MirFunction) {
        match &instruction.kind {
            MirInstructionKind::Branch { true_block, false_block, .. } => {
                let true_exists = function.blocks.iter().any(|b| b.id == *true_block);
                let false_exists = function.blocks.iter().any(|b| b.id == *false_block);
                
                if !true_exists {
                    self.errors.push(MirValidationError::InvalidBranchTarget {
                        target: true_block.0,
                    });
                }
                
                if !false_exists {
                    self.errors.push(MirValidationError::InvalidBranchTarget {
                        target: false_block.0,
                    });
                }
            }
            
            MirInstructionKind::Jump { target } => {
                let exists = function.blocks.iter().any(|b| b.id == *target);
                if !exists {
                    self.errors.push(MirValidationError::InvalidBranchTarget {
                        target: target.0,
                    });
                }
            }
            
            MirInstructionKind::Error => {
                self.errors.push(MirValidationError::InvalidInstruction {
                    message: "Invalid instruction".to_string(),
                });
            }
            
            _ => {
                // Other instructions are considered valid for now
            }
        }
    }
}

impl Default for MirValidator {
    fn default() -> Self {
        Self::new()
    }
}