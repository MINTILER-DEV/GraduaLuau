use super::block::MirBasicBlock;
use super::function::MirFunction;
use super::instruction::{MirInstruction, MirInstructionKind};
use super::module::MirModule;
use super::types::{MirType, MirValueId};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct MirValidator {
    errors: Vec<MirValidationError>,
}

#[derive(Debug, Clone)]
pub enum MirValidationError {
    UnreachableBlock { block_id: usize },
    InvalidInstruction { message: String },
    MissingExitPath { function: String },
    MissingTerminator { block_id: usize },
    InvalidTerminator { block_id: usize, message: String },
    InvalidBranchTarget { target: usize },
    InvalidValue { value_id: usize, message: String },
}

impl MirValidator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
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

        let mut value_ids = HashSet::new();
        for value in &function.values {
            if !value_ids.insert(value.id) {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: value.id.0,
                    message: "duplicate MIR value id".to_string(),
                });
            }

            if value.value_type == MirType::Unknown {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: value.id.0,
                    message: "MIR value has unknown type".to_string(),
                });
            }
        }

        for block in &function.blocks {
            self.validate_block(block, function, &value_ids);
        }

        self.validate_cfg(function);

        let has_exit = function.blocks.iter().any(|block| {
            matches!(
                block.terminator,
                Some(super::instruction::MirTerminator::Return { .. })
            )
        });

        if !has_exit {
            self.errors.push(MirValidationError::MissingExitPath {
                function: function.name.clone(),
            });
        }
    }

    fn validate_cfg(&mut self, function: &MirFunction) {
        let Some(cfg) = function.cfg.as_ref() else {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!("Function '{}' has no CFG", function.name),
            });
            return;
        };

        if let Err(cfg_errors) = cfg.validate() {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!(
                    "Function '{}' has invalid CFG: {:?}",
                    function.name, cfg_errors
                ),
            });
        }

        for block_id in cfg.unreachable_blocks() {
            self.errors.push(MirValidationError::UnreachableBlock {
                block_id: block_id.0,
            });
        }

        for block in &function.blocks {
            if let Some(node) = cfg.nodes.get(&block.id) {
                if node.successors != block.successors {
                    self.errors.push(MirValidationError::InvalidInstruction {
                        message: format!(
                            "Block {} CFG successors do not match block successors",
                            block.id.0
                        ),
                    });
                }

                let mut node_predecessors = node.predecessors.clone();
                let mut block_predecessors = block.predecessors.clone();
                node_predecessors.sort_by_key(|block_id| block_id.0);
                block_predecessors.sort_by_key(|block_id| block_id.0);

                if node_predecessors != block_predecessors {
                    self.errors.push(MirValidationError::InvalidInstruction {
                        message: format!(
                            "Block {} CFG predecessors do not match block predecessors",
                            block.id.0
                        ),
                    });
                }
            } else {
                self.errors.push(MirValidationError::InvalidInstruction {
                    message: format!("Block {} is missing from CFG", block.id.0),
                });
            }
        }

        if let Some(entry_block_index) = function.entry_block {
            if let Some(entry_block) = function.blocks.get(entry_block_index) {
                if cfg.entry != entry_block.id {
                    self.errors.push(MirValidationError::InvalidInstruction {
                        message: format!("Function '{}' CFG entry mismatch", function.name),
                    });
                }

                if !entry_block.predecessors.is_empty() {
                    self.errors.push(MirValidationError::InvalidInstruction {
                        message: format!(
                            "Function '{}' entry block has predecessors",
                            function.name
                        ),
                    });
                }
            }
        }
    }

    fn validate_block(
        &mut self,
        block: &MirBasicBlock,
        function: &MirFunction,
        value_ids: &HashSet<MirValueId>,
    ) {
        if block.terminator.is_none() {
            self.errors.push(MirValidationError::MissingTerminator {
                block_id: block.id.0,
            });
        }

        let terminator_positions: Vec<usize> = block
            .instructions
            .iter()
            .enumerate()
            .filter_map(|(index, instruction)| instruction.is_terminator().then_some(index))
            .collect();

        match terminator_positions.as_slice() {
            [] => {}
            [last] if *last == block.instructions.len().saturating_sub(1) => {}
            [position] => self.errors.push(MirValidationError::InvalidTerminator {
                block_id: block.id.0,
                message: format!("terminator at instruction {position} is not last"),
            }),
            _ => self.errors.push(MirValidationError::InvalidTerminator {
                block_id: block.id.0,
                message: "block has multiple terminators".to_string(),
            }),
        }

        if let (Some(recorded), Some(last)) = (
            block.terminator.as_ref(),
            block
                .instructions
                .last()
                .and_then(|instruction| instruction.terminator()),
        ) {
            if recorded != &last {
                self.errors.push(MirValidationError::InvalidTerminator {
                    block_id: block.id.0,
                    message: "recorded terminator does not match final instruction".to_string(),
                });
            }
        }

        for instruction in &block.instructions {
            self.validate_instruction(instruction, function, value_ids);
        }
    }

    fn validate_instruction(
        &mut self,
        instruction: &MirInstruction,
        function: &MirFunction,
        value_ids: &HashSet<MirValueId>,
    ) {
        match &instruction.kind {
            MirInstructionKind::Branch {
                condition,
                true_block,
                false_block,
            } => {
                self.validate_value(*condition, value_ids, "branch condition");
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
                    self.errors
                        .push(MirValidationError::InvalidBranchTarget { target: target.0 });
                }
            }

            MirInstructionKind::Return { value } => {
                if let Some(value) = value {
                    self.validate_value(*value, value_ids, "return value");
                }
            }

            MirInstructionKind::Const { result, .. }
            | MirInstructionKind::Load { result, .. }
            | MirInstructionKind::TableNew { result } => {
                self.validate_value(*result, value_ids, "instruction result");
            }

            MirInstructionKind::Add {
                result,
                left,
                right,
            }
            | MirInstructionKind::Subtract {
                result,
                left,
                right,
            }
            | MirInstructionKind::Multiply {
                result,
                left,
                right,
            }
            | MirInstructionKind::Divide {
                result,
                left,
                right,
            }
            | MirInstructionKind::Modulo {
                result,
                left,
                right,
            }
            | MirInstructionKind::Equal {
                result,
                left,
                right,
            }
            | MirInstructionKind::NotEqual {
                result,
                left,
                right,
            }
            | MirInstructionKind::LessThan {
                result,
                left,
                right,
            }
            | MirInstructionKind::LessEqual {
                result,
                left,
                right,
            }
            | MirInstructionKind::GreaterThan {
                result,
                left,
                right,
            }
            | MirInstructionKind::GreaterEqual {
                result,
                left,
                right,
            }
            | MirInstructionKind::And {
                result,
                left,
                right,
            }
            | MirInstructionKind::Or {
                result,
                left,
                right,
            }
            | MirInstructionKind::Compare {
                result,
                left,
                right,
                ..
            } => {
                self.validate_value(*result, value_ids, "instruction result");
                self.validate_value(*left, value_ids, "left operand");
                self.validate_value(*right, value_ids, "right operand");
            }

            MirInstructionKind::Not { result, operand } => {
                self.validate_value(*result, value_ids, "instruction result");
                self.validate_value(*operand, value_ids, "unary operand");
            }

            MirInstructionKind::Store { value, .. } => {
                self.validate_value(*value, value_ids, "store value");
            }

            MirInstructionKind::Move { result, value } => {
                self.validate_value(*result, value_ids, "move result");
                self.validate_value(*value, value_ids, "move value");
            }

            MirInstructionKind::AllocateLocal { local, .. } => {
                self.validate_value(*local, value_ids, "local allocation");
            }

            MirInstructionKind::Call {
                result, arguments, ..
            } => {
                if let Some(result) = result {
                    self.validate_value(*result, value_ids, "call result");
                }
                for argument in arguments {
                    self.validate_value(*argument, value_ids, "call argument");
                }
            }

            MirInstructionKind::TableSet { table, key, value } => {
                self.validate_value(*table, value_ids, "table value");
                self.validate_value(*key, value_ids, "table key");
                self.validate_value(*value, value_ids, "table field value");
            }

            MirInstructionKind::TableGet { result, table, key } => {
                self.validate_value(*result, value_ids, "table get result");
                self.validate_value(*table, value_ids, "table value");
                self.validate_value(*key, value_ids, "table key");
            }

            MirInstructionKind::Unreachable => {}

            MirInstructionKind::Error => {
                self.errors.push(MirValidationError::InvalidInstruction {
                    message: "Invalid instruction".to_string(),
                });
            }
        }
    }

    fn validate_value(
        &mut self,
        value_id: MirValueId,
        value_ids: &HashSet<MirValueId>,
        context: &str,
    ) {
        if !value_ids.contains(&value_id) {
            self.errors.push(MirValidationError::InvalidValue {
                value_id: value_id.0,
                message: format!("unknown MIR value used as {context}"),
            });
        }
    }
}

impl Default for MirValidator {
    fn default() -> Self {
        Self::new()
    }
}
