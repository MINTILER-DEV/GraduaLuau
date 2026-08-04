use std::collections::{HashMap, HashSet};

use super::instruction::MirInstructionKind;
use super::module::MirModule;
use super::types::{MirValue, MirValueId};
use super::value::MirValueKind;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MirOptimizationStats {
    pub constants_folded: usize,
    pub copies_propagated: usize,
    pub dead_instructions_removed: usize,
    pub unreachable_blocks_removed: usize,
}

#[derive(Debug, Clone)]
pub struct MirOptimizationResult {
    pub module: MirModule,
    pub stats: MirOptimizationStats,
}

#[derive(Debug, Default)]
pub struct MirOptimizer {
    stats: MirOptimizationStats,
}

impl MirOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn optimize(&mut self, module: &MirModule) -> MirOptimizationResult {
        self.stats = MirOptimizationStats::default();
        let mut optimized = module.clone();

        for function in &mut optimized.functions {
            self.copy_propagation(function);
            self.constant_folding(function);
            self.dead_code_elimination(function);
            self.remove_unreachable_blocks(function);
            Self::refresh_block_terminators(function);
            function.rebuild_cfg();
        }

        MirOptimizationResult {
            module: optimized,
            stats: self.stats.clone(),
        }
    }

    pub fn stats(&self) -> &MirOptimizationStats {
        &self.stats
    }

    fn copy_propagation(&mut self, function: &mut super::function::MirFunction) {
        let mut replacements = HashMap::new();

        for block in &function.blocks {
            for instruction in &block.instructions {
                if let MirInstructionKind::Move { result, value } = instruction.kind {
                    let resolved = Self::resolve_replacement(value, &replacements);
                    replacements.insert(result, resolved);
                }
            }
        }

        if replacements.is_empty() {
            return;
        }

        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                Self::replace_instruction_uses(&mut instruction.kind, &replacements);
            }

            let before = block.instructions.len();
            block.instructions.retain(|instruction| {
                !matches!(instruction.kind, MirInstructionKind::Move { result, .. } if replacements.contains_key(&result))
            });
            self.stats.copies_propagated += before - block.instructions.len();
        }

        let replaced_results: HashSet<_> = replacements.keys().copied().collect();
        function
            .values
            .retain(|value| !replaced_results.contains(&value.id));
    }

    fn constant_folding(&mut self, function: &mut super::function::MirFunction) {
        let mut constants = HashMap::new();

        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                match &mut instruction.kind {
                    MirInstructionKind::Const { result, value } => {
                        constants.insert(*result, value.clone());
                    }
                    _ => {
                        if let Some((result, value)) =
                            Self::fold_instruction(&instruction.kind, &constants)
                        {
                            instruction.kind = MirInstructionKind::Const {
                                result,
                                value: value.clone(),
                            };
                            constants.insert(result, value);
                            self.stats.constants_folded += 1;
                        }
                    }
                }
            }
        }
    }

    fn dead_code_elimination(&mut self, function: &mut super::function::MirFunction) {
        let uses = Self::value_use_counts(function);
        let protected_values: HashSet<_> = function
            .values
            .iter()
            .filter_map(|value| {
                matches!(
                    value.kind,
                    MirValueKind::Local { .. }
                        | MirValueKind::Parameter { .. }
                        | MirValueKind::Global { .. }
                        | MirValueKind::FunctionReference { .. }
                )
                .then_some(value.id)
            })
            .collect();
        let mut removed_values = HashSet::new();

        for block in &mut function.blocks {
            let before = block.instructions.len();
            block.instructions.retain(|instruction| {
                let Some(result) = Self::instruction_result(&instruction.kind) else {
                    return true;
                };

                if protected_values.contains(&result) {
                    return true;
                }

                let removable = Self::is_pure_instruction(&instruction.kind)
                    && uses.get(&result).copied().unwrap_or(0) == 0;
                if removable {
                    removed_values.insert(result);
                }
                !removable
            });
            self.stats.dead_instructions_removed += before - block.instructions.len();
        }

        if !removed_values.is_empty() {
            function
                .values
                .retain(|value| !removed_values.contains(&value.id));
        }
    }

    fn remove_unreachable_blocks(&mut self, function: &mut super::function::MirFunction) {
        function.rebuild_cfg();
        let Some(cfg) = function.cfg.as_ref() else {
            return;
        };

        let reachable: HashSet<_> = cfg.reachable_blocks().into_iter().collect();
        if reachable.len() == function.blocks.len() {
            return;
        }

        let before = function.blocks.len();
        function
            .blocks
            .retain(|block| reachable.contains(&block.id));
        self.stats.unreachable_blocks_removed += before - function.blocks.len();

        let live_blocks: HashSet<_> = function.blocks.iter().map(|block| block.id).collect();
        for block in &mut function.blocks {
            block
                .predecessors
                .retain(|predecessor| live_blocks.contains(predecessor));
            block
                .successors
                .retain(|successor| live_blocks.contains(successor));
        }

        function.entry_block = function
            .blocks
            .iter()
            .position(|block| block.is_entry)
            .or(function.entry_block);
        function.exit_blocks = function
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| block.is_exit.then_some(index))
            .collect();
    }

    fn refresh_block_terminators(function: &mut super::function::MirFunction) {
        for block in &mut function.blocks {
            block.terminator = block
                .instructions
                .last()
                .and_then(|instruction| instruction.terminator());
            block.successors = block
                .terminator
                .as_ref()
                .map(|terminator| terminator.successors())
                .unwrap_or_default();
        }
    }

    fn fold_instruction(
        kind: &MirInstructionKind,
        constants: &HashMap<MirValueId, MirValue>,
    ) -> Option<(MirValueId, MirValue)> {
        match kind {
            MirInstructionKind::Add {
                result,
                left,
                right,
            } => Self::fold_binary(*result, *left, *right, constants, |left, right| {
                Some(left + right)
            }),
            MirInstructionKind::Subtract {
                result,
                left,
                right,
            } => Self::fold_binary(*result, *left, *right, constants, |left, right| {
                Some(left - right)
            }),
            MirInstructionKind::Multiply {
                result,
                left,
                right,
            } => Self::fold_binary(*result, *left, *right, constants, |left, right| {
                Some(left * right)
            }),
            MirInstructionKind::Divide {
                result,
                left,
                right,
            } => Self::fold_binary(*result, *left, *right, constants, |left, right| {
                (right != 0).then_some(left / right)
            }),
            MirInstructionKind::Modulo {
                result,
                left,
                right,
            } => Self::fold_binary(*result, *left, *right, constants, |left, right| {
                (right != 0).then_some(left % right)
            }),
            MirInstructionKind::Equal {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left == right
            }),
            MirInstructionKind::NotEqual {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left != right
            }),
            MirInstructionKind::LessThan {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left < right
            }),
            MirInstructionKind::LessEqual {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left <= right
            }),
            MirInstructionKind::GreaterThan {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left > right
            }),
            MirInstructionKind::GreaterEqual {
                result,
                left,
                right,
            } => Self::fold_comparison(*result, *left, *right, constants, |left, right| {
                left >= right
            }),
            MirInstructionKind::And {
                result,
                left,
                right,
            } => {
                let (MirValue::Boolean(left), MirValue::Boolean(right)) =
                    (constants.get(left)?, constants.get(right)?)
                else {
                    return None;
                };
                Some((*result, MirValue::Boolean(*left && *right)))
            }
            MirInstructionKind::Or {
                result,
                left,
                right,
            } => {
                let (MirValue::Boolean(left), MirValue::Boolean(right)) =
                    (constants.get(left)?, constants.get(right)?)
                else {
                    return None;
                };
                Some((*result, MirValue::Boolean(*left || *right)))
            }
            MirInstructionKind::Not { result, operand } => {
                let MirValue::Boolean(value) = constants.get(operand)? else {
                    return None;
                };
                Some((*result, MirValue::Boolean(!*value)))
            }
            _ => None,
        }
    }

    fn fold_binary(
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
        constants: &HashMap<MirValueId, MirValue>,
        operation: impl Fn(i64, i64) -> Option<i64>,
    ) -> Option<(MirValueId, MirValue)> {
        let (MirValue::Integer(left), MirValue::Integer(right)) =
            (constants.get(&left)?, constants.get(&right)?)
        else {
            return None;
        };

        operation(*left, *right).map(|value| (result, MirValue::Integer(value)))
    }

    fn fold_comparison(
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
        constants: &HashMap<MirValueId, MirValue>,
        operation: impl Fn(i64, i64) -> bool,
    ) -> Option<(MirValueId, MirValue)> {
        let (MirValue::Integer(left), MirValue::Integer(right)) =
            (constants.get(&left)?, constants.get(&right)?)
        else {
            return None;
        };

        Some((result, MirValue::Boolean(operation(*left, *right))))
    }

    fn value_use_counts(function: &super::function::MirFunction) -> HashMap<MirValueId, usize> {
        let mut uses = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                for value_id in Self::instruction_uses(&instruction.kind) {
                    *uses.entry(value_id).or_insert(0) += 1;
                }
            }
        }
        uses
    }

    fn resolve_replacement(
        value_id: MirValueId,
        replacements: &HashMap<MirValueId, MirValueId>,
    ) -> MirValueId {
        let mut current = value_id;
        let mut seen = HashSet::new();
        while let Some(next) = replacements.get(&current).copied() {
            if !seen.insert(current) {
                break;
            }
            current = next;
        }
        current
    }

    fn replace_instruction_uses(
        kind: &mut MirInstructionKind,
        replacements: &HashMap<MirValueId, MirValueId>,
    ) {
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
            | MirInstructionKind::Compare { left, right, .. } => {
                *left = Self::resolve_replacement(*left, replacements);
                *right = Self::resolve_replacement(*right, replacements);
            }
            MirInstructionKind::Not { operand, .. } => {
                *operand = Self::resolve_replacement(*operand, replacements);
            }
            MirInstructionKind::Move { value, .. } | MirInstructionKind::Store { value, .. } => {
                *value = Self::resolve_replacement(*value, replacements);
            }
            MirInstructionKind::Branch { condition, .. } => {
                *condition = Self::resolve_replacement(*condition, replacements);
            }
            MirInstructionKind::Call { arguments, .. } => {
                for argument in arguments {
                    *argument = Self::resolve_replacement(*argument, replacements);
                }
            }
            MirInstructionKind::Return { value } => {
                if let Some(value) = value {
                    *value = Self::resolve_replacement(*value, replacements);
                }
            }
            MirInstructionKind::TableSet { table, key, value } => {
                *table = Self::resolve_replacement(*table, replacements);
                *key = Self::resolve_replacement(*key, replacements);
                *value = Self::resolve_replacement(*value, replacements);
            }
            MirInstructionKind::TableGet { table, key, .. } => {
                *table = Self::resolve_replacement(*table, replacements);
                *key = Self::resolve_replacement(*key, replacements);
            }
            MirInstructionKind::Const { .. }
            | MirInstructionKind::Load { .. }
            | MirInstructionKind::AllocateLocal { .. }
            | MirInstructionKind::Jump { .. }
            | MirInstructionKind::Unreachable
            | MirInstructionKind::TableNew { .. }
            | MirInstructionKind::Error => {}
        }
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

    fn is_pure_instruction(kind: &MirInstructionKind) -> bool {
        matches!(
            kind,
            MirInstructionKind::Const { .. }
                | MirInstructionKind::Add { .. }
                | MirInstructionKind::Subtract { .. }
                | MirInstructionKind::Multiply { .. }
                | MirInstructionKind::Divide { .. }
                | MirInstructionKind::Modulo { .. }
                | MirInstructionKind::Equal { .. }
                | MirInstructionKind::NotEqual { .. }
                | MirInstructionKind::LessThan { .. }
                | MirInstructionKind::LessEqual { .. }
                | MirInstructionKind::GreaterThan { .. }
                | MirInstructionKind::GreaterEqual { .. }
                | MirInstructionKind::And { .. }
                | MirInstructionKind::Or { .. }
                | MirInstructionKind::Not { .. }
                | MirInstructionKind::Load { .. }
                | MirInstructionKind::Move { .. }
                | MirInstructionKind::Compare { .. }
        )
    }
}
