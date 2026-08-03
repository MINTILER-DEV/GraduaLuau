use super::module::HirModule;
use super::module::HirGlobalVariable;
use super::statement::{HirStatement, HirStatementKind};
use super::function::HirFunction;
use super::expression::{HirExpression, HirExpressionKind, HirTableField};
use super::types::{HirUnaryOperator, HirBinaryOperator};

pub struct HirPrinter {
    indent: usize,
}

impl HirPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }
    
    pub fn print_module(&mut self, module: &HirModule) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Module '{}'\n", module.name));
        self.indent += 2;
        
        // Print global variables
        if !module.global_variables.is_empty() {
            output.push_str(&self.indent_str());
            output.push_str("Global Variables:\n");
            self.indent += 2;
            for global in &module.global_variables {
                output.push_str(&self.print_global_variable(global));
            }
            self.indent -= 2;
        }
        
        // Print functions
        if !module.functions.is_empty() {
            output.push_str(&self.indent_str());
            output.push_str("Functions:\n");
            self.indent += 2;
            for function in &module.functions {
                output.push_str(&self.print_function(function));
            }
            self.indent -= 2;
        }
        
        self.indent -= 2;
        output
    }
    
    fn print_global_variable(&mut self, global: &HirGlobalVariable) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        output.push_str(&format!("Global Variable: {}\n", global.name));
        
        if let Some(initializer) = &global.initializer {
            self.indent += 2;
            output.push_str(&self.indent_str());
            output.push_str("Initializer: ");
            output.push_str(&self.print_expression(initializer));
            output.push('\n');
            self.indent -= 2;
        }
        
        output
    }
    
    fn print_function(&mut self, function: &HirFunction) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        output.push_str(&format!("Function '{}'", function.name));
        
        if !function.parameters.is_empty() {
            output.push_str("(");
            let params: Vec<String> = function.parameters.iter()
                .map(|p| {
                    if let Some(param_type) = &p.param_type {
                        format!("{}: {:?}", p.name, param_type)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
            output.push(')');
        }
        
        if let Some(return_type) = &function.return_type {
            output.push_str(&format!(" -> {:?}", return_type));
        }
        
        output.push('\n');
        
        self.indent += 2;
        for statement in &function.body {
            output.push_str(&self.print_statement(statement));
        }
        self.indent -= 2;
        
        output
    }
    
    fn print_statement(&mut self, statement: &HirStatement) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        
        match &statement.kind {
            HirStatementKind::LocalVariable { variable, initializer } => {
                output.push_str(&format!("Local Variable: {}", variable.name));
                if let Some(init) = initializer {
                    output.push_str(" = ");
                    output.push_str(&self.print_expression(init));
                }
                output.push('\n');
            }
            HirStatementKind::Assignment { targets, values } => {
                output.push_str("Assignment: ");
                let target_strs: Vec<String> = targets.iter()
                    .map(|t| self.print_expression(t))
                    .collect();
                output.push_str(&target_strs.join(", "));
                output.push_str(" = ");
                let value_strs: Vec<String> = values.iter()
                    .map(|v| self.print_expression(v))
                    .collect();
                output.push_str(&value_strs.join(", "));
                output.push('\n');
            }
            HirStatementKind::Expression(expr) => {
                output.push_str(&self.print_expression(expr));
                output.push('\n');
            }
            HirStatementKind::Return(exprs) => {
                output.push_str("Return");
                if let Some(exprs) = exprs {
                    output.push_str(" ");
                    let expr_strs: Vec<String> = exprs.iter()
                        .map(|e| self.print_expression(e))
                        .collect();
                    output.push_str(&expr_strs.join(", "));
                }
                output.push('\n');
            }
            HirStatementKind::Block(statements) => {
                output.push_str("Block\n");
                self.indent += 2;
                for stmt in statements {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::If { condition, then_block, else_block } => {
                output.push_str("If ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
                self.indent += 2;
                output.push_str(&self.indent_str());
                output.push_str("Then:\n");
                self.indent += 2;
                for stmt in then_block {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
                
                if let Some(else_block) = else_block {
                    output.push_str(&self.indent_str());
                    output.push_str("Else:\n");
                    self.indent += 2;
                    for stmt in else_block {
                        output.push_str(&self.print_statement(stmt));
                    }
                    self.indent -= 2;
                }
                self.indent -= 2;
            }
            HirStatementKind::While { condition, body } => {
                output.push_str("While ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::RepeatUntil { body, condition } => {
                output.push_str("Repeat\n");
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
                output.push_str(&self.indent_str());
                output.push_str("Until ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
            }
            HirStatementKind::ForNumeric { variable, start, end, step, body } => {
                output.push_str(&format!("For {} = ", variable.name));
                output.push_str(&self.print_expression(start));
                output.push_str(" to ");
                output.push_str(&self.print_expression(end));
                if let Some(step) = step {
                    output.push_str(" step ");
                    output.push_str(&self.print_expression(step));
                }
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::ForGeneric { variables, iterables, body } => {
                output.push_str("For ");
                let var_names: Vec<String> = variables.iter().map(|v| v.name.clone()).collect();
                output.push_str(&var_names.join(", "));
                output.push_str(" in ");
                let iterable_strs: Vec<String> = iterables.iter()
                    .map(|i| self.print_expression(i))
                    .collect();
                output.push_str(&iterable_strs.join(", "));
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::Break => {
                output.push_str("Break\n");
            }
            HirStatementKind::Continue => {
                output.push_str("Continue\n");
            }
            HirStatementKind::Function { function } => {
                output.push_str(&self.print_function(function));
            }
            HirStatementKind::Error => {
                output.push_str("<Error>\n");
            }
        }
        
        output
    }
    
    fn print_expression(&mut self, expr: &HirExpression) -> String {
        match &expr.kind {
            HirExpressionKind::Nil => "nil".to_string(),
            HirExpressionKind::Boolean(b) => b.to_string(),
            HirExpressionKind::Number(n) => n.to_string(),
            HirExpressionKind::String(s) => format!("\"{}\"", s),
            HirExpressionKind::LocalVariable(id) => format!("local_{}", id.0),
            HirExpressionKind::GlobalVariable(name) => name.clone(),
            HirExpressionKind::Unary { operator, operand } => {
                let op_str = self.print_unary_operator(*operator);
                format!("{}({})", op_str, self.print_expression(operand))
            }
            HirExpressionKind::Binary { left, operator, right } => {
                let op_str = self.print_binary_operator(*operator);
                format!("({} {} {})", self.print_expression(left), op_str, self.print_expression(right))
            }
            HirExpressionKind::TableConstructor(fields) => {
                let mut field_strs = Vec::new();
                for field in fields {
                    match field {
                        HirTableField::Named { key, value } => {
                            field_strs.push(format!("{} = {}", key, self.print_expression(value)));
                        }
                        HirTableField::Indexed { key, value } => {
                            field_strs.push(format!("[{}] = {}", self.print_expression(key), self.print_expression(value)));
                        }
                        HirTableField::Expression(expr) => {
                            field_strs.push(self.print_expression(expr));
                        }
                    }
                }
                format!("{{{}}}", field_strs.join(", "))
            }
            HirExpressionKind::Index { object, index } => {
                format!("{}[{}]", self.print_expression(object), self.print_expression(index))
            }
            HirExpressionKind::FieldAccess { object, field } => {
                format!("{}.{}", self.print_expression(object), field)
            }
            HirExpressionKind::FunctionCall { callee, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("{}({})", self.print_expression(callee), arg_strs.join(", "))
            }
            HirExpressionKind::MethodCall { receiver, method, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("{}:{}({})", self.print_expression(receiver), method, arg_strs.join(", "))
            }
            HirExpressionKind::ClosurePlaceholder => "<closure>".to_string(),
            HirExpressionKind::BuiltinCall { function, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("<builtin {:?}>({})", function, arg_strs.join(", "))
            }
            HirExpressionKind::Error => "<error>".to_string(),
        }
    }
    
    fn print_unary_operator(&self, op: HirUnaryOperator) -> &'static str {
        match op {
            HirUnaryOperator::Negate => "-",
            HirUnaryOperator::Not => "not",
            HirUnaryOperator::Length => "#",
            HirUnaryOperator::BitwiseNot => "~",
        }
    }
    
    fn print_binary_operator(&self, op: HirBinaryOperator) -> &'static str {
        match op {
            HirBinaryOperator::Add => "+",
            HirBinaryOperator::Subtract => "-",
            HirBinaryOperator::Multiply => "*",
            HirBinaryOperator::Divide => "/",
            HirBinaryOperator::FloorDivide => "//",
            HirBinaryOperator::Modulo => "%",
            HirBinaryOperator::Exponent => "^",
            HirBinaryOperator::Equal => "==",
            HirBinaryOperator::NotEqual => "~=",
            HirBinaryOperator::LessThan => "<",
            HirBinaryOperator::LessEqual => "<=",
            HirBinaryOperator::GreaterThan => ">",
            HirBinaryOperator::GreaterEqual => ">=",
            HirBinaryOperator::And => "and",
            HirBinaryOperator::Or => "or",
            HirBinaryOperator::Concatenate => "..",
            HirBinaryOperator::BitwiseAnd => "&",
            HirBinaryOperator::BitwiseOr => "|",
            HirBinaryOperator::BitwiseXor => "~",
            HirBinaryOperator::BitwiseShiftLeft => "<<",
            HirBinaryOperator::BitwiseShiftRight => ">>",
        }
    }
    
    fn indent_str(&self) -> String {
        " ".repeat(self.indent)
    }
}

impl Default for HirPrinter {
    fn default() -> Self {
        Self::new()
    }
}