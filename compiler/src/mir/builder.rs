use std::collections::HashMap;

use crate::hir::{
    HirBinaryOperator, HirBuiltinFunction, HirExpression, HirExpressionKind, HirFunction,
    HirLocalVariable, HirModule, HirStatement, HirStatementKind, HirTableField, HirType,
    HirUnaryOperator, HirVariableId,
};
use crate::mir::block::MirBasicBlock;
use crate::mir::error::MirError;
use crate::mir::function::MirFunction;
use crate::mir::instruction::{MirInstruction, MirInstructionKind};
use crate::mir::module::MirModule;
use crate::mir::types::{MirBlockId, MirFunctionId, MirType, MirValue, MirValueId};
use crate::mir::value::{
    MirConstant, MirGlobal, MirLocal, MirParameter, MirValueData, MirValueKind,
};
use crate::source::SourceSpan;

pub struct MirBuilder {
    value_counter: usize,
    block_counter: usize,
    function_counter: usize,
    variable_symbols: HashMap<HirVariableId, usize>,
    active_exit_block: Option<MirBlockId>,
    active_return_slot: Option<String>,
}

impl MirBuilder {
    pub fn new() -> Self {
        Self {
            value_counter: 0,
            block_counter: 0,
            function_counter: 0,
            variable_symbols: HashMap::new(),
            active_exit_block: None,
            active_return_slot: None,
        }
    }

    pub fn create_block(
        &mut self,
        mir_function: &mut MirFunction,
        name: impl Into<String>,
        span: Option<SourceSpan>,
    ) -> usize {
        let id = self.new_block_id();
        mir_function.add_named_block(id, name, span)
    }

    pub fn switch_block(
        &self,
        mir_function: &MirFunction,
        block_id: MirBlockId,
    ) -> Result<usize, MirError> {
        mir_function
            .blocks
            .iter()
            .position(|block| block.id == block_id)
            .ok_or_else(|| MirError::LoweringError(format!("unknown MIR block {}", block_id.0)))
    }

    pub fn current_block(&self, mir_function: &MirFunction) -> Option<usize> {
        mir_function
            .blocks
            .iter()
            .rposition(|block| !block.is_terminated())
            .or_else(|| mir_function.blocks.len().checked_sub(1))
    }

    pub fn append_instruction(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        instruction: MirInstruction,
    ) -> Result<(), MirError> {
        self.emit_instruction(
            mir_function,
            block_index,
            instruction.kind,
            instruction.result_type,
            instruction.span,
        )
    }

    pub fn append_terminator(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        instruction: MirInstruction,
    ) -> Result<(), MirError> {
        self.emit_terminator(
            mir_function,
            block_index,
            instruction.kind,
            instruction.result_type,
            instruction.span,
        )
    }

    pub fn connect_blocks(&self, mir_function: &mut MirFunction) {
        mir_function.rebuild_cfg();
    }

    pub fn build(&mut self, hir_module: &HirModule) -> Result<MirModule, MirError> {
        let mut mir_module = MirModule::new(hir_module.name.clone());
        mir_module.metadata.span = Some(hir_module.span);
        mir_module.metadata.root_scope = hir_module.metadata.root_scope.map(|scope| scope.0);

        for global in &hir_module.global_variables {
            let value_type = global
                .var_type
                .as_ref()
                .map(|hir_type| self.lower_hir_type(hir_type))
                .unwrap_or(MirType::Any);
            let value_id = MirValueId::new(mir_module.globals.len());
            mir_module.add_global(MirGlobal {
                name: global.name.clone(),
                storage: format!("global_symbol_{}", global.symbol_id.0),
                value_id,
                value_type,
                symbol_id: Some(global.symbol_id.0),
                span: Some(global.span),
            });

            if let Some(constant) = global
                .initializer
                .as_ref()
                .and_then(Self::literal_mir_value)
            {
                mir_module.add_constant(MirConstant {
                    value_id,
                    value_type: Self::literal_type(&constant),
                    value: constant,
                    span: Some(global.span),
                });
            }
        }

        for hir_function in &hir_module.functions {
            let mir_function = self.lower_function(hir_function)?;
            mir_module.add_function(mir_function);
        }

        Ok(mir_module)
    }

    fn lower_function(&mut self, hir_function: &HirFunction) -> Result<MirFunction, MirError> {
        self.value_counter = 0;
        self.variable_symbols.clear();

        let function_id = MirFunctionId::new(self.function_counter);
        self.function_counter += 1;

        let mut mir_function = MirFunction::new(function_id, hir_function.name.clone());
        let return_type = self.infer_function_return_type(hir_function);
        mir_function.return_type = Some(return_type.clone());
        mir_function.metadata.span = Some(hir_function.span);
        mir_function.metadata.has_explicit_return = hir_function.metadata.has_explicit_return;

        for parameter in &hir_function.parameters {
            self.variable_symbols
                .insert(parameter.id, parameter.symbol_id.0);
            let value_type = parameter
                .param_type
                .as_ref()
                .map(|hir_type| self.lower_hir_type(hir_type))
                .unwrap_or(MirType::Any);
            let storage = self.local_slot_name_from_symbol(parameter.symbol_id.0);
            let parameter_index = mir_function.parameter_data.len();
            let value_id = self.new_value(
                &mut mir_function,
                MirValueKind::Parameter {
                    index: parameter_index,
                    storage: storage.clone(),
                    symbol_id: Some(parameter.symbol_id.0),
                },
                value_type.clone(),
                Some(parameter.span),
            );

            mir_function.add_parameter(MirParameter {
                name: parameter.name.clone(),
                storage: storage.clone(),
                value_id,
                value_type: value_type.clone(),
                symbol_id: Some(parameter.symbol_id.0),
                span: Some(parameter.span),
            });
            mir_function.add_local(MirLocal {
                storage,
                value_id,
                value_type,
                symbol_id: Some(parameter.symbol_id.0),
                span: Some(parameter.span),
            });
        }

        for local in &hir_function.local_variables {
            self.register_local(local, &mut mir_function);
        }

        let previous_exit_block = self.active_exit_block;
        let previous_return_slot = self.active_return_slot.clone();
        let exit_block_id = self.new_block_id();
        self.active_exit_block = Some(exit_block_id);
        self.active_return_slot = (return_type != MirType::Void).then(|| {
            let storage = "local_return".to_string();
            let value_id = self.new_value(
                &mut mir_function,
                MirValueKind::Local {
                    storage: storage.clone(),
                    symbol_id: None,
                },
                return_type.clone(),
                Some(hir_function.span),
            );
            mir_function.add_local(MirLocal {
                storage: storage.clone(),
                value_id,
                value_type: return_type.clone(),
                symbol_id: None,
                span: Some(hir_function.span),
            });
            storage
        });

        let entry_block_id = self.new_block_id();
        let entry_block = MirBasicBlock::with_entry(entry_block_id);
        mir_function.entry_block = Some(0);
        mir_function.add_block(entry_block);

        let locals = mir_function.locals.clone();
        for local in locals {
            self.emit_instruction(
                &mut mir_function,
                0,
                MirInstructionKind::AllocateLocal {
                    local: local.value_id,
                    name: local.storage,
                },
                None,
                local.span,
            )?;
        }

        let current_block = self.lower_statement_block(&hir_function.body, &mut mir_function, 0)?;
        if !mir_function.is_block_terminated(current_block) {
            self.emit_terminator(
                &mut mir_function,
                current_block,
                MirInstructionKind::Jump {
                    target: exit_block_id,
                },
                None,
                Some(hir_function.span),
            )?;
        }

        let exit_index = mir_function.blocks.len();
        let mut exit_block = MirBasicBlock::with_exit(exit_block_id);
        exit_block.span = Some(hir_function.span);
        mir_function.add_block(exit_block);

        if let Some(return_slot) = self.active_return_slot.clone() {
            let return_value = self.new_value(
                &mut mir_function,
                MirValueKind::Temporary,
                return_type,
                Some(hir_function.span),
            );
            let return_value_type = self.value_type(&mir_function, return_value);
            self.emit_instruction(
                &mut mir_function,
                exit_index,
                MirInstructionKind::Load {
                    result: return_value,
                    name: return_slot,
                },
                return_value_type,
                Some(hir_function.span),
            )?;
            self.emit_terminator(
                &mut mir_function,
                exit_index,
                MirInstructionKind::Return {
                    value: Some(return_value),
                },
                Some(MirType::Void),
                Some(hir_function.span),
            )?;
        } else {
            self.emit_terminator(
                &mut mir_function,
                exit_index,
                MirInstructionKind::Return { value: None },
                Some(MirType::Void),
                Some(hir_function.span),
            )?;
        }

        mir_function.rebuild_cfg();
        mir_function.exit_blocks = mir_function
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                block.terminator.as_ref().and_then(|terminator| {
                    matches!(terminator, crate::mir::MirTerminator::Return { .. }).then_some(index)
                })
            })
            .collect();

        self.active_exit_block = previous_exit_block;
        self.active_return_slot = previous_return_slot;

        Ok(mir_function)
    }

    fn register_local(&mut self, local: &HirLocalVariable, mir_function: &mut MirFunction) {
        self.variable_symbols.insert(local.id, local.symbol_id.0);
        let value_type = local
            .var_type
            .as_ref()
            .map(|hir_type| self.lower_hir_type(hir_type))
            .unwrap_or(MirType::Any);
        let storage = self.local_slot_name_from_symbol(local.symbol_id.0);
        let value_id = self.new_value(
            mir_function,
            MirValueKind::Local {
                storage: storage.clone(),
                symbol_id: Some(local.symbol_id.0),
            },
            value_type.clone(),
            Some(local.span),
        );

        mir_function.add_local(MirLocal {
            storage,
            value_id,
            value_type,
            symbol_id: Some(local.symbol_id.0),
            span: Some(local.span),
        });
    }

    fn lower_statement_block(
        &mut self,
        statements: &[HirStatement],
        mir_function: &mut MirFunction,
        mut block_index: usize,
    ) -> Result<usize, MirError> {
        for statement in statements {
            if mir_function.is_block_terminated(block_index) {
                break;
            }
            block_index = self.lower_statement(statement, mir_function, block_index)?;
        }

        Ok(block_index)
    }

    fn lower_statement(
        &mut self,
        statement: &HirStatement,
        mir_function: &mut MirFunction,
        block_index: usize,
    ) -> Result<usize, MirError> {
        match &statement.kind {
            HirStatementKind::LocalVariable {
                variable,
                initializer,
            } => {
                self.variable_symbols
                    .insert(variable.id, variable.symbol_id.0);
                if let Some(initializer) = initializer {
                    let value = self.lower_expression(initializer, mir_function, block_index)?;
                    self.emit_instruction(
                        mir_function,
                        block_index,
                        MirInstructionKind::Store {
                            name: self.local_slot_name_from_symbol(variable.symbol_id.0),
                            value,
                        },
                        None,
                        Some(statement.span),
                    )?;
                }

                Ok(block_index)
            }
            HirStatementKind::Assignment { targets, values } => {
                for (target, value) in targets.iter().zip(values.iter()) {
                    let value_id = self.lower_expression(value, mir_function, block_index)?;
                    self.lower_assignment_target(
                        target,
                        value_id,
                        mir_function,
                        block_index,
                        statement.span,
                    )?;
                }

                Ok(block_index)
            }
            HirStatementKind::Expression(expression) => {
                self.lower_expression_for_effect(expression, mir_function, block_index)?;
                Ok(block_index)
            }
            HirStatementKind::Return(expressions) => {
                let return_value = expressions
                    .as_ref()
                    .and_then(|values| values.first())
                    .map(|expression| self.lower_expression(expression, mir_function, block_index))
                    .transpose()?;

                self.emit_return_to_exit(mir_function, block_index, return_value, statement.span)?;
                Ok(block_index)
            }
            HirStatementKind::Block(statements) => {
                self.lower_statement_block(statements, mir_function, block_index)
            }
            HirStatementKind::If {
                condition,
                then_block,
                else_block,
            } => self.lower_if_statement(
                condition,
                then_block,
                else_block.as_deref(),
                mir_function,
                block_index,
                statement.span,
            ),
            HirStatementKind::While { condition, body } => self.lower_while_statement(
                condition,
                body,
                mir_function,
                block_index,
                statement.span,
            ),
            HirStatementKind::Function { .. } => Ok(block_index),
            HirStatementKind::RepeatUntil { .. }
            | HirStatementKind::ForNumeric { .. }
            | HirStatementKind::ForGeneric { .. }
            | HirStatementKind::Break
            | HirStatementKind::Continue => Err(MirError::LoweringError(format!(
                "unsupported HIR control-flow node during MIR lowering at {}..{}",
                statement.span.start(),
                statement.span.end()
            ))),
            HirStatementKind::Error => Err(MirError::LoweringError(
                "invalid HIR statement reached MIR lowering".to_string(),
            )),
        }
    }

    fn lower_if_statement(
        &mut self,
        condition: &HirExpression,
        then_block: &[HirStatement],
        else_block: Option<&[HirStatement]>,
        mir_function: &mut MirFunction,
        block_index: usize,
        span: SourceSpan,
    ) -> Result<usize, MirError> {
        let condition_value = self.lower_expression(condition, mir_function, block_index)?;
        let then_block_id = self.new_block_id();
        let else_block_id = self.new_block_id();
        let merge_block_id = self.new_block_id();

        self.emit_terminator(
            mir_function,
            block_index,
            MirInstructionKind::Branch {
                condition: condition_value,
                true_block: then_block_id,
                false_block: else_block_id,
            },
            None,
            Some(span),
        )?;

        let then_index = self.push_block(mir_function, then_block_id);
        let then_end = self.lower_statement_block(then_block, mir_function, then_index)?;
        let then_falls_through = !mir_function.is_block_terminated(then_end);
        if then_falls_through {
            self.emit_terminator(
                mir_function,
                then_end,
                MirInstructionKind::Jump {
                    target: merge_block_id,
                },
                None,
                Some(span),
            )?;
        }

        let else_index = self.push_block(mir_function, else_block_id);
        let else_end = if let Some(else_block) = else_block {
            self.lower_statement_block(else_block, mir_function, else_index)?
        } else {
            else_index
        };
        let else_falls_through = !mir_function.is_block_terminated(else_end);
        if else_falls_through {
            self.emit_terminator(
                mir_function,
                else_end,
                MirInstructionKind::Jump {
                    target: merge_block_id,
                },
                None,
                Some(span),
            )?;
        }

        if then_falls_through || else_falls_through {
            Ok(self.push_block(mir_function, merge_block_id))
        } else {
            Ok(then_end)
        }
    }

    fn lower_while_statement(
        &mut self,
        condition: &HirExpression,
        body: &[HirStatement],
        mir_function: &mut MirFunction,
        block_index: usize,
        span: SourceSpan,
    ) -> Result<usize, MirError> {
        let condition_block_id = self.new_block_id();
        let body_block_id = self.new_block_id();
        let exit_block_id = self.new_block_id();

        self.emit_terminator(
            mir_function,
            block_index,
            MirInstructionKind::Jump {
                target: condition_block_id,
            },
            None,
            Some(span),
        )?;

        let condition_index = self.push_block(mir_function, condition_block_id);
        let condition_value = self.lower_expression(condition, mir_function, condition_index)?;
        self.emit_terminator(
            mir_function,
            condition_index,
            MirInstructionKind::Branch {
                condition: condition_value,
                true_block: body_block_id,
                false_block: exit_block_id,
            },
            None,
            Some(span),
        )?;

        let body_index = self.push_block(mir_function, body_block_id);
        let body_end = self.lower_statement_block(body, mir_function, body_index)?;
        if !mir_function.is_block_terminated(body_end) {
            self.emit_terminator(
                mir_function,
                body_end,
                MirInstructionKind::Jump {
                    target: condition_block_id,
                },
                None,
                Some(span),
            )?;
        }

        Ok(self.push_block(mir_function, exit_block_id))
    }

    fn lower_expression_for_effect(
        &mut self,
        expression: &HirExpression,
        mir_function: &mut MirFunction,
        block_index: usize,
    ) -> Result<(), MirError> {
        match &expression.kind {
            HirExpressionKind::FunctionCall { callee, arguments } => {
                self.lower_function_call(
                    callee,
                    arguments,
                    false,
                    mir_function,
                    block_index,
                    expression.span,
                )?;
            }
            HirExpressionKind::BuiltinCall {
                function,
                arguments,
            } => {
                self.lower_builtin_call(
                    function,
                    arguments,
                    false,
                    mir_function,
                    block_index,
                    expression.span,
                )?;
            }
            _ => {
                self.lower_expression(expression, mir_function, block_index)?;
            }
        }

        Ok(())
    }

    fn lower_expression(
        &mut self,
        expression: &HirExpression,
        mir_function: &mut MirFunction,
        block_index: usize,
    ) -> Result<MirValueId, MirError> {
        match &expression.kind {
            HirExpressionKind::Nil => self.emit_const(
                mir_function,
                block_index,
                MirValue::Nil,
                MirType::Any,
                expression.span,
            ),
            HirExpressionKind::Boolean(value) => self.emit_const(
                mir_function,
                block_index,
                MirValue::Boolean(*value),
                MirType::Boolean,
                expression.span,
            ),
            HirExpressionKind::Number(value) => {
                if value.fract() == 0.0 {
                    self.emit_const(
                        mir_function,
                        block_index,
                        MirValue::Integer(*value as i64),
                        MirType::Integer,
                        expression.span,
                    )
                } else {
                    self.emit_const(
                        mir_function,
                        block_index,
                        MirValue::Float(*value),
                        MirType::Float,
                        expression.span,
                    )
                }
            }
            HirExpressionKind::String(value) => self.emit_const(
                mir_function,
                block_index,
                MirValue::String(value.clone()),
                MirType::String,
                expression.span,
            ),
            HirExpressionKind::LocalVariable(variable_id) => {
                let result = self.new_temporary(
                    mir_function,
                    expression.expr_type.as_ref(),
                    expression.span,
                );
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::Load {
                        result,
                        name: self.local_slot_name(*variable_id),
                    },
                    self.value_type(mir_function, result),
                    Some(expression.span),
                )?;
                Ok(result)
            }
            HirExpressionKind::GlobalVariable(name) => {
                let result = self.new_value(
                    mir_function,
                    MirValueKind::Global {
                        name: name.clone(),
                        symbol_id: expression.symbol_id.map(|symbol_id| symbol_id.0),
                    },
                    expression
                        .expr_type
                        .as_ref()
                        .map(|hir_type| self.lower_hir_type(hir_type))
                        .unwrap_or(MirType::Any),
                    Some(expression.span),
                );
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::Load {
                        result,
                        name: self.global_slot_name(
                            name,
                            expression.symbol_id.map(|symbol_id| symbol_id.0),
                        ),
                    },
                    self.value_type(mir_function, result),
                    Some(expression.span),
                )?;
                Ok(result)
            }
            HirExpressionKind::Unary { operator, operand } => self.lower_unary_expression(
                operator,
                operand,
                mir_function,
                block_index,
                expression,
            ),
            HirExpressionKind::Binary {
                left,
                operator,
                right,
            } => self.lower_binary_expression(
                left,
                operator,
                right,
                mir_function,
                block_index,
                expression,
            ),
            HirExpressionKind::TableConstructor(fields) => {
                self.lower_table_constructor(fields, mir_function, block_index, expression)
            }
            HirExpressionKind::Index { object, index } => {
                let table = self.lower_expression(object, mir_function, block_index)?;
                let key = self.lower_expression(index, mir_function, block_index)?;
                let result = self.new_temporary(
                    mir_function,
                    expression.expr_type.as_ref(),
                    expression.span,
                );
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::TableGet { result, table, key },
                    self.value_type(mir_function, result),
                    Some(expression.span),
                )?;
                Ok(result)
            }
            HirExpressionKind::FieldAccess { object, field } => {
                let table = self.lower_expression(object, mir_function, block_index)?;
                let key = self.emit_const(
                    mir_function,
                    block_index,
                    MirValue::String(field.clone()),
                    MirType::String,
                    expression.span,
                )?;
                let result = self.new_temporary(
                    mir_function,
                    expression.expr_type.as_ref(),
                    expression.span,
                );
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::TableGet { result, table, key },
                    self.value_type(mir_function, result),
                    Some(expression.span),
                )?;
                Ok(result)
            }
            HirExpressionKind::FunctionCall { callee, arguments } => self
                .lower_function_call(
                    callee,
                    arguments,
                    true,
                    mir_function,
                    block_index,
                    expression.span,
                )?
                .ok_or_else(|| {
                    MirError::LoweringError("function call produced no value".to_string())
                }),
            HirExpressionKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                let mut lowered_arguments = Vec::with_capacity(arguments.len() + 1);
                lowered_arguments.push(self.lower_expression(
                    receiver,
                    mir_function,
                    block_index,
                )?);
                for argument in arguments {
                    lowered_arguments.push(self.lower_expression(
                        argument,
                        mir_function,
                        block_index,
                    )?);
                }

                let result = self.new_temporary(
                    mir_function,
                    expression.expr_type.as_ref(),
                    expression.span,
                );
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::Call {
                        result: Some(result),
                        function: method.clone(),
                        arguments: lowered_arguments,
                    },
                    self.value_type(mir_function, result),
                    Some(expression.span),
                )?;
                Ok(result)
            }
            HirExpressionKind::BuiltinCall {
                function,
                arguments,
            } => self
                .lower_builtin_call(
                    function,
                    arguments,
                    true,
                    mir_function,
                    block_index,
                    expression.span,
                )?
                .ok_or_else(|| {
                    MirError::LoweringError("builtin call produced no value".to_string())
                }),
            HirExpressionKind::InterpolatedString(parts) => {
                let mut text = String::new();
                for part in parts {
                    match part {
                        crate::hir::HirInterpolatedStringPart::Text(part_text) => {
                            text.push_str(part_text);
                        }
                        crate::hir::HirInterpolatedStringPart::Expression(expression) => {
                            self.lower_expression(expression, mir_function, block_index)?;
                        }
                    }
                }
                self.emit_const(
                    mir_function,
                    block_index,
                    MirValue::String(text),
                    MirType::String,
                    expression.span,
                )
            }
            HirExpressionKind::ClosurePlaceholder | HirExpressionKind::Error => Err(
                MirError::LoweringError("invalid HIR expression reached MIR lowering".to_string()),
            ),
        }
    }

    fn lower_assignment_target(
        &mut self,
        target: &HirExpression,
        value: MirValueId,
        mir_function: &mut MirFunction,
        block_index: usize,
        span: SourceSpan,
    ) -> Result<(), MirError> {
        match &target.kind {
            HirExpressionKind::LocalVariable(variable_id) => self.emit_instruction(
                mir_function,
                block_index,
                MirInstructionKind::Store {
                    name: self.local_slot_name(*variable_id),
                    value,
                },
                None,
                Some(span),
            ),
            HirExpressionKind::GlobalVariable(name) => self.emit_instruction(
                mir_function,
                block_index,
                MirInstructionKind::Store {
                    name: self
                        .global_slot_name(name, target.symbol_id.map(|symbol_id| symbol_id.0)),
                    value,
                },
                None,
                Some(span),
            ),
            HirExpressionKind::Index { object, index } => {
                let table = self.lower_expression(object, mir_function, block_index)?;
                let key = self.lower_expression(index, mir_function, block_index)?;
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::TableSet { table, key, value },
                    None,
                    Some(span),
                )
            }
            HirExpressionKind::FieldAccess { object, field } => {
                let table = self.lower_expression(object, mir_function, block_index)?;
                let key = self.emit_const(
                    mir_function,
                    block_index,
                    MirValue::String(field.clone()),
                    MirType::String,
                    span,
                )?;
                self.emit_instruction(
                    mir_function,
                    block_index,
                    MirInstructionKind::TableSet { table, key, value },
                    None,
                    Some(span),
                )
            }
            _ => Err(MirError::LoweringError(format!(
                "invalid assignment target reached MIR lowering at {}..{}",
                target.span.start(),
                target.span.end()
            ))),
        }
    }

    fn lower_unary_expression(
        &mut self,
        operator: &HirUnaryOperator,
        operand: &HirExpression,
        mir_function: &mut MirFunction,
        block_index: usize,
        expression: &HirExpression,
    ) -> Result<MirValueId, MirError> {
        let operand_id = self.lower_expression(operand, mir_function, block_index)?;
        let result =
            self.new_temporary(mir_function, expression.expr_type.as_ref(), expression.span);

        let kind = match operator {
            HirUnaryOperator::Not => MirInstructionKind::Not {
                result,
                operand: operand_id,
            },
            HirUnaryOperator::Negate => {
                let zero = self.emit_const(
                    mir_function,
                    block_index,
                    MirValue::Integer(0),
                    MirType::Integer,
                    expression.span,
                )?;
                MirInstructionKind::Subtract {
                    result,
                    left: zero,
                    right: operand_id,
                }
            }
            HirUnaryOperator::Length | HirUnaryOperator::BitwiseNot => {
                return Err(MirError::LoweringError(format!(
                    "unsupported unary operator in MIR lowering at {}..{}",
                    expression.span.start(),
                    expression.span.end()
                )));
            }
        };

        self.emit_instruction(
            mir_function,
            block_index,
            kind,
            self.value_type(mir_function, result),
            Some(expression.span),
        )?;
        Ok(result)
    }

    fn lower_binary_expression(
        &mut self,
        left: &HirExpression,
        operator: &HirBinaryOperator,
        right: &HirExpression,
        mir_function: &mut MirFunction,
        block_index: usize,
        expression: &HirExpression,
    ) -> Result<MirValueId, MirError> {
        let left_id = self.lower_expression(left, mir_function, block_index)?;
        let right_id = self.lower_expression(right, mir_function, block_index)?;
        let result =
            self.new_temporary(mir_function, expression.expr_type.as_ref(), expression.span);

        let kind = match operator {
            HirBinaryOperator::Add => MirInstructionKind::Add {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Subtract => MirInstructionKind::Subtract {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Multiply => MirInstructionKind::Multiply {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Divide | HirBinaryOperator::FloorDivide => {
                MirInstructionKind::Divide {
                    result,
                    left: left_id,
                    right: right_id,
                }
            }
            HirBinaryOperator::Modulo => MirInstructionKind::Modulo {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Equal => MirInstructionKind::Equal {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::NotEqual => MirInstructionKind::NotEqual {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::LessThan => MirInstructionKind::LessThan {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::LessEqual => MirInstructionKind::LessEqual {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::GreaterThan => MirInstructionKind::GreaterThan {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::GreaterEqual => MirInstructionKind::GreaterEqual {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::And => MirInstructionKind::And {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Or => MirInstructionKind::Or {
                result,
                left: left_id,
                right: right_id,
            },
            HirBinaryOperator::Concatenate
            | HirBinaryOperator::Exponent
            | HirBinaryOperator::BitwiseAnd
            | HirBinaryOperator::BitwiseOr
            | HirBinaryOperator::BitwiseXor
            | HirBinaryOperator::BitwiseShiftLeft
            | HirBinaryOperator::BitwiseShiftRight => {
                return Err(MirError::LoweringError(format!(
                    "unsupported binary operator in MIR lowering at {}..{}",
                    expression.span.start(),
                    expression.span.end()
                )));
            }
        };

        self.emit_instruction(
            mir_function,
            block_index,
            kind,
            self.value_type(mir_function, result),
            Some(expression.span),
        )?;
        Ok(result)
    }

    fn lower_function_call(
        &mut self,
        callee: &HirExpression,
        arguments: &[HirExpression],
        needs_result: bool,
        mir_function: &mut MirFunction,
        block_index: usize,
        span: SourceSpan,
    ) -> Result<Option<MirValueId>, MirError> {
        let mut argument_ids = Vec::with_capacity(arguments.len());
        for argument in arguments {
            argument_ids.push(self.lower_expression(argument, mir_function, block_index)?);
        }

        let function_name = match &callee.kind {
            HirExpressionKind::GlobalVariable(name) => name.clone(),
            HirExpressionKind::LocalVariable(_) => {
                let callee_value = self.lower_expression(callee, mir_function, block_index)?;
                format!("value_{}", callee_value.0)
            }
            _ => {
                return Err(MirError::LoweringError(format!(
                    "unsupported function callee in MIR lowering at {}..{}",
                    callee.span.start(),
                    callee.span.end()
                )));
            }
        };

        let result =
            needs_result.then(|| self.new_temporary(mir_function, callee.expr_type.as_ref(), span));
        self.emit_instruction(
            mir_function,
            block_index,
            MirInstructionKind::Call {
                result,
                function: function_name,
                arguments: argument_ids,
            },
            result.and_then(|id| self.value_type(mir_function, id)),
            Some(span),
        )?;

        Ok(result)
    }

    fn lower_builtin_call(
        &mut self,
        function: &HirBuiltinFunction,
        arguments: &[HirExpression],
        needs_result: bool,
        mir_function: &mut MirFunction,
        block_index: usize,
        span: SourceSpan,
    ) -> Result<Option<MirValueId>, MirError> {
        let mut argument_ids = Vec::with_capacity(arguments.len());
        for argument in arguments {
            argument_ids.push(self.lower_expression(argument, mir_function, block_index)?);
        }

        let (function_name, return_type) = self.lower_builtin(function);
        let result = (needs_result && return_type != MirType::Void).then(|| {
            self.new_value(
                mir_function,
                MirValueKind::Temporary,
                return_type.clone(),
                Some(span),
            )
        });

        self.emit_instruction(
            mir_function,
            block_index,
            MirInstructionKind::Call {
                result,
                function: function_name.to_string(),
                arguments: argument_ids,
            },
            Some(return_type.clone()),
            Some(span),
        )?;

        if needs_result && result.is_none() {
            return self
                .emit_const(mir_function, block_index, MirValue::Nil, MirType::Any, span)
                .map(Some);
        }

        Ok(result)
    }

    fn lower_table_constructor(
        &mut self,
        fields: &[HirTableField],
        mir_function: &mut MirFunction,
        block_index: usize,
        expression: &HirExpression,
    ) -> Result<MirValueId, MirError> {
        let table = self.new_value(
            mir_function,
            MirValueKind::Temporary,
            MirType::Table,
            Some(expression.span),
        );
        self.emit_instruction(
            mir_function,
            block_index,
            MirInstructionKind::TableNew { result: table },
            Some(MirType::Table),
            Some(expression.span),
        )?;

        for (index, field) in fields.iter().enumerate() {
            let (key, value) = match field {
                HirTableField::Named { key, value } => {
                    let key = self.emit_const(
                        mir_function,
                        block_index,
                        MirValue::String(key.clone()),
                        MirType::String,
                        value.span,
                    )?;
                    (
                        key,
                        self.lower_expression(value, mir_function, block_index)?,
                    )
                }
                HirTableField::Indexed { key, value } => (
                    self.lower_expression(key, mir_function, block_index)?,
                    self.lower_expression(value, mir_function, block_index)?,
                ),
                HirTableField::Expression(value) => {
                    let key = self.emit_const(
                        mir_function,
                        block_index,
                        MirValue::Integer(index as i64 + 1),
                        MirType::Integer,
                        value.span,
                    )?;
                    (
                        key,
                        self.lower_expression(value, mir_function, block_index)?,
                    )
                }
            };

            self.emit_instruction(
                mir_function,
                block_index,
                MirInstructionKind::TableSet { table, key, value },
                None,
                Some(expression.span),
            )?;
        }

        Ok(table)
    }

    fn emit_const(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        value: MirValue,
        value_type: MirType,
        span: SourceSpan,
    ) -> Result<MirValueId, MirError> {
        let result = self.new_value(
            mir_function,
            MirValueKind::Constant,
            value_type.clone(),
            Some(span),
        );
        self.emit_instruction(
            mir_function,
            block_index,
            MirInstructionKind::Const { result, value },
            Some(value_type),
            Some(span),
        )?;
        Ok(result)
    }

    fn emit_return_to_exit(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        return_value: Option<MirValueId>,
        span: SourceSpan,
    ) -> Result<(), MirError> {
        if let (Some(return_slot), Some(return_value)) =
            (self.active_return_slot.clone(), return_value)
        {
            self.emit_instruction(
                mir_function,
                block_index,
                MirInstructionKind::Store {
                    name: return_slot,
                    value: return_value,
                },
                None,
                Some(span),
            )?;
        }

        let exit_block = self.active_exit_block.ok_or_else(|| {
            MirError::LoweringError("return lowered without an active exit block".to_string())
        })?;

        self.emit_terminator(
            mir_function,
            block_index,
            MirInstructionKind::Jump { target: exit_block },
            None,
            Some(span),
        )
    }

    fn emit_instruction(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        kind: MirInstructionKind,
        result_type: Option<MirType>,
        span: Option<SourceSpan>,
    ) -> Result<(), MirError> {
        if mir_function.is_block_terminated(block_index) {
            return Err(MirError::LoweringError(format!(
                "attempted to emit instruction after terminator in block {}",
                mir_function.blocks[block_index].id.0
            )));
        }

        mir_function.add_instruction(block_index, MirInstruction::new(kind, result_type, span));
        Ok(())
    }

    fn emit_terminator(
        &mut self,
        mir_function: &mut MirFunction,
        block_index: usize,
        kind: MirInstructionKind,
        result_type: Option<MirType>,
        span: Option<SourceSpan>,
    ) -> Result<(), MirError> {
        if mir_function.is_block_terminated(block_index) {
            return Err(MirError::LoweringError(format!(
                "block {} already has a terminator",
                mir_function.blocks[block_index].id.0
            )));
        }

        mir_function.add_instruction(block_index, MirInstruction::new(kind, result_type, span));
        Ok(())
    }

    fn new_value(
        &mut self,
        mir_function: &mut MirFunction,
        kind: MirValueKind,
        value_type: MirType,
        span: Option<SourceSpan>,
    ) -> MirValueId {
        let id = MirValueId::new(self.value_counter);
        self.value_counter += 1;
        mir_function.add_value(MirValueData::new(id, value_type, kind, span));
        id
    }

    fn new_temporary(
        &mut self,
        mir_function: &mut MirFunction,
        hir_type: Option<&HirType>,
        span: SourceSpan,
    ) -> MirValueId {
        let value_type = hir_type
            .map(|hir_type| self.lower_hir_type(hir_type))
            .unwrap_or(MirType::Any);
        self.new_value(
            mir_function,
            MirValueKind::Temporary,
            value_type,
            Some(span),
        )
    }

    fn value_type(&self, mir_function: &MirFunction, value_id: MirValueId) -> Option<MirType> {
        mir_function
            .values
            .iter()
            .find(|value| value.id == value_id)
            .map(|value| value.value_type.clone())
    }

    fn new_block_id(&mut self) -> MirBlockId {
        let id = MirBlockId::new(self.block_counter);
        self.block_counter += 1;
        id
    }

    fn push_block(&mut self, mir_function: &mut MirFunction, id: MirBlockId) -> usize {
        let index = mir_function.blocks.len();
        mir_function.add_block(MirBasicBlock::new(id));
        index
    }

    fn local_slot_name(&self, id: HirVariableId) -> String {
        self.variable_symbols
            .get(&id)
            .map(|symbol_id| self.local_slot_name_from_symbol(*symbol_id))
            .unwrap_or_else(|| format!("local_value_{}", id.0))
    }

    fn local_slot_name_from_symbol(&self, symbol_id: usize) -> String {
        format!("local_symbol_{symbol_id}")
    }

    fn global_slot_name(&self, name: &str, symbol_id: Option<usize>) -> String {
        symbol_id
            .map(|symbol_id| format!("global_symbol_{symbol_id}"))
            .unwrap_or_else(|| name.to_string())
    }

    fn infer_function_return_type(&self, hir_function: &HirFunction) -> MirType {
        if let Some(return_type) = hir_function.return_type.as_ref() {
            return self.lower_hir_type(return_type);
        }

        self.find_first_return_type(&hir_function.body)
            .map(|hir_type| self.lower_hir_type(hir_type))
            .unwrap_or(MirType::Void)
    }

    fn find_first_return_type<'a>(&self, statements: &'a [HirStatement]) -> Option<&'a HirType> {
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Return(Some(values)) => {
                    if let Some(value) = values.first().and_then(|value| value.expr_type.as_ref()) {
                        return Some(value);
                    }
                }
                HirStatementKind::Block(statements) => {
                    if let Some(value_type) = self.find_first_return_type(statements) {
                        return Some(value_type);
                    }
                }
                HirStatementKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    if let Some(value_type) = self.find_first_return_type(then_block) {
                        return Some(value_type);
                    }
                    if let Some(else_block) = else_block {
                        if let Some(value_type) = self.find_first_return_type(else_block) {
                            return Some(value_type);
                        }
                    }
                }
                _ => {}
            }
        }

        None
    }

    fn lower_hir_type(&self, hir_type: &HirType) -> MirType {
        match hir_type {
            HirType::Nil => MirType::Void,
            HirType::Boolean => MirType::Boolean,
            HirType::Integer => MirType::Integer,
            HirType::Number => MirType::Integer,
            HirType::String => MirType::String,
            HirType::Table => MirType::Table,
            HirType::Function => MirType::Function,
            HirType::Thread | HirType::Userdata | HirType::Any | HirType::Unknown => MirType::Any,
        }
    }

    fn lower_builtin(&self, function: &HirBuiltinFunction) -> (&'static str, MirType) {
        match function {
            HirBuiltinFunction::Print => ("glua_print", MirType::Void),
            HirBuiltinFunction::Type => ("glua_type", MirType::String),
            HirBuiltinFunction::ToNumber => ("glua_tonumber", MirType::Integer),
            HirBuiltinFunction::ToString => ("glua_tostring", MirType::String),
            HirBuiltinFunction::Error => ("glua_error", MirType::Void),
            HirBuiltinFunction::Pairs => ("glua_pairs", MirType::Function),
            HirBuiltinFunction::Ipairs => ("glua_ipairs", MirType::Function),
            HirBuiltinFunction::Require => ("glua_require", MirType::Any),
        }
    }

    fn literal_mir_value(expression: &HirExpression) -> Option<MirValue> {
        match &expression.kind {
            HirExpressionKind::Nil => Some(MirValue::Nil),
            HirExpressionKind::Boolean(value) => Some(MirValue::Boolean(*value)),
            HirExpressionKind::Number(value) if value.fract() == 0.0 => {
                Some(MirValue::Integer(*value as i64))
            }
            HirExpressionKind::Number(value) => Some(MirValue::Float(*value)),
            HirExpressionKind::String(value) => Some(MirValue::String(value.clone())),
            _ => None,
        }
    }

    fn literal_type(value: &MirValue) -> MirType {
        match value {
            MirValue::Integer(_) => MirType::Integer,
            MirValue::Float(_) => MirType::Float,
            MirValue::Boolean(_) => MirType::Boolean,
            MirValue::String(_) => MirType::String,
            MirValue::Nil => MirType::Any,
            MirValue::Unit => MirType::Void,
        }
    }
}

impl Default for MirBuilder {
    fn default() -> Self {
        Self::new()
    }
}
