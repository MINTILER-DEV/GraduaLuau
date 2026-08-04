use super::block::MirBasicBlock;
use super::function::MirFunction;
use super::instruction::{MirInstruction, MirInstructionKind};
use super::lifetime::MirLifetimeAnalysis;
use super::module::MirModule;
use super::ssa::MirSsaPreparation;
use super::types::{MirBlockId, MirType, MirValueId};
use super::value::MirValueKind;
use std::collections::{HashMap, HashSet};

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
    InvalidModule {
        message: String,
    },
    InvalidFunction {
        function: String,
        message: String,
    },
    MissingExitPath {
        function: String,
    },
    MissingTerminator {
        block_id: usize,
    },
    InvalidTerminator {
        block_id: usize,
        message: String,
    },
    InvalidBranchTarget {
        target: usize,
    },
    InvalidValue {
        value_id: usize,
        message: String,
    },
    InvalidType {
        value_id: Option<usize>,
        message: String,
    },
    InvalidMetadata {
        function: String,
        message: String,
    },
}

impl MirValidator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn validate(&mut self, module: &MirModule) -> Result<(), Vec<MirValidationError>> {
        self.errors.clear();
        self.validate_module(module);

        for function in &module.functions {
            self.validate_function(function, module);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn report(&self) -> String {
        let mut output = String::new();
        output.push_str("MIR Validation\n");
        if self.errors.is_empty() {
            output.push_str("✔ Module\n");
            output.push_str("✔ Functions\n");
            output.push_str("✔ Blocks\n");
            output.push_str("✔ Instructions\n");
            output.push_str("✔ CFG\n");
            output.push_str("✔ Types\n");
            output.push_str("✔ Values\n");
            output.push_str("Validation Passed\n");
        } else {
            output.push_str(&format!("✘ {} error(s)\n", self.errors.len()));
            for error in &self.errors {
                output.push_str(&format!("  - {:?}\n", error));
            }
        }
        output
    }

    fn validate_module(&mut self, module: &MirModule) {
        let mut function_ids = HashSet::new();
        let mut function_names = HashSet::new();
        for function in &module.functions {
            if !function_ids.insert(function.id) {
                self.errors.push(MirValidationError::InvalidModule {
                    message: format!("duplicate function id {}", function.id.0),
                });
            }

            if !function_names.insert(function.name.clone()) {
                self.errors.push(MirValidationError::InvalidModule {
                    message: format!("duplicate function name '{}'", function.name),
                });
            }
        }

        let mut global_ids = HashSet::new();
        let mut global_storage = HashSet::new();
        for global in &module.globals {
            if !global_ids.insert(global.value_id) {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: global.value_id.0,
                    message: "duplicate global value id".to_string(),
                });
            }

            if !global_storage.insert(global.storage.clone()) {
                self.errors.push(MirValidationError::InvalidModule {
                    message: format!("duplicate global storage '{}'", global.storage),
                });
            }

            if !Self::valid_type(&global.value_type) {
                self.errors.push(MirValidationError::InvalidType {
                    value_id: Some(global.value_id.0),
                    message: "global has invalid type".to_string(),
                });
            }
        }

        let mut constant_ids = HashSet::new();
        for constant in &module.constants {
            if !constant_ids.insert(constant.value_id) {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: constant.value_id.0,
                    message: "duplicate constant value id".to_string(),
                });
            }

            if !Self::valid_type(&constant.value_type) {
                self.errors.push(MirValidationError::InvalidType {
                    value_id: Some(constant.value_id.0),
                    message: "constant has invalid type".to_string(),
                });
            }
        }
    }

    fn validate_function(&mut self, function: &MirFunction, module: &MirModule) {
        // Check if function has at least one block
        if function.blocks.is_empty() {
            self.errors.push(MirValidationError::InvalidFunction {
                function: function.name.clone(),
                message: format!("Function '{}' has no blocks", function.name),
            });
            return;
        }

        // Check if function has an entry block
        match function.entry_block {
            Some(entry_block) if entry_block < function.blocks.len() => {}
            Some(entry_block) => self.errors.push(MirValidationError::InvalidFunction {
                function: function.name.clone(),
                message: format!("entry block index {entry_block} is out of range"),
            }),
            None => self.errors.push(MirValidationError::InvalidFunction {
                function: function.name.clone(),
                message: format!("Function '{}' has no entry block", function.name),
            }),
        }

        if !Self::valid_type(function.return_type.as_ref().unwrap_or(&MirType::Void)) {
            self.errors.push(MirValidationError::InvalidType {
                value_id: None,
                message: format!("function '{}' has invalid return type", function.name),
            });
        }

        self.validate_parameters_and_locals(function);

        let mut value_ids = HashSet::new();
        for value in &function.values {
            if !value_ids.insert(value.id) {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: value.id.0,
                    message: "duplicate MIR value id".to_string(),
                });
            }

            if !Self::valid_type(&value.value_type) {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: value.id.0,
                    message: "MIR value has invalid type".to_string(),
                });
            }
        }

        let block_ids = self.validate_block_ids(function);
        for block in &function.blocks {
            self.validate_block(block, function, &value_ids, &block_ids, module);
        }

        self.validate_cfg(function);
        self.validate_value_definitions(function, &value_ids);
        self.validate_analysis_metadata(function);

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

    fn validate_parameters_and_locals(&mut self, function: &MirFunction) {
        let mut parameter_storage = HashSet::new();
        let mut local_storage = HashSet::new();

        for parameter in &function.parameter_data {
            if !parameter_storage.insert(parameter.storage.clone()) {
                self.errors.push(MirValidationError::InvalidFunction {
                    function: function.name.clone(),
                    message: format!("duplicate parameter storage '{}'", parameter.storage),
                });
            }

            if !Self::valid_type(&parameter.value_type) {
                self.errors.push(MirValidationError::InvalidType {
                    value_id: Some(parameter.value_id.0),
                    message: format!("parameter '{}' has invalid type", parameter.name),
                });
            }
        }

        for local in &function.locals {
            if !local_storage.insert(local.storage.clone()) {
                self.errors.push(MirValidationError::InvalidFunction {
                    function: function.name.clone(),
                    message: format!("duplicate local storage '{}'", local.storage),
                });
            }

            if !Self::valid_type(&local.value_type) {
                self.errors.push(MirValidationError::InvalidType {
                    value_id: Some(local.value_id.0),
                    message: format!("local '{}' has invalid type", local.storage),
                });
            }
        }
    }

    fn validate_block_ids(&mut self, function: &MirFunction) -> HashSet<MirBlockId> {
        let mut block_ids = HashSet::new();
        for block in &function.blocks {
            if !block_ids.insert(block.id) {
                self.errors.push(MirValidationError::InvalidFunction {
                    function: function.name.clone(),
                    message: format!("duplicate block id {}", block.id.0),
                });
            }
        }
        block_ids
    }

    fn validate_cfg(&mut self, function: &MirFunction) {
        let Some(cfg) = function.cfg.as_ref() else {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!("Function '{}' has no CFG", function.name),
            });
            return;
        };

        if let Err(cfg_errors) = cfg.validate() {
            self.errors.push(MirValidationError::InvalidFunction {
                function: function.name.clone(),
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
                let mut node_successors = node.successors.clone();
                let mut block_successors = block.successors.clone();
                node_successors.sort_by_key(|block_id| block_id.0);
                block_successors.sort_by_key(|block_id| block_id.0);

                if node_successors != block_successors {
                    self.errors.push(MirValidationError::InvalidFunction {
                        function: function.name.clone(),
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
                    self.errors.push(MirValidationError::InvalidFunction {
                        function: function.name.clone(),
                        message: format!(
                            "Block {} CFG predecessors do not match block predecessors",
                            block.id.0
                        ),
                    });
                }
            } else {
                self.errors.push(MirValidationError::InvalidFunction {
                    function: function.name.clone(),
                    message: format!("Block {} is missing from CFG", block.id.0),
                });
            }
        }

        if let Some(entry_block_index) = function.entry_block {
            if let Some(entry_block) = function.blocks.get(entry_block_index) {
                if cfg.entry != entry_block.id {
                    self.errors.push(MirValidationError::InvalidFunction {
                        function: function.name.clone(),
                        message: format!("Function '{}' CFG entry mismatch", function.name),
                    });
                }

                if !entry_block.predecessors.is_empty() {
                    self.errors.push(MirValidationError::InvalidFunction {
                        function: function.name.clone(),
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
        block_ids: &HashSet<MirBlockId>,
        module: &MirModule,
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
            self.validate_instruction(instruction, function, value_ids, block_ids, module);
        }
    }

    fn validate_instruction(
        &mut self,
        instruction: &MirInstruction,
        function: &MirFunction,
        value_ids: &HashSet<MirValueId>,
        block_ids: &HashSet<MirBlockId>,
        module: &MirModule,
    ) {
        match &instruction.kind {
            MirInstructionKind::Branch {
                condition,
                true_block,
                false_block,
            } => {
                self.validate_value(*condition, value_ids, "branch condition");

                if !block_ids.contains(true_block) {
                    self.errors.push(MirValidationError::InvalidBranchTarget {
                        target: true_block.0,
                    });
                }

                if !block_ids.contains(false_block) {
                    self.errors.push(MirValidationError::InvalidBranchTarget {
                        target: false_block.0,
                    });
                }

                self.validate_specific_type(
                    *condition,
                    function,
                    MirType::Boolean,
                    "branch condition",
                );
            }

            MirInstructionKind::Jump { target } => {
                if !block_ids.contains(target) {
                    self.errors
                        .push(MirValidationError::InvalidBranchTarget { target: target.0 });
                }
            }

            MirInstructionKind::Return { value } => {
                if let Some(value) = value {
                    self.validate_value(*value, value_ids, "return value");
                    if let Some(return_type) = &function.return_type {
                        self.validate_assignable_type(
                            *value,
                            function,
                            return_type,
                            "return value",
                        );
                    }
                } else if function
                    .return_type
                    .as_ref()
                    .is_some_and(|ty| *ty != MirType::Void)
                {
                    self.errors.push(MirValidationError::InvalidType {
                        value_id: None,
                        message: format!(
                            "function '{}' returns void from non-void function",
                            function.name
                        ),
                    });
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
                self.validate_binary_types(instruction, function);
            }

            MirInstructionKind::Not { result, operand } => {
                self.validate_value(*result, value_ids, "instruction result");
                self.validate_value(*operand, value_ids, "unary operand");
                self.validate_specific_type(*result, function, MirType::Boolean, "not result");
                self.validate_specific_type(*operand, function, MirType::Boolean, "not operand");
            }

            MirInstructionKind::Store { name, value } => {
                self.validate_value(*value, value_ids, "store value");
                self.validate_storage_exists(name, function, module, "store target");
                if let Some(storage_type) = self.storage_type(name, function, module) {
                    self.validate_assignable_type(*value, function, &storage_type, "store value");
                }
            }

            MirInstructionKind::Move { result, value } => {
                self.validate_value(*result, value_ids, "move result");
                self.validate_value(*value, value_ids, "move value");
                self.validate_same_value_type(*result, *value, function, "move");
            }

            MirInstructionKind::AllocateLocal { local, name } => {
                self.validate_value(*local, value_ids, "local allocation");
                self.validate_storage_exists(name, function, module, "local allocation");
            }

            MirInstructionKind::Call {
                result,
                function: callee,
                arguments,
            } => {
                if let Some(result) = result {
                    self.validate_value(*result, value_ids, "call result");
                }
                for argument in arguments {
                    self.validate_value(*argument, value_ids, "call argument");
                }
                self.validate_callee(callee, module);
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

    fn validate_value_definitions(
        &mut self,
        function: &MirFunction,
        value_ids: &HashSet<MirValueId>,
    ) {
        let mut creators: HashMap<MirValueId, (MirBlockId, usize)> = HashMap::new();

        for block in &function.blocks {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                if let Some(result) = Self::instruction_result(&instruction.kind) {
                    if creators
                        .insert(result, (block.id, instruction_index))
                        .is_some()
                    {
                        self.errors.push(MirValidationError::InvalidValue {
                            value_id: result.0,
                            message: "MIR value has multiple creators".to_string(),
                        });
                    }
                }
            }
        }

        for value_id in value_ids {
            if !creators.contains_key(value_id)
                && !function
                    .values
                    .iter()
                    .find(|value| value.id == *value_id)
                    .is_some_and(|value| {
                        matches!(value.kind, MirValueKind::FunctionReference { .. })
                    })
            {
                self.errors.push(MirValidationError::InvalidValue {
                    value_id: value_id.0,
                    message: "MIR value has no creator instruction".to_string(),
                });
            }
        }

        let dominators = function
            .cfg
            .as_ref()
            .map(|cfg| cfg.dominators())
            .unwrap_or_default();

        for block in &function.blocks {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                for used_value in Self::instruction_uses(&instruction.kind) {
                    let Some((definition_block, definition_index)) =
                        creators.get(&used_value).copied()
                    else {
                        continue;
                    };

                    let valid = if definition_block == block.id {
                        definition_index < instruction_index
                    } else {
                        dominators.get(&block.id).is_some_and(|block_dominators| {
                            block_dominators.contains(&definition_block)
                        })
                    };

                    if !valid {
                        self.errors.push(MirValidationError::InvalidValue {
                            value_id: used_value.0,
                            message: format!(
                                "value used before definition in block {} instruction {}",
                                block.id.0, instruction_index
                            ),
                        });
                    }
                }
            }
        }
    }

    fn validate_analysis_metadata(&mut self, function: &MirFunction) {
        let fresh_ssa = MirSsaPreparation::analyze(function);
        if let Err(errors) = MirSsaPreparation::validate(&fresh_ssa) {
            self.errors.push(MirValidationError::InvalidMetadata {
                function: function.name.clone(),
                message: format!("fresh SSA metadata is invalid: {:?}", errors),
            });
        }

        if let Some(existing) = &function.metadata.ssa {
            if existing.variables.len() != fresh_ssa.variables.len()
                || existing.definitions.len() != fresh_ssa.definitions.len()
                || existing.uses.len() != fresh_ssa.uses.len()
                || existing.phi_candidates.len() != fresh_ssa.phi_candidates.len()
            {
                self.errors.push(MirValidationError::InvalidMetadata {
                    function: function.name.clone(),
                    message: "stored SSA metadata is out of sync with MIR".to_string(),
                });
            }
        }

        let fresh_lifetimes = MirLifetimeAnalysis::analyze(function, &fresh_ssa);
        if let Err(errors) = MirLifetimeAnalysis::validate(&fresh_lifetimes) {
            self.errors.push(MirValidationError::InvalidMetadata {
                function: function.name.clone(),
                message: format!("fresh lifetime metadata is invalid: {:?}", errors),
            });
        }

        if let Some(existing) = &function.metadata.lifetimes {
            if existing.variable_lifetimes.len() != fresh_lifetimes.variable_lifetimes.len()
                || existing.value_lifetimes.len() != fresh_lifetimes.value_lifetimes.len()
                || existing.dead_variables != fresh_lifetimes.dead_variables
            {
                self.errors.push(MirValidationError::InvalidMetadata {
                    function: function.name.clone(),
                    message: "stored lifetime metadata is out of sync with MIR".to_string(),
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

    fn validate_storage_exists(
        &mut self,
        name: &str,
        function: &MirFunction,
        module: &MirModule,
        context: &str,
    ) {
        if self.storage_type(name, function, module).is_none() {
            self.errors.push(MirValidationError::InvalidInstruction {
                message: format!("unknown storage '{name}' used as {context}"),
            });
        }
    }

    fn validate_callee(&mut self, callee: &str, module: &MirModule) {
        const RUNTIME_FUNCTIONS: &[&str] = &[
            "glua_print",
            "glua_print_i64",
            "glua_print_f64",
            "glua_print_bool",
            "glua_type",
            "glua_tonumber",
            "glua_tostring",
            "glua_error",
            "glua_pairs",
            "glua_ipairs",
            "glua_require",
            "glua_table_new",
            "glua_table_set",
            "glua_table_get",
        ];

        if module
            .functions
            .iter()
            .any(|function| function.name == callee)
            || RUNTIME_FUNCTIONS.contains(&callee)
        {
            return;
        }

        self.errors.push(MirValidationError::InvalidInstruction {
            message: format!("unknown callee '{callee}'"),
        });
    }

    fn validate_binary_types(&mut self, instruction: &MirInstruction, function: &MirFunction) {
        match instruction.kind {
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
            } => {
                self.validate_numeric_type(left, function, "left arithmetic operand");
                self.validate_numeric_type(right, function, "right arithmetic operand");
                self.validate_numeric_type(result, function, "arithmetic result");
            }
            MirInstructionKind::And {
                result,
                left,
                right,
            }
            | MirInstructionKind::Or {
                result,
                left,
                right,
            } => {
                self.validate_specific_type(result, function, MirType::Boolean, "boolean result");
                self.validate_specific_type(
                    left,
                    function,
                    MirType::Boolean,
                    "left boolean operand",
                );
                self.validate_specific_type(
                    right,
                    function,
                    MirType::Boolean,
                    "right boolean operand",
                );
            }
            MirInstructionKind::Equal {
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
            | MirInstructionKind::Compare {
                result,
                left,
                right,
                ..
            } => {
                self.validate_specific_type(
                    result,
                    function,
                    MirType::Boolean,
                    "comparison result",
                );
                self.validate_same_value_type(left, right, function, "comparison operands");
            }
            _ => {}
        }
    }

    fn validate_numeric_type(
        &mut self,
        value_id: MirValueId,
        function: &MirFunction,
        context: &str,
    ) {
        let Some(value_type) = self.value_type(value_id, function) else {
            return;
        };

        if !matches!(value_type, MirType::Integer | MirType::Float | MirType::Any) {
            self.errors.push(MirValidationError::InvalidType {
                value_id: Some(value_id.0),
                message: format!("expected numeric type for {context}, got {:?}", value_type),
            });
        }
    }

    fn validate_specific_type(
        &mut self,
        value_id: MirValueId,
        function: &MirFunction,
        expected: MirType,
        context: &str,
    ) {
        let Some(value_type) = self.value_type(value_id, function) else {
            return;
        };

        if value_type != expected && value_type != MirType::Any {
            self.errors.push(MirValidationError::InvalidType {
                value_id: Some(value_id.0),
                message: format!(
                    "expected {:?} for {context}, got {:?}",
                    expected, value_type
                ),
            });
        }
    }

    fn validate_same_value_type(
        &mut self,
        left: MirValueId,
        right: MirValueId,
        function: &MirFunction,
        context: &str,
    ) {
        let (Some(left_type), Some(right_type)) = (
            self.value_type(left, function),
            self.value_type(right, function),
        ) else {
            return;
        };

        if left_type != right_type && left_type != MirType::Any && right_type != MirType::Any {
            self.errors.push(MirValidationError::InvalidType {
                value_id: Some(left.0),
                message: format!(
                    "type mismatch for {context}: {:?} vs {:?}",
                    left_type, right_type
                ),
            });
        }
    }

    fn validate_assignable_type(
        &mut self,
        value_id: MirValueId,
        function: &MirFunction,
        expected: &MirType,
        context: &str,
    ) {
        let Some(actual) = self.value_type(value_id, function) else {
            return;
        };

        if actual != *expected
            && !matches!(actual, MirType::Any | MirType::Function)
            && *expected != MirType::Any
        {
            self.errors.push(MirValidationError::InvalidType {
                value_id: Some(value_id.0),
                message: format!(
                    "type mismatch for {context}: expected {:?}, got {:?}",
                    expected, actual
                ),
            });
        }
    }

    fn storage_type(
        &self,
        name: &str,
        function: &MirFunction,
        module: &MirModule,
    ) -> Option<MirType> {
        function
            .locals
            .iter()
            .find(|local| local.storage == name)
            .map(|local| local.value_type.clone())
            .or_else(|| {
                module
                    .globals
                    .iter()
                    .find(|global| global.storage == name || global.name == name)
                    .map(|global| global.value_type.clone())
            })
    }

    fn value_type(&self, value_id: MirValueId, function: &MirFunction) -> Option<MirType> {
        function
            .values
            .iter()
            .find(|value| value.id == value_id)
            .map(|value| value.value_type.clone())
    }

    fn valid_type(value_type: &MirType) -> bool {
        !matches!(value_type, MirType::Unknown)
    }

    fn instruction_result(kind: &MirInstructionKind) -> Option<MirValueId> {
        match kind {
            MirInstructionKind::Const { result, .. }
            | MirInstructionKind::Add { result, .. }
            | MirInstructionKind::Subtract { result, .. }
            | MirInstructionKind::Multiply { result, .. }
            | MirInstructionKind::Divide { result, .. }
            | MirInstructionKind::Modulo { result, .. }
            | MirInstructionKind::Equal { result, .. }
            | MirInstructionKind::NotEqual { result, .. }
            | MirInstructionKind::LessThan { result, .. }
            | MirInstructionKind::LessEqual { result, .. }
            | MirInstructionKind::GreaterThan { result, .. }
            | MirInstructionKind::GreaterEqual { result, .. }
            | MirInstructionKind::And { result, .. }
            | MirInstructionKind::Or { result, .. }
            | MirInstructionKind::Not { result, .. }
            | MirInstructionKind::Load { result, .. }
            | MirInstructionKind::Move { result, .. }
            | MirInstructionKind::Call {
                result: Some(result),
                ..
            }
            | MirInstructionKind::Compare { result, .. }
            | MirInstructionKind::TableNew { result }
            | MirInstructionKind::TableGet { result, .. } => Some(*result),
            MirInstructionKind::AllocateLocal { local, .. } => Some(*local),
            MirInstructionKind::Call { result: None, .. }
            | MirInstructionKind::Store { .. }
            | MirInstructionKind::Branch { .. }
            | MirInstructionKind::Jump { .. }
            | MirInstructionKind::Unreachable
            | MirInstructionKind::Return { .. }
            | MirInstructionKind::TableSet { .. }
            | MirInstructionKind::Error => None,
        }
    }

    fn instruction_uses(kind: &MirInstructionKind) -> Vec<MirValueId> {
        match kind {
            MirInstructionKind::Add { left, right, .. }
            | MirInstructionKind::Subtract { left, right, .. }
            | MirInstructionKind::Multiply { left, right, .. }
            | MirInstructionKind::Divide { left, right, .. }
            | MirInstructionKind::Modulo { left, right, .. }
            | MirInstructionKind::Equal { left, right, .. }
            | MirInstructionKind::NotEqual { left, right, .. }
            | MirInstructionKind::LessThan { left, right, .. }
            | MirInstructionKind::LessEqual { left, right, .. }
            | MirInstructionKind::GreaterThan { left, right, .. }
            | MirInstructionKind::GreaterEqual { left, right, .. }
            | MirInstructionKind::And { left, right, .. }
            | MirInstructionKind::Or { left, right, .. }
            | MirInstructionKind::Compare { left, right, .. } => vec![*left, *right],
            MirInstructionKind::Not { operand, .. } => vec![*operand],
            MirInstructionKind::Move { value, .. } => vec![*value],
            MirInstructionKind::Store { value, .. } => vec![*value],
            MirInstructionKind::Branch { condition, .. } => vec![*condition],
            MirInstructionKind::Call { arguments, .. } => arguments.clone(),
            MirInstructionKind::Return { value } => value.iter().copied().collect(),
            MirInstructionKind::TableSet { table, key, value } => vec![*table, *key, *value],
            MirInstructionKind::TableGet { table, key, .. } => vec![*table, *key],
            MirInstructionKind::Const { .. }
            | MirInstructionKind::Load { .. }
            | MirInstructionKind::AllocateLocal { .. }
            | MirInstructionKind::Jump { .. }
            | MirInstructionKind::Unreachable
            | MirInstructionKind::TableNew { .. }
            | MirInstructionKind::Error => Vec::new(),
        }
    }
}

impl Default for MirValidator {
    fn default() -> Self {
        Self::new()
    }
}
