use crate::mir::MirModule;
use crate::mir::function::MirFunction;
use crate::mir::block::MirBasicBlock;
use crate::mir::instruction::{MirInstruction, MirInstructionKind};
use crate::mir::types::{MirValue, MirType};
use crate::llvm::error::LlvmError;
use crate::llvm::types::{LlvmType, map_mir_type};

pub struct LlvmGenerator {
    module_name: String,
}

impl LlvmGenerator {
    pub fn new(module_name: String) -> Self {
        Self { module_name }
    }
    
    pub fn generate(&mut self, mir: &MirModule) -> Result<String, LlvmError> {
        let mut ir = String::new();
        
        // Module header
        ir.push_str(&format!("; Module ID: '{}'\n", self.module_name));
        ir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n");
        ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        ir.push_str("\n");
        
        // Generate function declarations
        self.generate_runtime_declarations(&mut ir);
        
        // Generate functions
        for function in &mir.functions {
            self.generate_function(function, &mut ir)?;
        }
        
        Ok(ir)
    }
    
    fn generate_runtime_declarations(&self, ir: &mut String) {
        // Declare runtime functions
        ir.push_str("declare void @glua_print(i8*)\n");
        ir.push_str("declare i8* @glua_table_new()\n");
        ir.push_str("declare void @glua_table_set(i8*, i8*, i8*)\n");
        ir.push_str("declare i8* @glua_table_get(i8*, i8*)\n");
        ir.push_str("\n");
    }
    
    fn generate_function(&self, function: &MirFunction, ir: &mut String) -> Result<(), LlvmError> {
        let return_type = map_mir_type(function.return_type.as_ref().unwrap_or(&MirType::Void));
        
        // Function signature
        ir.push_str(&format!("define {} @{}(", return_type.to_string(), function.name));
        
        // Parameters
        let param_types: Vec<String> = function.parameters.iter()
            .map(|_| "i8*".to_string())
            .collect();
        ir.push_str(&param_types.join(", "));
        
        ir.push_str(") {\n");
        
        // Generate basic blocks
        for block in &function.blocks {
            self.generate_block(block, ir)?;
        }
        
        ir.push_str("}\n\n");
        
        Ok(())
    }
    
    fn generate_block(&self, block: &MirBasicBlock, ir: &mut String) -> Result<(), LlvmError> {
        let block_label = if block.is_entry {
            "entry".to_string()
        } else {
            format!("block{}", block.id.0)
        };
        
        ir.push_str(&format!("{}:\n", block_label));
        
        for instruction in &block.instructions {
            self.generate_instruction(instruction, ir)?;
        }
        
        Ok(())
    }
    
    fn generate_instruction(&self, instruction: &MirInstruction, ir: &mut String) -> Result<(), LlvmError> {
        ir.push_str("    ");
        
        match &instruction.kind {
            MirInstructionKind::Const { result, value } => {
                let value_str = self.mir_value_to_llvm(value);
                let llvm_type = self.mir_value_type(value);
                ir.push_str(&format!("%{} = {} {}\n", result.0, llvm_type.to_string(), value_str));
            }
            
            MirInstructionKind::Add { result, left, right } => {
                ir.push_str(&format!("%{} = add i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Subtract { result, left, right } => {
                ir.push_str(&format!("%{} = sub i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Multiply { result, left, right } => {
                ir.push_str(&format!("%{} = mul i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Divide { result, left, right } => {
                ir.push_str(&format!("%{} = sdiv i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Modulo { result, left, right } => {
                ir.push_str(&format!("%{} = srem i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Equal { result, left, right } => {
                ir.push_str(&format!("%{} = icmp eq i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::NotEqual { result, left, right } => {
                ir.push_str(&format!("%{} = icmp ne i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::LessThan { result, left, right } => {
                ir.push_str(&format!("%{} = icmp slt i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::LessEqual { result, left, right } => {
                ir.push_str(&format!("%{} = icmp sle i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::GreaterThan { result, left, right } => {
                ir.push_str(&format!("%{} = icmp sgt i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::GreaterEqual { result, left, right } => {
                ir.push_str(&format!("%{} = icmp sge i64 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::And { result, left, right } => {
                ir.push_str(&format!("%{} = and i1 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Or { result, left, right } => {
                ir.push_str(&format!("%{} = or i1 %{}, %{}\n", result.0, left.0, right.0));
            }
            
            MirInstructionKind::Not { result, operand } => {
                ir.push_str(&format!("%{} = xor i1 %{}, true\n", result.0, operand.0));
            }
            
            MirInstructionKind::Load { result, name } => {
                ir.push_str(&format!("%{} = load i64, i64* @{}\n", result.0, name));
            }
            
            MirInstructionKind::Store { name, value } => {
                ir.push_str(&format!("store i64 %{}, i64* @{}\n", value.0, name));
            }
            
            MirInstructionKind::Branch { condition, true_block, false_block } => {
                let true_label = if true_block.0 == 0 { "entry" } else { &format!("block{}", true_block.0) };
                let false_label = if false_block.0 == 0 { "entry" } else { &format!("block{}", false_block.0) };
                ir.push_str(&format!("br i1 %{}, label %{}, label %{}\n", condition.0, true_label, false_label));
            }
            
            MirInstructionKind::Jump { target } => {
                let label = if target.0 == 0 { "entry" } else { &format!("block{}", target.0) };
                ir.push_str(&format!("br label %{}\n", label));
            }
            
            MirInstructionKind::Call { result, function, arguments } => {
                let args: Vec<String> = arguments.iter().map(|a| format!("i8* %{}", a.0)).collect();
                if let Some(r) = result {
                    ir.push_str(&format!("%{} = call i8* @{}({})\n", r.0, function, args.join(", ")));
                } else {
                    ir.push_str(&format!("call void @{}({})\n", function, args.join(", ")));
                }
            }
            
            MirInstructionKind::Return { value } => {
                if let Some(v) = value {
                    ir.push_str(&format!("ret i64 %{}\n", v.0));
                } else {
                    ir.push_str("ret void\n");
                }
            }
            
            MirInstructionKind::TableNew { result } => {
                ir.push_str(&format!("%{} = call i8* @glua_table_new()\n", result.0));
            }
            
            MirInstructionKind::TableSet { table, key, value } => {
                ir.push_str(&format!("call void @glua_table_set(i8* %{}, i8* %{}, i8* %{})\n", table.0, key.0, value.0));
            }
            
            MirInstructionKind::TableGet { result, table, key } => {
                ir.push_str(&format!("%{} = call i8* @glua_table_get(i8* %{}, i8* %{})\n", result.0, table.0, key.0));
            }
            
            MirInstructionKind::Error => {
                ir.push_str("; <error instruction>\n");
            }
        }
        
        Ok(())
    }
    
    fn mir_value_to_llvm(&self, value: &MirValue) -> String {
        match value {
            MirValue::Integer(n) => n.to_string(),
            MirValue::Float(f) => f.to_string(),
            MirValue::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
            MirValue::String(s) => format!("c\"{}\\00\"", s),
            MirValue::Nil => "null".to_string(),
            MirValue::Unit => "()".to_string(),
        }
    }
    
    fn mir_value_type(&self, value: &MirValue) -> LlvmType {
        match value {
            MirValue::Integer(_) => LlvmType::Integer(64),
            MirValue::Float(_) => LlvmType::Double,
            MirValue::Boolean(_) => LlvmType::Boolean,
            MirValue::String(_) => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
            MirValue::Nil => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
            MirValue::Unit => LlvmType::Void,
        }
    }
}