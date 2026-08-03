use crate::hir::{HirModule, HirFunction, HirStatement, HirExpression, HirExpressionKind, HirStatementKind, HirType, HirVariableId};
use crate::mir::error::MirError;
use crate::mir::module::MirModule;
use crate::mir::function::MirFunction;
use crate::mir::block::MirBasicBlock;
use crate::mir::instruction::{MirInstruction, MirInstructionKind};
use crate::mir::types::{MirValueId, MirBlockId, MirFunctionId, MirType, MirValue};

pub struct MirBuilder {
    value_counter: usize,
    block_counter: usize,
    function_counter: usize,
}

impl MirBuilder {
    pub fn new() -> Self {
        Self {
            value_counter: 0,
            block_counter: 0,
            function_counter: 0,
        }
    }
    
    pub fn build(&mut self, hir_module: &HirModule) -> Result<MirModule, MirError> {
        let mut mir_module = MirModule::new(hir_module.name.clone());
        
        for hir_function in &hir_module.functions {
            let mir_function = self.lower_function(hir_function)?;
            mir_module.add_function(mir_function);
        }
        
        Ok(mir_module)
    }
    
    fn lower_function(&mut self, hir_function: &HirFunction) -> Result<MirFunction, MirError> {
        let function_id = MirFunctionId::new(self.function_counter);
        self.function_counter += 1;
        
        let mut mir_function = MirFunction::new(function_id, hir_function.name.clone());
        mir_function.return_type = self.infer_function_return_type(hir_function);
        
        // Create entry block
        let entry_block_id = MirBlockId::new(self.block_counter);
        self.block_counter += 1;
        let entry_block = MirBasicBlock::with_entry(entry_block_id);
        mir_function.entry_block = Some(0);
        mir_function.add_block(entry_block);
        
        // Lower function body
        for statement in &hir_function.body {
            self.lower_statement(statement, &mut mir_function, 0)?;
        }
        
        // Add return if none exists
        if !mir_function.blocks.last().map_or(false, |b| {
            b.instructions.iter().any(|i| matches!(i.kind, MirInstructionKind::Return { .. }))
        }) {
            let ret_instr = MirInstruction {
                kind: MirInstructionKind::Return { value: None },
                result_type: Some(MirType::Void),
            };
            mir_function.add_instruction(mir_function.blocks.len() - 1, ret_instr);
        }
        
        Ok(mir_function)
    }
    
    fn lower_statement(&mut self, statement: &HirStatement, mir_function: &mut MirFunction, block_index: usize) -> Result<(), MirError> {
        match &statement.kind {
            HirStatementKind::LocalVariable { variable, initializer } => {
                if let Some(init) = initializer {
                    let init_value_id = self.lower_expression(init, mir_function, block_index)?;
                    let store_instr = MirInstruction {
                        kind: MirInstructionKind::Store {
                            name: self.local_slot_name(variable.id),
                            value: init_value_id,
                        },
                        result_type: None,
                    };
                    mir_function.add_instruction(block_index, store_instr);
                }
            }
            
            HirStatementKind::Assignment { targets, values } => {
                for (target, value) in targets.iter().zip(values.iter()) {
                    let value_id = self.lower_expression(value, mir_function, block_index)?;
                    if let Some(name) = self.assignment_target_name(target) {
                        let store_instr = MirInstruction {
                            kind: MirInstructionKind::Store {
                                name,
                                value: value_id,
                            },
                            result_type: None,
                        };
                        mir_function.add_instruction(block_index, store_instr);
                    }
                }
            }
            
            HirStatementKind::Expression(expr) => {
                if let HirExpressionKind::FunctionCall { callee, arguments } = &expr.kind {
                    let mut arg_ids = Vec::new();
                    for arg in arguments {
                        arg_ids.push(self.lower_expression(arg, mir_function, block_index)?);
                    }

                    let function_name = if let HirExpressionKind::GlobalVariable(name) = &callee.kind {
                        name.clone()
                    } else {
                        "unknown".to_string()
                    };

                    let call_instr = MirInstruction {
                        kind: MirInstructionKind::Call {
                            result: None,
                            function: function_name,
                            arguments: arg_ids,
                        },
                        result_type: Some(MirType::Void),
                    };
                    mir_function.add_instruction(block_index, call_instr);
                } else {
                    self.lower_expression(expr, mir_function, block_index)?;
                }
            }
            
            HirStatementKind::Return(exprs) => {
                let return_value = if let Some(exprs) = exprs {
                    if let Some(first) = exprs.first() {
                        Some(self.lower_expression(first, mir_function, block_index)?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                let ret_instr = MirInstruction {
                    kind: MirInstructionKind::Return { value: return_value },
                    result_type: Some(MirType::Void),
                };
                mir_function.add_instruction(block_index, ret_instr);
            }
            
            HirStatementKind::Block(statements) => {
                for stmt in statements {
                    self.lower_statement(stmt, mir_function, block_index)?;
                }
            }
            
            // Control flow statements - simplified for initial implementation
            HirStatementKind::If { condition, then_block, else_block } => {
                let cond_value_id = self.lower_expression(condition, mir_function, block_index)?;
                
                let then_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                let else_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                let merge_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                
                let branch_instr = MirInstruction {
                    kind: MirInstructionKind::Branch {
                        condition: cond_value_id,
                        true_block: then_block_id,
                        false_block: else_block_id,
                    },
                    result_type: None,
                };
                mir_function.add_instruction(block_index, branch_instr);
                
                // Add then block
                let then_block_idx = mir_function.blocks.len();
                let then_block_basic = MirBasicBlock::new(then_block_id);
                mir_function.add_block(then_block_basic);
                
                for stmt in then_block {
                    self.lower_statement(stmt, mir_function, then_block_idx)?;
                }
                
                let jump_to_merge = MirInstruction {
                    kind: MirInstructionKind::Jump { target: merge_block_id },
                    result_type: None,
                };
                mir_function.add_instruction(then_block_idx, jump_to_merge);
                
                // Add else block
                let else_block_idx = mir_function.blocks.len();
                let else_block_basic = MirBasicBlock::new(else_block_id);
                mir_function.add_block(else_block_basic);
                
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.lower_statement(stmt, mir_function, else_block_idx)?;
                    }
                }
                
                let jump_to_merge_else = MirInstruction {
                    kind: MirInstructionKind::Jump { target: merge_block_id },
                    result_type: None,
                };
                mir_function.add_instruction(else_block_idx, jump_to_merge_else);
                
                // Add merge block
                let merge_block = MirBasicBlock::new(merge_block_id);
                mir_function.add_block(merge_block);
            }
            
            HirStatementKind::While { condition, body } => {
                let cond_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                let body_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                let exit_block_id = MirBlockId::new(self.block_counter);
                self.block_counter += 1;
                
                // Jump to condition block
                let jump_to_cond = MirInstruction {
                    kind: MirInstructionKind::Jump { target: cond_block_id },
                    result_type: None,
                };
                mir_function.add_instruction(block_index, jump_to_cond);
                
                // Condition block
                let cond_block = MirBasicBlock::new(cond_block_id);
                let cond_block_index = mir_function.blocks.len();
                mir_function.add_block(cond_block);
                
                let cond_value_id = self.lower_expression(condition, mir_function, cond_block_index)?;
                
                let branch_instr = MirInstruction {
                    kind: MirInstructionKind::Branch {
                        condition: cond_value_id,
                        true_block: body_block_id,
                        false_block: exit_block_id,
                    },
                    result_type: None,
                };
                mir_function.add_instruction(cond_block_index, branch_instr);
                
                // Body block
                let body_block_idx = mir_function.blocks.len();
                let body_block_basic = MirBasicBlock::new(body_block_id);
                mir_function.add_block(body_block_basic);
                
                for stmt in body {
                    self.lower_statement(stmt, mir_function, body_block_idx)?;
                }
                
                let jump_back_to_cond = MirInstruction {
                    kind: MirInstructionKind::Jump { target: cond_block_id },
                    result_type: None,
                };
                mir_function.add_instruction(body_block_idx, jump_back_to_cond);
                
                // Exit block
                let exit_block = MirBasicBlock::new(exit_block_id);
                mir_function.add_block(exit_block);
            }
            
            // Simplified handling for other control flow
            HirStatementKind::RepeatUntil { .. } | HirStatementKind::ForNumeric { .. } | 
            HirStatementKind::ForGeneric { .. } | HirStatementKind::Break | 
            HirStatementKind::Continue => {
                // For initial implementation, just skip these complex control flow structures
            }
            
            HirStatementKind::Function { .. } => {
                // Nested functions not implemented yet
            }
            
            HirStatementKind::Error => {
                // Skip error statements
            }
        }
        
        Ok(())
    }
    
    fn lower_expression(&mut self, expr: &HirExpression, mir_function: &mut MirFunction, block_index: usize) -> Result<MirValueId, MirError> {
        let result_id = MirValueId::new(self.value_counter);
        self.value_counter += 1;
        
        let instruction = match &expr.kind {
            HirExpressionKind::Nil => {
                MirInstruction {
                    kind: MirInstructionKind::Const {
                        result: result_id,
                        value: MirValue::Nil,
                    },
                    result_type: Some(MirType::Any),
                }
            }
            
            HirExpressionKind::Boolean(b) => {
                MirInstruction {
                    kind: MirInstructionKind::Const {
                        result: result_id,
                        value: MirValue::Boolean(*b),
                    },
                    result_type: Some(MirType::Boolean),
                }
            }
            
            HirExpressionKind::Number(n) => {
                let (mir_value, result_type) = if n.fract() == 0.0 {
                    (MirValue::Integer(*n as i64), MirType::Integer)
                } else {
                    (MirValue::Float(*n), MirType::Float)
                };
                MirInstruction {
                    kind: MirInstructionKind::Const {
                        result: result_id,
                        value: mir_value,
                    },
                    result_type: Some(result_type),
                }
            }
            
            HirExpressionKind::String(s) => {
                MirInstruction {
                    kind: MirInstructionKind::Const {
                        result: result_id,
                        value: MirValue::String(s.clone()),
                    },
                    result_type: Some(MirType::String),
                }
            }
            
            HirExpressionKind::LocalVariable(id) => {
                MirInstruction {
                    kind: MirInstructionKind::Load {
                        result: result_id,
                        name: self.local_slot_name(*id),
                    },
                    result_type: Some(MirType::Any),
                }
            }
            
            HirExpressionKind::GlobalVariable(name) => {
                MirInstruction {
                    kind: MirInstructionKind::Load {
                        result: result_id,
                        name: name.clone(),
                    },
                    result_type: Some(MirType::Any),
                }
            }
            
            HirExpressionKind::Binary { left, operator, right } => {
                let left_id = self.lower_expression(left, mir_function, block_index)?;
                let right_id = self.lower_expression(right, mir_function, block_index)?;
                
                let kind = match operator {
                    crate::hir::types::HirBinaryOperator::Add => MirInstructionKind::Add {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::Subtract => MirInstructionKind::Subtract {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::Multiply => MirInstructionKind::Multiply {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::Divide => MirInstructionKind::Divide {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::Equal => MirInstructionKind::Equal {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::And => MirInstructionKind::And {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    crate::hir::types::HirBinaryOperator::Or => MirInstructionKind::Or {
                        result: result_id,
                        left: left_id,
                        right: right_id,
                    },
                    _ => MirInstructionKind::Error,
                };
                
                MirInstruction {
                    kind,
                    result_type: Some(MirType::Any),
                }
            }
            
            HirExpressionKind::Unary { operator, operand } => {
                let operand_id = self.lower_expression(operand, mir_function, block_index)?;
                
                let kind = match operator {
                    crate::hir::types::HirUnaryOperator::Not => MirInstructionKind::Not {
                        result: result_id,
                        operand: operand_id,
                    },
                    _ => MirInstructionKind::Error,
                };
                
                MirInstruction {
                    kind,
                    result_type: Some(MirType::Any),
                }
            }
            
            HirExpressionKind::FunctionCall { callee, arguments } => {
                let mut arg_ids = Vec::new();
                for arg in arguments {
                    arg_ids.push(self.lower_expression(arg, mir_function, block_index)?);
                }
                
                let function_name = if let HirExpressionKind::GlobalVariable(name) = &callee.kind {
                    name.clone()
                } else {
                    "unknown".to_string()
                };
                
                MirInstruction {
                    kind: MirInstructionKind::Call {
                        result: Some(result_id),
                        function: function_name,
                        arguments: arg_ids,
                    },
                    result_type: Some(MirType::Any),
                }
            }

            HirExpressionKind::BuiltinCall { function, arguments } => {
                let mut arg_ids = Vec::new();
                for arg in arguments {
                    arg_ids.push(self.lower_expression(arg, mir_function, block_index)?);
                }

                let function_name = match function {
                    crate::hir::types::HirBuiltinFunction::Print => "glua_print",
                    crate::hir::types::HirBuiltinFunction::Type => "glua_type",
                    crate::hir::types::HirBuiltinFunction::ToNumber => "glua_tonumber",
                    crate::hir::types::HirBuiltinFunction::ToString => "glua_tostring",
                    crate::hir::types::HirBuiltinFunction::Error => "glua_error",
                    crate::hir::types::HirBuiltinFunction::Pairs => "glua_pairs",
                    crate::hir::types::HirBuiltinFunction::Ipairs => "glua_ipairs",
                    crate::hir::types::HirBuiltinFunction::Require => "glua_require",
                }
                .to_string();

                MirInstruction {
                    kind: MirInstructionKind::Call {
                        result: None,
                        function: function_name,
                        arguments: arg_ids,
                    },
                    result_type: Some(MirType::Void),
                }
            }
            
            HirExpressionKind::TableConstructor(_) => {
                MirInstruction {
                    kind: MirInstructionKind::TableNew {
                        result: result_id,
                    },
                    result_type: Some(MirType::Table),
                }
            }
            
            HirExpressionKind::ClosurePlaceholder => {
                MirInstruction {
                    kind: MirInstructionKind::Error,
                    result_type: None,
                }
            }
            
            _ => MirInstruction {
                kind: MirInstructionKind::Error,
                result_type: None,
            },
        };
        
        mir_function.add_instruction(block_index, instruction);
        Ok(result_id)
    }

    fn local_slot_name(&self, id: HirVariableId) -> String {
        format!("local_{}", id.0)
    }

    fn assignment_target_name(&self, target: &HirExpression) -> Option<String> {
        match &target.kind {
            HirExpressionKind::LocalVariable(id) => Some(self.local_slot_name(*id)),
            HirExpressionKind::GlobalVariable(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn infer_function_return_type(&self, hir_function: &HirFunction) -> Option<MirType> {
        if let Some(return_type) = hir_function.return_type.as_ref() {
            return Some(self.lower_hir_type(return_type));
        }

        if hir_function.body.iter().any(|statement| {
            matches!(
                &statement.kind,
                HirStatementKind::Return(Some(values)) if !values.is_empty()
            )
        }) {
            return Some(MirType::Integer);
        }

        None
    }

    fn lower_hir_type(&self, hir_type: &HirType) -> MirType {
        match hir_type {
            HirType::Nil => MirType::Any,
            HirType::Boolean => MirType::Boolean,
            HirType::Number => MirType::Integer,
            HirType::String => MirType::String,
            HirType::Table => MirType::Table,
            HirType::Function => MirType::Function,
            HirType::Any => MirType::Any,
            HirType::Unknown => MirType::Unknown,
        }
    }
}
