use std::collections::{HashMap, HashSet, VecDeque};

use super::function::MirFunction;
use super::instruction::MirInstructionKind;
use super::types::{MirBlockId, MirType, MirValueId};
use super::value::MirValueKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirProgramPoint {
    pub block: MirBlockId,
    pub instruction_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirVariableCategory {
    Local,
    Parameter,
    Global,
    CompilerTemporary,
    ReturnSlot,
}

#[derive(Debug, Clone)]
pub struct MirSsaVariable {
    pub storage: String,
    pub value_id: MirValueId,
    pub symbol_id: Option<usize>,
    pub value_type: MirType,
    pub category: MirVariableCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirDefinitionKind {
    Allocation,
    Store,
}

#[derive(Debug, Clone)]
pub struct MirVariableDefinition {
    pub variable: String,
    pub value_id: MirValueId,
    pub symbol_id: Option<usize>,
    pub value_type: MirType,
    pub block: MirBlockId,
    pub instruction_index: usize,
    pub assigned_value: Option<MirValueId>,
    pub kind: MirDefinitionKind,
}

#[derive(Debug, Clone)]
pub struct MirVariableUse {
    pub variable: String,
    pub value_id: MirValueId,
    pub symbol_id: Option<usize>,
    pub block: MirBlockId,
    pub instruction_index: usize,
    pub loaded_value: MirValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirPhiCandidate {
    pub variable: String,
    pub value_id: MirValueId,
    pub symbol_id: Option<usize>,
    pub block: MirBlockId,
}

#[derive(Debug, Clone, Default)]
pub struct MirSsaMetadata {
    pub variables: HashMap<String, MirSsaVariable>,
    pub definitions: HashMap<String, Vec<MirVariableDefinition>>,
    pub uses: HashMap<String, Vec<MirVariableUse>>,
    pub dominators: HashMap<MirBlockId, HashSet<MirBlockId>>,
    pub immediate_dominators: HashMap<MirBlockId, MirBlockId>,
    pub dominance_frontiers: HashMap<MirBlockId, HashSet<MirBlockId>>,
    pub phi_candidates: HashMap<String, Vec<MirPhiCandidate>>,
}

pub struct MirSsaPreparation;

impl MirSsaPreparation {
    pub fn analyze(function: &MirFunction) -> MirSsaMetadata {
        let mut metadata = MirSsaMetadata {
            variables: Self::collect_variables(function),
            ..MirSsaMetadata::default()
        };

        Self::collect_definitions_and_uses(function, &mut metadata);

        if let Some(cfg) = &function.cfg {
            metadata.dominators = cfg.dominators();
            metadata.immediate_dominators = cfg.immediate_dominators();
            metadata.dominance_frontiers = cfg.dominance_frontiers();
            metadata.phi_candidates = Self::discover_phi_candidates(&metadata);
        }

        metadata
    }

    pub fn validate(metadata: &MirSsaMetadata) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (storage, definitions) in &metadata.definitions {
            if !metadata.variables.contains_key(storage) {
                errors.push(format!("definition references unknown variable {storage}"));
            }

            for definition in definitions {
                if definition.variable != *storage {
                    errors.push(format!(
                        "definition for {storage} was recorded under {}",
                        definition.variable
                    ));
                }
            }
        }

        for (storage, uses) in &metadata.uses {
            if !metadata.variables.contains_key(storage) {
                errors.push(format!("use references unknown variable {storage}"));
            }

            if !metadata.definitions.contains_key(storage) {
                errors.push(format!("use of {storage} has no recorded definition"));
            }

            for variable_use in uses {
                if variable_use.variable != *storage {
                    errors.push(format!(
                        "use for {storage} was recorded under {}",
                        variable_use.variable
                    ));
                }
            }
        }

        for (block, frontier) in &metadata.dominance_frontiers {
            if !metadata.dominators.contains_key(block) {
                errors.push(format!(
                    "dominance frontier recorded for non-dominated block {}",
                    block.0
                ));
            }

            for frontier_block in frontier {
                if !metadata.dominators.contains_key(frontier_block) {
                    errors.push(format!(
                        "dominance frontier references unknown block {}",
                        frontier_block.0
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn dump(metadata: &MirSsaMetadata) -> String {
        let mut output = String::new();
        let mut variables: Vec<_> = metadata.variables.keys().collect();
        variables.sort();

        for storage in variables {
            output.push_str(&format!("Variable {storage}\n"));

            if let Some(definitions) = metadata.definitions.get(storage) {
                output.push_str("  Definitions:\n");
                for definition in definitions {
                    output.push_str(&format!(
                        "    Block{} @{}\n",
                        definition.block.0, definition.instruction_index
                    ));
                }
            }

            if let Some(uses) = metadata.uses.get(storage) {
                output.push_str("  Uses:\n");
                for variable_use in uses {
                    output.push_str(&format!(
                        "    Block{} @{}\n",
                        variable_use.block.0, variable_use.instruction_index
                    ));
                }
            }

            if let Some(candidates) = metadata.phi_candidates.get(storage) {
                output.push_str("  Phi Candidates:\n");
                for candidate in candidates {
                    output.push_str(&format!("    Block{}\n", candidate.block.0));
                }
            }
        }

        output
    }

    fn collect_variables(function: &MirFunction) -> HashMap<String, MirSsaVariable> {
        let mut variables = HashMap::new();

        for local in &function.locals {
            let category = if local.storage == "local_return" {
                MirVariableCategory::ReturnSlot
            } else if function
                .parameter_data
                .iter()
                .any(|parameter| parameter.storage == local.storage)
            {
                MirVariableCategory::Parameter
            } else {
                MirVariableCategory::Local
            };

            variables.insert(
                local.storage.clone(),
                MirSsaVariable {
                    storage: local.storage.clone(),
                    value_id: local.value_id,
                    symbol_id: local.symbol_id,
                    value_type: local.value_type.clone(),
                    category,
                },
            );
        }

        for value in &function.values {
            if let MirValueKind::Global { name, symbol_id } = &value.kind {
                variables
                    .entry(name.clone())
                    .or_insert_with(|| MirSsaVariable {
                        storage: name.clone(),
                        value_id: value.id,
                        symbol_id: *symbol_id,
                        value_type: value.value_type.clone(),
                        category: MirVariableCategory::Global,
                    });
            }
        }

        variables
    }

    fn collect_definitions_and_uses(function: &MirFunction, metadata: &mut MirSsaMetadata) {
        for block in &function.blocks {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                match &instruction.kind {
                    MirInstructionKind::AllocateLocal { local, name } => {
                        if let Some(variable) = metadata.variables.get(name) {
                            metadata.definitions.entry(name.clone()).or_default().push(
                                MirVariableDefinition {
                                    variable: name.clone(),
                                    value_id: variable.value_id,
                                    symbol_id: variable.symbol_id,
                                    value_type: variable.value_type.clone(),
                                    block: block.id,
                                    instruction_index,
                                    assigned_value: Some(*local),
                                    kind: MirDefinitionKind::Allocation,
                                },
                            );
                        }
                    }
                    MirInstructionKind::Store { name, value } => {
                        if let Some(variable) = metadata.variables.get(name) {
                            metadata.definitions.entry(name.clone()).or_default().push(
                                MirVariableDefinition {
                                    variable: name.clone(),
                                    value_id: variable.value_id,
                                    symbol_id: variable.symbol_id,
                                    value_type: variable.value_type.clone(),
                                    block: block.id,
                                    instruction_index,
                                    assigned_value: Some(*value),
                                    kind: MirDefinitionKind::Store,
                                },
                            );
                        }
                    }
                    MirInstructionKind::Load { result, name } => {
                        if let Some(variable) = metadata.variables.get(name) {
                            metadata
                                .uses
                                .entry(name.clone())
                                .or_default()
                                .push(MirVariableUse {
                                    variable: name.clone(),
                                    value_id: variable.value_id,
                                    symbol_id: variable.symbol_id,
                                    block: block.id,
                                    instruction_index,
                                    loaded_value: *result,
                                });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn discover_phi_candidates(metadata: &MirSsaMetadata) -> HashMap<String, Vec<MirPhiCandidate>> {
        let mut phi_candidates = HashMap::new();

        for (storage, definitions) in &metadata.definitions {
            let definition_blocks: HashSet<_> = definitions
                .iter()
                .filter(|definition| definition.kind == MirDefinitionKind::Store)
                .map(|definition| definition.block)
                .collect();

            if definition_blocks.len() < 2 {
                continue;
            }

            let Some(variable) = metadata.variables.get(storage) else {
                continue;
            };

            let mut worklist: VecDeque<_> = definition_blocks.iter().copied().collect();
            let mut visited = HashSet::new();
            let mut candidates = Vec::new();

            while let Some(block) = worklist.pop_front() {
                if let Some(frontier) = metadata.dominance_frontiers.get(&block) {
                    let mut ordered_frontier: Vec<_> = frontier.iter().copied().collect();
                    ordered_frontier.sort_by_key(|block_id| block_id.0);

                    for frontier_block in ordered_frontier {
                        if !visited.insert(frontier_block) {
                            continue;
                        }

                        candidates.push(MirPhiCandidate {
                            variable: storage.clone(),
                            value_id: variable.value_id,
                            symbol_id: variable.symbol_id,
                            block: frontier_block,
                        });

                        if !definition_blocks.contains(&frontier_block) {
                            worklist.push_back(frontier_block);
                        }
                    }
                }
            }

            if !candidates.is_empty() {
                phi_candidates.insert(storage.clone(), candidates);
            }
        }

        phi_candidates
    }
}
