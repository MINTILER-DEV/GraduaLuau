use super::block::MirBasicBlock;
use super::function::MirFunction;
use super::instruction::{MirInstruction, MirInstructionKind};
use super::module::MirModule;
use super::types::{MirCompareOperator, MirValue};

pub struct MirPrinter {
    indent: usize,
}

impl MirPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    pub fn print_module(&mut self, module: &MirModule) -> String {
        let mut output = String::new();

        output.push_str(&format!("Module '{}'\n", module.name));
        self.indent += 2;

        for function in &module.functions {
            output.push_str(&self.print_function(function));
        }

        self.indent -= 2;
        output
    }

    fn print_function(&mut self, function: &MirFunction) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        output.push_str(&format!("Function '{}'\n", function.name));

        self.indent += 2;
        for block in &function.blocks {
            output.push_str(&self.print_block(block));
        }
        if function.cfg.is_some() {
            output.push_str(&self.print_cfg(function));
        }
        self.indent -= 2;

        output
    }

    fn print_block(&mut self, block: &MirBasicBlock) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());

        let block_type = if block.is_entry {
            "Entry "
        } else if block.is_exit {
            "Exit "
        } else {
            ""
        };

        output.push_str(&format!(
            "{}Block{} '{}':\n",
            block_type, block.id.0, block.name
        ));

        self.indent += 2;
        if !block.predecessors.is_empty() {
            let predecessors: Vec<String> = block
                .predecessors
                .iter()
                .map(|block_id| format!("Block{}", block_id.0))
                .collect();
            output.push_str(&self.indent_str());
            output.push_str(&format!("predecessors: {}\n", predecessors.join(", ")));
        }
        if !block.successors.is_empty() {
            let successors: Vec<String> = block
                .successors
                .iter()
                .map(|block_id| format!("Block{}", block_id.0))
                .collect();
            output.push_str(&self.indent_str());
            output.push_str(&format!("successors: {}\n", successors.join(", ")));
        }
        for instruction in &block.instructions {
            output.push_str(&self.print_instruction(instruction));
        }
        self.indent -= 2;

        output
    }

    pub fn print_cfg(&mut self, function: &MirFunction) -> String {
        let mut output = String::new();
        let Some(cfg) = function.cfg.as_ref() else {
            return output;
        };

        output.push_str(&self.indent_str());
        output.push_str("CFG:\n");
        self.indent += 2;
        output.push_str(&self.indent_str());
        output.push_str(&format!("entry: Block{}\n", cfg.entry.0));

        if !cfg.exits.is_empty() {
            let exits: Vec<String> = cfg
                .exits
                .iter()
                .map(|block_id| format!("Block{}", block_id.0))
                .collect();
            output.push_str(&self.indent_str());
            output.push_str(&format!("exits: {}\n", exits.join(", ")));
        }

        for edge in &cfg.edges {
            output.push_str(&self.indent_str());
            output.push_str(&format!(
                "edge Block{} -> Block{} ({})\n",
                edge.source.0,
                edge.target.0,
                edge.kind.label()
            ));
        }

        for loop_info in &cfg.loops {
            let blocks: Vec<String> = loop_info
                .body_blocks
                .iter()
                .map(|block_id| format!("Block{}", block_id.0))
                .collect();
            output.push_str(&self.indent_str());
            output.push_str(&format!(
                "loop header Block{} body [{}]\n",
                loop_info.header.0,
                blocks.join(", ")
            ));
        }

        self.indent -= 2;
        output
    }

    fn print_instruction(&mut self, instruction: &MirInstruction) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());

        match &instruction.kind {
            MirInstructionKind::Const { result, value } => {
                output.push_str(&format!(
                    "const %{} = {}\n",
                    result.0,
                    self.print_value(value)
                ));
            }

            MirInstructionKind::Add {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("add %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Subtract {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("sub %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Multiply {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("mul %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Divide {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("div %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Modulo {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("mod %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Equal {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("eq %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::NotEqual {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("ne %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::LessThan {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("lt %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::LessEqual {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("le %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::GreaterThan {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("gt %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::GreaterEqual {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("ge %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::And {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("and %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Or {
                result,
                left,
                right,
            } => {
                output.push_str(&format!("or %{}, %{}, %{}\n", result.0, left.0, right.0));
            }

            MirInstructionKind::Not { result, operand } => {
                output.push_str(&format!("not %{}, %{}\n", result.0, operand.0));
            }

            MirInstructionKind::Load { result, name } => {
                output.push_str(&format!("load %{}, {}\n", result.0, name));
            }

            MirInstructionKind::Store { name, value } => {
                output.push_str(&format!("store {}, %{}\n", name, value.0));
            }

            MirInstructionKind::Move { result, value } => {
                output.push_str(&format!("move %{}, %{}\n", result.0, value.0));
            }

            MirInstructionKind::AllocateLocal { local, name } => {
                output.push_str(&format!("alloc_local %{}, {}\n", local.0, name));
            }

            MirInstructionKind::Branch {
                condition,
                true_block,
                false_block,
            } => {
                output.push_str(&format!(
                    "branch %{}, Block{}, Block{}\n",
                    condition.0, true_block.0, false_block.0
                ));
            }

            MirInstructionKind::Jump { target } => {
                output.push_str(&format!("jump Block{}\n", target.0));
            }

            MirInstructionKind::Unreachable => {
                output.push_str("unreachable\n");
            }

            MirInstructionKind::Call {
                result,
                function,
                arguments,
            } => {
                let args: Vec<String> = arguments.iter().map(|a| format!("%{}", a.0)).collect();
                let result_str = if let Some(r) = result {
                    format!("%{} = ", r.0)
                } else {
                    String::new()
                };
                output.push_str(&format!(
                    "call {}{}({})\n",
                    result_str,
                    function,
                    args.join(", ")
                ));
            }

            MirInstructionKind::Compare {
                result,
                operator,
                left,
                right,
            } => {
                output.push_str(&format!(
                    "cmp {} %{}, %{}, %{}\n",
                    self.print_compare_operator(operator),
                    result.0,
                    left.0,
                    right.0
                ));
            }

            MirInstructionKind::Return { value } => {
                if let Some(v) = value {
                    output.push_str(&format!("return %{}\n", v.0));
                } else {
                    output.push_str("return\n");
                }
            }

            MirInstructionKind::TableNew { result } => {
                output.push_str(&format!("table_new %{}\n", result.0));
            }

            MirInstructionKind::TableSet { table, key, value } => {
                output.push_str(&format!(
                    "table_set %{}, %{}, %{}\n",
                    table.0, key.0, value.0
                ));
            }

            MirInstructionKind::TableGet { result, table, key } => {
                output.push_str(&format!(
                    "table_get %{}, %{}, %{}\n",
                    result.0, table.0, key.0
                ));
            }

            MirInstructionKind::Error => {
                output.push_str("<error>\n");
            }
        }

        output
    }

    fn print_value(&self, value: &MirValue) -> String {
        match value {
            MirValue::Integer(n) => n.to_string(),
            MirValue::Float(f) => f.to_string(),
            MirValue::Boolean(b) => b.to_string(),
            MirValue::String(s) => format!("\"{}\"", s),
            MirValue::Nil => "nil".to_string(),
            MirValue::Unit => "()".to_string(),
        }
    }

    fn print_compare_operator(&self, operator: &MirCompareOperator) -> &'static str {
        match operator {
            MirCompareOperator::Equal => "eq",
            MirCompareOperator::NotEqual => "ne",
            MirCompareOperator::LessThan => "lt",
            MirCompareOperator::LessEqual => "le",
            MirCompareOperator::GreaterThan => "gt",
            MirCompareOperator::GreaterEqual => "ge",
        }
    }

    fn indent_str(&self) -> String {
        " ".repeat(self.indent)
    }
}

impl Default for MirPrinter {
    fn default() -> Self {
        Self::new()
    }
}
