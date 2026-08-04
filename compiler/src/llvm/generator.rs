use crate::llvm::error::LlvmError;
use crate::llvm::types::{LlvmType, map_mir_type};
use crate::mir::MirModule;
use crate::mir::block::MirBasicBlock;
use crate::mir::function::MirFunction;
use crate::mir::instruction::{MirInstruction, MirInstructionKind};
use crate::mir::types::{MirCompareOperator, MirType, MirValue, MirValueId};
use std::collections::HashMap;

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
                string_constant.name, length, escaped
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
        ir.push_str("declare void @glua_print_i64(i64)\n");
        ir.push_str("declare void @glua_print_f64(double)\n");
        ir.push_str("declare void @glua_print_bool(i1)\n");
        ir.push_str("declare i8* @glua_table_new()\n");
        ir.push_str("declare void @glua_table_set(i8*, i8*, i8*)\n");
        ir.push_str("declare i8* @glua_table_get(i8*, i8*)\n");
        ir.push_str("\n");
    }

    fn generate_function(
        &mut self,
        function: &MirFunction,
        ir: &mut String,
    ) -> Result<(), LlvmError> {
        let return_type = self.function_return_type(function);
        let local_slots = self.collect_local_slots(function);
        let value_types = self.collect_value_types(function);

        // Function signature
        ir.push_str(&format!(
            "define {} @{}(",
            return_type.to_string(),
            function.name
        ));

        // Parameters
        let param_types: Vec<String> = function
            .parameter_data
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                format!(
                    "{} %arg{}",
                    map_mir_type(&parameter.value_type).to_string(),
                    index
                )
            })
            .collect();
        ir.push_str(&param_types.join(", "));

        ir.push_str(") {\n");

        // Generate basic blocks
        for block in &function.blocks {
            self.generate_block(block, function, &local_slots, &value_types, ir)?;
        }

        ir.push_str("}\n\n");

        Ok(())
    }

    fn generate_block(
        &mut self,
        block: &MirBasicBlock,
        function: &MirFunction,
        local_slots: &HashMap<String, LlvmType>,
        value_types: &HashMap<MirValueId, LlvmType>,
        ir: &mut String,
    ) -> Result<(), LlvmError> {
        let block_label = if block.is_entry {
            "entry".to_string()
        } else {
            format!("block{}", block.id.0)
        };

        ir.push_str(&format!("{}:\n", block_label));

        if block.is_entry {
            let mut ordered_slots: Vec<_> = local_slots.iter().collect();
            ordered_slots.sort_by(|left, right| left.0.cmp(right.0));
            for (slot, slot_type) in ordered_slots {
                ir.push_str(&format!(
                    "    %{} = alloca {}, align 8\n",
                    slot,
                    slot_type.to_string()
                ));
            }

            for (index, parameter) in function.parameter_data.iter().enumerate() {
                if let Some(slot_type) = local_slots.get(&parameter.storage) {
                    ir.push_str(&format!(
                        "    store {} %arg{}, {}* %{}\n",
                        slot_type.to_string(),
                        index,
                        slot_type.to_string(),
                        parameter.storage
                    ));
                }
            }
        }

        for instruction in &block.instructions {
            self.generate_instruction(instruction, local_slots, value_types, ir)?;
        }

        Ok(())
    }

    fn generate_instruction(
        &mut self,
        instruction: &MirInstruction,
        local_slots: &HashMap<String, LlvmType>,
        value_types: &HashMap<MirValueId, LlvmType>,
        ir: &mut String,
    ) -> Result<(), LlvmError> {
        ir.push_str("    ");

        match &instruction.kind {
            MirInstructionKind::Const { result, value } => match value {
                MirValue::String(string_value) => {
                    let global_name = self.intern_string_constant(string_value.clone());
                    let length = string_value.as_bytes().len() + 1;
                    ir.push_str(&format!(
                        "%{} = getelementptr inbounds [{} x i8], [{} x i8]* @{}, i64 0, i64 0\n",
                        result.0, length, length, global_name
                    ));
                }
                MirValue::Integer(integer_value) => {
                    ir.push_str(&format!("%{} = add i64 0, {}\n", result.0, integer_value));
                }
                MirValue::Float(float_value) => {
                    ir.push_str(&format!(
                        "%{} = fadd double 0.0, {}\n",
                        result.0,
                        Self::format_float_literal(*float_value)
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
            },

            MirInstructionKind::Add {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = add i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Subtract {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = sub i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Multiply {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = mul i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Divide {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = sdiv i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Modulo {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = srem i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Equal {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp eq i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::NotEqual {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp ne i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::LessThan {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp slt i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::LessEqual {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp sle i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::GreaterThan {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp sgt i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::GreaterEqual {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp sge i64 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::And {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = and i1 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Or {
                result,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = or i1 %{}, %{}\n",
                    result.0, left.0, right.0
                ));
            }

            MirInstructionKind::Not { result, operand } => {
                ir.push_str(&format!("%{} = xor i1 %{}, true\n", result.0, operand.0));
            }

            MirInstructionKind::Load { result, name } => {
                let value_type = local_slots
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.llvm_value_type(value_types, *result));
                ir.push_str(&format!(
                    "%{} = load {}, {}* {}\n",
                    result.0,
                    value_type.to_string(),
                    value_type.to_string(),
                    self.format_pointer_name(name)
                ));
            }

            MirInstructionKind::Store { name, value } => {
                let value_type = local_slots
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| self.llvm_value_type(value_types, *value));
                ir.push_str(&format!(
                    "store {} %{}, {}* {}\n",
                    value_type.to_string(),
                    value.0,
                    value_type.to_string(),
                    self.format_pointer_name(name)
                ));
            }

            MirInstructionKind::Move { result, value } => {
                ir.push_str(&format!("%{} = add i64 0, %{}\n", result.0, value.0));
            }

            MirInstructionKind::AllocateLocal { .. } => {
                ir.push_str("; local slot allocated in entry prologue\n");
            }

            MirInstructionKind::Branch {
                condition,
                true_block,
                false_block,
            } => {
                let true_label = if true_block.0 == 0 {
                    "entry"
                } else {
                    &format!("block{}", true_block.0)
                };
                let false_label = if false_block.0 == 0 {
                    "entry"
                } else {
                    &format!("block{}", false_block.0)
                };
                ir.push_str(&format!(
                    "br i1 %{}, label %{}, label %{}\n",
                    condition.0, true_label, false_label
                ));
            }

            MirInstructionKind::Jump { target } => {
                let label = if target.0 == 0 {
                    "entry"
                } else {
                    &format!("block{}", target.0)
                };
                ir.push_str(&format!("br label %{}\n", label));
            }

            MirInstructionKind::Unreachable => {
                ir.push_str("unreachable\n");
            }

            MirInstructionKind::Call {
                result,
                function,
                arguments,
            } => {
                if function == "glua_print" && arguments.len() == 1 {
                    let argument = arguments[0];
                    let argument_type = self.llvm_value_type(value_types, argument);
                    let print_function = self.print_function_for_type(&argument_type);
                    ir.push_str(&format!(
                        "call void @{}({} %{})\n",
                        print_function,
                        argument_type.to_string(),
                        argument.0
                    ));
                    return Ok(());
                }

                let args: Vec<String> = arguments
                    .iter()
                    .map(|argument| {
                        let argument_type = self.llvm_value_type(value_types, *argument);
                        format!("{} %{}", argument_type.to_string(), argument.0)
                    })
                    .collect();
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

            MirInstructionKind::Compare {
                result,
                operator,
                left,
                right,
            } => {
                ir.push_str(&format!(
                    "%{} = icmp {} i64 %{}, %{}\n",
                    result.0,
                    self.compare_predicate(operator),
                    left.0,
                    right.0
                ));
            }

            MirInstructionKind::Return { value } => {
                if let Some(v) = value {
                    let value_type = self.llvm_value_type(value_types, *v);
                    ir.push_str(&format!("ret {} %{}\n", value_type.to_string(), v.0));
                } else {
                    ir.push_str("ret void\n");
                }
            }

            MirInstructionKind::TableNew { result } => {
                ir.push_str(&format!("%{} = call i8* @glua_table_new()\n", result.0));
            }

            MirInstructionKind::TableSet { table, key, value } => {
                ir.push_str(&format!(
                    "call void @glua_table_set(i8* %{}, i8* %{}, i8* %{})\n",
                    table.0, key.0, value.0
                ));
            }

            MirInstructionKind::TableGet { result, table, key } => {
                ir.push_str(&format!(
                    "%{} = call i8* @glua_table_get(i8* %{}, i8* %{})\n",
                    result.0, table.0, key.0
                ));
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
        self.string_constant_names
            .insert(value.clone(), name.clone());
        self.string_constants.push(StringConstant {
            name: name.clone(),
            value,
        });
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
            "glua_table_new" | "glua_table_get" => {
                LlvmType::Pointer(Box::new(LlvmType::Integer(8)))
            }
            other => self
                .function_return_types
                .get(other)
                .cloned()
                .unwrap_or(LlvmType::Void),
        }
    }

    fn collect_local_slots(&self, function: &MirFunction) -> HashMap<String, LlvmType> {
        let mut slots = HashMap::new();

        for local in &function.locals {
            if self.is_local_slot(&local.storage) {
                slots.insert(local.storage.clone(), map_mir_type(&local.value_type));
            }
        }

        for block in &function.blocks {
            for instruction in &block.instructions {
                match &instruction.kind {
                    MirInstructionKind::Load { name, .. }
                    | MirInstructionKind::Store { name, .. }
                    | MirInstructionKind::AllocateLocal { name, .. } => {
                        if self.is_local_slot(name) {
                            slots
                                .entry(name.clone())
                                .or_insert_with(|| LlvmType::Integer(64));
                        }
                    }
                    _ => {}
                }
            }
        }

        slots
    }

    fn collect_value_types(&self, function: &MirFunction) -> HashMap<MirValueId, LlvmType> {
        function
            .values
            .iter()
            .map(|value| (value.id, map_mir_type(&value.value_type)))
            .collect()
    }

    fn llvm_value_type(
        &self,
        value_types: &HashMap<MirValueId, LlvmType>,
        value_id: MirValueId,
    ) -> LlvmType {
        value_types
            .get(&value_id)
            .cloned()
            .unwrap_or(LlvmType::Integer(64))
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

    fn print_function_for_type(&self, value_type: &LlvmType) -> &'static str {
        match value_type {
            LlvmType::Integer(64) => "glua_print_i64",
            LlvmType::Double => "glua_print_f64",
            LlvmType::Boolean => "glua_print_bool",
            _ => "glua_print",
        }
    }

    fn format_float_literal(value: f64) -> String {
        let mut text = value.to_string();
        if !text.contains('.') && !text.contains('e') && !text.contains('E') {
            text.push_str(".0");
        }
        text
    }

    fn compare_predicate(&self, operator: &MirCompareOperator) -> &'static str {
        match operator {
            MirCompareOperator::Equal => "eq",
            MirCompareOperator::NotEqual => "ne",
            MirCompareOperator::LessThan => "slt",
            MirCompareOperator::LessEqual => "sle",
            MirCompareOperator::GreaterThan => "sgt",
            MirCompareOperator::GreaterEqual => "sge",
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
