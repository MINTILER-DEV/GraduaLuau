use std::collections::{HashMap, HashSet};

use super::function::MirFunction;
use super::instruction::MirInstructionKind;
use super::ssa::{MirProgramPoint, MirSsaMetadata};
use super::types::{MirBlockId, MirValueId};
use super::value::MirValueKind;

#[derive(Debug, Clone)]
pub struct MirVariableLifetime {
    pub storage: String,
    pub value_id: MirValueId,
    pub definition: Option<MirProgramPoint>,
    pub first_use: Option<MirProgramPoint>,
    pub last_use: Option<MirProgramPoint>,
    pub live_blocks: HashSet<MirBlockId>,
    pub dead: bool,
}

#[derive(Debug, Clone)]
pub struct MirValueLifetime {
    pub value_id: MirValueId,
    pub definition: Option<MirProgramPoint>,
    pub uses: Vec<MirProgramPoint>,
    pub last_use: Option<MirProgramPoint>,
    pub live_blocks: HashSet<MirBlockId>,
    pub dead: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MirLifetimeMetadata {
    pub live_in: HashMap<MirBlockId, HashSet<String>>,
    pub live_out: HashMap<MirBlockId, HashSet<String>>,
    pub variable_lifetimes: HashMap<String, MirVariableLifetime>,
    pub value_lifetimes: HashMap<MirValueId, MirValueLifetime>,
    pub dead_variables: HashSet<String>,
    pub dead_values: HashSet<MirValueId>,
}

pub struct MirLifetimeAnalysis;

impl MirLifetimeAnalysis {
    pub fn analyze(function: &MirFunction, ssa: &MirSsaMetadata) -> MirLifetimeMetadata {
        let (block_defs, block_uses) = Self::block_variable_sets(function, ssa);
        let (live_in, live_out) = Self::compute_live_sets(function, &block_defs, &block_uses);
        let mut metadata = MirLifetimeMetadata {
            live_in,
            live_out,
            ..MirLifetimeMetadata::default()
        };

        metadata.variable_lifetimes = Self::compute_variable_lifetimes(function, ssa, &metadata);
        metadata.value_lifetimes = Self::compute_value_lifetimes(function);
        metadata.dead_variables = metadata
            .variable_lifetimes
            .iter()
            .filter_map(|(storage, lifetime)| lifetime.dead.then_some(storage.clone()))
            .collect();
        metadata.dead_values = metadata
            .value_lifetimes
            .iter()
            .filter_map(|(value_id, lifetime)| lifetime.dead.then_some(*value_id))
            .collect();

        metadata
    }

    pub fn validate(metadata: &MirLifetimeMetadata) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (storage, lifetime) in &metadata.variable_lifetimes {
            if lifetime.storage != *storage {
                errors.push(format!(
                    "lifetime for {storage} was recorded under {}",
                    lifetime.storage
                ));
            }

            if lifetime.first_use.is_some() && lifetime.definition.is_none() {
                errors.push(format!(
                    "live range for {storage} has a use without a definition"
                ));
            }

            if lifetime.dead && lifetime.last_use.is_some() {
                errors.push(format!("dead variable {storage} has a recorded use"));
            }
        }

        for (value_id, lifetime) in &metadata.value_lifetimes {
            if lifetime.value_id != *value_id {
                errors.push(format!(
                    "value lifetime for %{} was recorded under %{}",
                    lifetime.value_id.0, value_id.0
                ));
            }

            if !lifetime.uses.is_empty() && lifetime.definition.is_none() {
                errors.push(format!("value %{} is used before definition", value_id.0));
            }

            if lifetime.dead && lifetime.last_use.is_some() {
                errors.push(format!("dead value %{} has a recorded use", value_id.0));
            }
        }

        for (block, live_in) in &metadata.live_in {
            if let Some(live_out) = metadata.live_out.get(block) {
                for variable in live_in.intersection(live_out) {
                    if !metadata.variable_lifetimes.contains_key(variable) {
                        errors.push(format!(
                            "live set for block {} references unknown variable {variable}",
                            block.0
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn dump(metadata: &MirLifetimeMetadata) -> String {
        let mut output = String::new();
        let mut variables: Vec<_> = metadata.variable_lifetimes.keys().collect();
        variables.sort();

        for storage in variables {
            let lifetime = &metadata.variable_lifetimes[storage];
            output.push_str(&format!("Variable {storage}\n"));
            if let Some(definition) = lifetime.definition {
                output.push_str(&format!(
                    "  Definition: Block{} @{}\n",
                    definition.block.0, definition.instruction_index
                ));
            }
            if let Some(last_use) = lifetime.last_use {
                output.push_str(&format!(
                    "  Last Use: Block{} @{}\n",
                    last_use.block.0, last_use.instruction_index
                ));
            }
            output.push_str(&format!("  Dead: {}\n", lifetime.dead));
        }

        output
    }

    fn block_variable_sets(
        function: &MirFunction,
        ssa: &MirSsaMetadata,
    ) -> (
        HashMap<MirBlockId, HashSet<String>>,
        HashMap<MirBlockId, HashSet<String>>,
    ) {
        let mut block_defs = HashMap::new();
        let mut block_uses = HashMap::new();

        for block in &function.blocks {
            let mut defs = HashSet::new();
            let mut uses = HashSet::new();

            for instruction in &block.instructions {
                match &instruction.kind {
                    MirInstructionKind::Load { name, .. } => {
                        if ssa.variables.contains_key(name) && !defs.contains(name) {
                            uses.insert(name.clone());
                        }
                    }
                    MirInstructionKind::Store { name, .. }
                    | MirInstructionKind::AllocateLocal { name, .. } => {
                        if ssa.variables.contains_key(name) {
                            defs.insert(name.clone());
                        }
                    }
                    _ => {}
                }
            }

            block_defs.insert(block.id, defs);
            block_uses.insert(block.id, uses);
        }

        (block_defs, block_uses)
    }

    fn compute_live_sets(
        function: &MirFunction,
        block_defs: &HashMap<MirBlockId, HashSet<String>>,
        block_uses: &HashMap<MirBlockId, HashSet<String>>,
    ) -> (
        HashMap<MirBlockId, HashSet<String>>,
        HashMap<MirBlockId, HashSet<String>>,
    ) {
        let mut live_in: HashMap<_, _> = function
            .blocks
            .iter()
            .map(|block| (block.id, HashSet::new()))
            .collect();
        let mut live_out = live_in.clone();

        let mut changed = true;
        while changed {
            changed = false;

            for block in function.blocks.iter().rev() {
                let old_in = live_in.get(&block.id).cloned().unwrap_or_default();
                let old_out = live_out.get(&block.id).cloned().unwrap_or_default();

                let mut new_out = HashSet::new();
                for successor in &block.successors {
                    if let Some(successor_live_in) = live_in.get(successor) {
                        new_out.extend(successor_live_in.iter().cloned());
                    }
                }

                let mut new_in = block_uses.get(&block.id).cloned().unwrap_or_default();
                let defs = block_defs.get(&block.id).cloned().unwrap_or_default();
                new_in.extend(new_out.difference(&defs).cloned());

                if new_in != old_in || new_out != old_out {
                    live_in.insert(block.id, new_in);
                    live_out.insert(block.id, new_out);
                    changed = true;
                }
            }
        }

        (live_in, live_out)
    }

    fn compute_variable_lifetimes(
        function: &MirFunction,
        ssa: &MirSsaMetadata,
        metadata: &MirLifetimeMetadata,
    ) -> HashMap<String, MirVariableLifetime> {
        let mut lifetimes = HashMap::new();

        for (storage, variable) in &ssa.variables {
            let definitions = ssa.definitions.get(storage).cloned().unwrap_or_default();
            let uses = ssa.uses.get(storage).cloned().unwrap_or_default();
            let definition = definitions
                .iter()
                .map(|definition| MirProgramPoint {
                    block: definition.block,
                    instruction_index: definition.instruction_index,
                })
                .min_by_key(|point| Self::point_order(function, *point));
            let first_use = uses
                .iter()
                .map(|variable_use| MirProgramPoint {
                    block: variable_use.block,
                    instruction_index: variable_use.instruction_index,
                })
                .min_by_key(|point| Self::point_order(function, *point));
            let last_use = uses
                .iter()
                .map(|variable_use| MirProgramPoint {
                    block: variable_use.block,
                    instruction_index: variable_use.instruction_index,
                })
                .max_by_key(|point| Self::point_order(function, *point));
            let mut live_blocks: HashSet<_> =
                uses.iter().map(|variable_use| variable_use.block).collect();

            if let Some(definition) = definition {
                live_blocks.insert(definition.block);
            }

            for (block, live_in) in &metadata.live_in {
                if live_in.contains(storage) {
                    live_blocks.insert(*block);
                }
            }

            for (block, live_out) in &metadata.live_out {
                if live_out.contains(storage) {
                    live_blocks.insert(*block);
                }
            }

            lifetimes.insert(
                storage.clone(),
                MirVariableLifetime {
                    storage: storage.clone(),
                    value_id: variable.value_id,
                    definition,
                    first_use,
                    last_use,
                    live_blocks,
                    dead: uses.is_empty(),
                },
            );
        }

        lifetimes
    }

    fn compute_value_lifetimes(function: &MirFunction) -> HashMap<MirValueId, MirValueLifetime> {
        let mut lifetimes: HashMap<_, _> = function
            .values
            .iter()
            .map(|value| {
                (
                    value.id,
                    MirValueLifetime {
                        value_id: value.id,
                        definition: Self::implicit_value_definition(function, value.id),
                        uses: Vec::new(),
                        last_use: None,
                        live_blocks: HashSet::new(),
                        dead: false,
                    },
                )
            })
            .collect();

        for block in &function.blocks {
            for (instruction_index, instruction) in block.instructions.iter().enumerate() {
                let point = MirProgramPoint {
                    block: block.id,
                    instruction_index,
                };

                if let Some(result) = Self::instruction_result(&instruction.kind) {
                    lifetimes
                        .entry(result)
                        .or_insert_with(|| MirValueLifetime {
                            value_id: result,
                            definition: None,
                            uses: Vec::new(),
                            last_use: None,
                            live_blocks: HashSet::new(),
                            dead: false,
                        })
                        .definition = Some(point);
                }

                for used_value in Self::instruction_uses(&instruction.kind) {
                    let lifetime =
                        lifetimes
                            .entry(used_value)
                            .or_insert_with(|| MirValueLifetime {
                                value_id: used_value,
                                definition: None,
                                uses: Vec::new(),
                                last_use: None,
                                live_blocks: HashSet::new(),
                                dead: false,
                            });
                    lifetime.uses.push(point);
                    lifetime.live_blocks.insert(block.id);
                }
            }
        }

        for lifetime in lifetimes.values_mut() {
            if let Some(definition) = lifetime.definition {
                lifetime.live_blocks.insert(definition.block);
            }
            lifetime.last_use = lifetime
                .uses
                .iter()
                .copied()
                .max_by_key(|point| Self::point_order(function, *point));
            lifetime.dead = lifetime.uses.is_empty()
                && !matches!(
                    function
                        .values
                        .iter()
                        .find(|value| value.id == lifetime.value_id)
                        .map(|value| &value.kind),
                    Some(MirValueKind::Parameter { .. } | MirValueKind::Local { .. })
                );
        }

        lifetimes
    }

    fn implicit_value_definition(
        function: &MirFunction,
        value_id: MirValueId,
    ) -> Option<MirProgramPoint> {
        let entry = function
            .entry_block
            .and_then(|entry_index| function.blocks.get(entry_index))
            .map(|block| block.id)?;

        function
            .values
            .iter()
            .find(|value| value.id == value_id)
            .and_then(|value| {
                matches!(
                    value.kind,
                    MirValueKind::Parameter { .. } | MirValueKind::Local { .. }
                )
                .then_some(MirProgramPoint {
                    block: entry,
                    instruction_index: 0,
                })
            })
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

    fn point_order(function: &MirFunction, point: MirProgramPoint) -> (usize, usize) {
        let block_index = function
            .blocks
            .iter()
            .position(|block| block.id == point.block)
            .unwrap_or(usize::MAX);
        (block_index, point.instruction_index)
    }
}
