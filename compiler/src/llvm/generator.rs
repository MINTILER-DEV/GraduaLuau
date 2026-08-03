use crate::mir::MirModule;
use crate::mir::function::MirFunction;
use crate::mir::block::MirBasicBlock;
use crate::mir::instruction::{MirInstruction, MirInstructionKind};
use crate::mir::types::{MirValue, MirType};
use crate::llvm::error::LlvmError;
use crate::llvm::types::{LlvmType, map_mir_type};
use std::collections::{BTreeSet, HashMap};

pub struct LlvmGenerator {
    module_name: String,
    string_constants: Vec<StringConstant>,
    string_constant_names: HashMap<String, String>,
    function_return_types: HashMap<String, LlvmType>,
}

struct StringConstant {
    name: String,
    value: String,
}

impl LlvmGenerator {
    pub fn new(module_name: String) -> Self {
        Self {
            module_name,
            string_constants: Vec::new(),
            string_constant_names: HashMap::new(),
            function_return_types: HashMap::new(),
        }
    }
    
    pub fn generate(&mut self, mir: &MirModule) -> Result<String, LlvmError> {
        let mut ir = String::new();
        let mut functions_ir = String::new();

        self.function_return_types = mir
            .functions
            .iter()
            .map(|function| {
                (
                    function.name.clone(),
                    map_mir_type(function.return_type.as_ref().unwrap_or(&MirType::Void)),
                )
            })
            .collect();
        
        // Module header
        ir.push_str(&format!("; Module ID: '{}'\n", self.module_name));
        ir.push_str("target triple = \"x86_64-unknown-linux-gnu\"\n");
        ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        ir.push_str("\n");
        
        // Generate function declarations
        self.generate_runtime_declarations(&mut ir);

        // Generate functions
        for function in &mir.functions {
            self.generate_function(function, &mut functions_ir)?;
        }

        // Emit string constants before function definitions
        for string_constant in &self.string_constants {
            let escaped = Self::escape_llvm_bytes(string_constant.value.as_bytes());
            let length = string_constant.value.as_bytes().len() + 1;
            ir.push_str(&format!(
                "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n",
                string_constant.name,
                length,
                escaped
            ));
        }

        if !self.string_constants.is_empty() {
            ir.push('\n');
        }

        // Generate functions
        ir.push_str(&functions_ir);
        
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
    
    fn generate_function(&mut self, function: &MirFunction, ir: &mut String) -> Result<(), LlvmError> {
        let return_type = self.function_return_type(function);
        let local_slots = self.collect_local_slots(function);
        
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
            self.generate_block(block, &local_slots, ir)?;
        }
        
        ir.push_str("}\n\n");
        
        Ok(())
    }
    
    fn generate_block(&mut self, block: &MirBasicBlock, local_slots: &BTreeSet<String>, ir: &mut String) -> Result<(), LlvmError> {
        let block_label = if block.is_entry {
            "entry".to_string()
        } else {
            format!("block{}", block.id.0)
        };
        
        ir.push_str(&format!("{}:\n", block_label));

        if block.is_entry {
            for slot in local_slots {
                ir.push_str(&format!("    %{} = alloca i64, align 8\n", slot));
            }
        }
        
        for instruction in &block.instructions {
            self.generate_instruction(instruction, ir)?;
        }
        
        Ok(())
    }
    
    fn generate_instruction(&mut self, instruction: &MirInstruction, ir: &mut String) -> Result<(), LlvmError> {
        ir.push_str("    ");
        
        match &instruction.kind {
            MirInstructionKind::Const { result, value } => {
                match value {
                    MirValue::String(string_value) => {
                        let global_name = self.intern_string_constant(string_value.clone());
                        let length = string_value.as_bytes().len() + 1;
                        ir.push_str(&format!(
                            "%{} = getelementptr inbounds [{} x i8], [{} x i8]* @{}, i64 0, i64 0\n",
                            result.0,
                            length,
                            length,
                            global_name
                        ));
                    }
                    MirValue::Integer(integer_value) => {
                        ir.push_str(&format!("%{} = add i64 0, {}\n", result.0, integer_value));
                    }
                    MirValue::Float(float_value) => {
                        ir.push_str(&format!(
                            "%{} = fadd double 0.0, {}\n",
                            result.0,
                            self.mir_value_to_llvm(value)
                        ));
                    }
                    MirValue::Boolean(boolean_value) => {
                        ir.push_str(&format!(
                            "%{} = xor i1 false, {}\n",
                            result.0,
                            if *boolean_value { "true" } else { "false" }
                        ));
                    }
                    MirValue::Nil => {
                        ir.push_str(&format!("%{} = inttoptr i64 0 to i8*\n", result.0));
                    }
                    MirValue::Unit => {
                        ir.push_str(&format!("%{} = add i64 0, 0\n", result.0));
                    }
                }
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
                ir.push_str(&format!(
                    "%{} = load i64, i64* {}\n",
                    result.0,
                    self.format_pointer_name(name)
                ));
            }
            
            MirInstructionKind::Store { name, value } => {
                ir.push_str(&format!(
                    "store i64 %{}, i64* {}\n",
                    value.0,
                    self.format_pointer_name(name)
                ));
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
                let call_return_type = self.call_return_type(function);
                if let Some(r) = result {
                    ir.push_str(&format!(
                        "%{} = call {} @{}({})\n",
                        r.0,
                        call_return_type.to_string(),
                        function,
                        args.join(", ")
                    ));
                } else {
                    ir.push_str(&format!(
                        "call {} @{}({})\n",
                        call_return_type.to_string(),
                        function,
                        args.join(", ")
                    ));
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

    fn intern_string_constant(&mut self, value: String) -> String {
        if let Some(name) = self.string_constant_names.get(&value) {
            return name.clone();
        }

        let name = format!(".str{}", self.string_constants.len());
        self.string_constant_names.insert(value.clone(), name.clone());
        self.string_constants.push(StringConstant { name: name.clone(), value });
        name
    }

    fn function_return_type(&self, function: &MirFunction) -> LlvmType {
        self.function_return_types
            .get(&function.name)
            .cloned()
            .unwrap_or(LlvmType::Void)
    }

    fn call_return_type(&self, function_name: &str) -> LlvmType {
        match function_name {
            "glua_print" | "glua_table_set" => LlvmType::Void,
            "glua_table_new" | "glua_table_get" => LlvmType::Pointer(Box::new(LlvmType::Integer(8))),
            other => self
                .function_return_types
                .get(other)
                .cloned()
                .unwrap_or(LlvmType::Void),
        }
    }

    fn collect_local_slots(&self, function: &MirFunction) -> BTreeSet<String> {
        let mut slots = BTreeSet::new();

        for block in &function.blocks {
            for instruction in &block.instructions {
                match &instruction.kind {
                    MirInstructionKind::Load { name, .. } | MirInstructionKind::Store { name, .. } => {
                        if self.is_local_slot(name) {
                            slots.insert(name.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        slots
    }

    fn is_local_slot(&self, name: &str) -> bool {
        name.starts_with("local_")
    }

    fn format_pointer_name(&self, name: &str) -> String {
        if self.is_local_slot(name) {
            format!("%{}", name)
        } else {
            format!("@{}", name)
        }
    }
    
    fn mir_value_to_llvm(&self, value: &MirValue) -> String {
        match value {
            MirValue::Integer(n) => n.to_string(),
            MirValue::Float(f) => {
                let mut text = f.to_string();
                if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                    text.push_str(".0");
                }
                text
            }
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

    fn escape_llvm_bytes(bytes: &[u8]) -> String {
        let mut output = String::new();

        for byte in bytes {
            match byte {
                b'\\' => output.push_str("\\5C"),
                b'"' => output.push_str("\\22"),
                b'\n' => output.push_str("\\0A"),
                b'\r' => output.push_str("\\0D"),
                b'\t' => output.push_str("\\09"),
                0x20..=0x7E => output.push(*byte as char),
                other => output.push_str(&format!("\\{:02X}", other)),
            }
        }

        output
    }
}
