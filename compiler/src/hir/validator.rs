use crate::source::SourceSpan;
use super::module::HirModule;
use super::module::HirGlobalVariable;
use super::statement::{HirStatement, HirStatementKind};
use super::function::HirFunction;
use super::expression::{HirExpression, HirExpressionKind, HirTableField};

#[derive(Debug, Clone)]
pub struct HirValidator {
    errors: Vec<HirValidationError>,
}

#[derive(Debug, Clone)]
pub enum HirValidationError {
    InvalidControlFlow {
        message: String,
        span: SourceSpan,
    },
    InvalidExpression {
        message: String,
        span: SourceSpan,
    },
}

impl HirValidator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }
    
    pub fn validate(&mut self, module: &HirModule) -> Result<(), Vec<HirValidationError>> {
        self.validate_module(module);
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }
    
    fn validate_module(&mut self, module: &HirModule) {
        // Empty modules are valid (might contain only comments or be empty files)
        // Skip empty module check for now
        
        // Validate all functions
        for function in &module.functions {
            self.validate_function(function);
        }
        
        // Validate global variables
        for global in &module.global_variables {
            self.validate_global_variable(global);
        }
    }
    
    fn validate_function(&mut self, function: &HirFunction) {
        // Empty functions are valid in Lua (stubs, forward declarations)
        // Skip empty function body check for now
        
        // Validate function body statements
        for statement in &function.body {
            self.validate_statement(statement);
        }
    }
    
    fn validate_global_variable(&mut self, global: &HirGlobalVariable) {
        // Validate initializer if present
        if let Some(initializer) = &global.initializer {
            self.validate_expression(initializer);
        }
    }
    
    fn validate_statement(&mut self, statement: &HirStatement) {
        match &statement.kind {
            HirStatementKind::LocalVariable { initializer, variable: _ } => {
                if let Some(init) = initializer {
                    self.validate_expression(init);
                }
            }
            HirStatementKind::Assignment { targets, values } => {
                for target in targets {
                    self.validate_expression(target);
                }
                for value in values {
                    self.validate_expression(value);
                }
            }
            HirStatementKind::Expression(expr) => {
                self.validate_expression(expr);
            }
            HirStatementKind::Return(exprs) => {
                if let Some(exprs) = exprs {
                    for expr in exprs {
                        self.validate_expression(expr);
                    }
                }
            }
            HirStatementKind::If { condition, then_block, else_block } => {
                self.validate_expression(condition);
                for stmt in then_block {
                    self.validate_statement(stmt);
                }
                if let Some(else_block) = else_block {
                    for stmt in else_block {
                        self.validate_statement(stmt);
                    }
                }
            }
            HirStatementKind::While { condition, body } => {
                self.validate_expression(condition);
                for stmt in body {
                    self.validate_statement(stmt);
                }
            }
            HirStatementKind::RepeatUntil { body, condition } => {
                for stmt in body {
                    self.validate_statement(stmt);
                }
                self.validate_expression(condition);
            }
            HirStatementKind::ForNumeric { start, end, step, body, variable: _ } => {
                self.validate_expression(start);
                self.validate_expression(end);
                if let Some(step) = step {
                    self.validate_expression(step);
                }
                for stmt in body {
                    self.validate_statement(stmt);
                }
            }
            HirStatementKind::ForGeneric { iterables, body, variables: _ } => {
                for iterable in iterables {
                    self.validate_expression(iterable);
                }
                for stmt in body {
                    self.validate_statement(stmt);
                }
            }
            HirStatementKind::Break | HirStatementKind::Continue => {
                // These are valid in their respective contexts
                // Context validation would require more complex analysis
            }
            HirStatementKind::Block(statements) => {
                for stmt in statements {
                    self.validate_statement(stmt);
                }
            }
            HirStatementKind::Function { function } => {
                self.validate_function(function);
            }
            HirStatementKind::Error => {
                // Error statements are already marked as invalid
            }
        }
    }
    
    fn validate_expression(&mut self, expression: &HirExpression) {
        match &expression.kind {
            HirExpressionKind::Unary { operand, operator: _ } => {
                self.validate_expression(operand);
            }
            HirExpressionKind::Binary { left, right, operator: _ } => {
                self.validate_expression(left);
                self.validate_expression(right);
            }
            HirExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        HirTableField::Named { value, key: _ } => {
                            self.validate_expression(value);
                        }
                        HirTableField::Indexed { key, value } => {
                            self.validate_expression(key);
                            self.validate_expression(value);
                        }
                        HirTableField::Expression(expr) => {
                            self.validate_expression(expr);
                        }
                    }
                }
            }
            HirExpressionKind::Index { object, index } => {
                self.validate_expression(object);
                self.validate_expression(index);
            }
            HirExpressionKind::FieldAccess { object, field: _ } => {
                self.validate_expression(object);
            }
            HirExpressionKind::FunctionCall { callee, arguments } => {
                self.validate_expression(callee);
                for arg in arguments {
                    self.validate_expression(arg);
                }
            }
            HirExpressionKind::MethodCall { receiver, arguments, method: _ } => {
                self.validate_expression(receiver);
                for arg in arguments {
                    self.validate_expression(arg);
                }
            }
            HirExpressionKind::ClosurePlaceholder => {
                // Closures are stored separately to avoid circular dependencies
                // Validation would need to be done on the actual function storage
            }
            HirExpressionKind::BuiltinCall { arguments, function: _ } => {
                for arg in arguments {
                    self.validate_expression(arg);
                }
            }
            // Literals and variables are always valid
            HirExpressionKind::Nil
            | HirExpressionKind::Boolean(_)
            | HirExpressionKind::Number(_)
            | HirExpressionKind::String(_)
            | HirExpressionKind::LocalVariable(_)
            | HirExpressionKind::GlobalVariable(_) => {}
            HirExpressionKind::Error => {
                self.errors.push(HirValidationError::InvalidExpression {
                    message: "Expression contains error".to_string(),
                    span: expression.span,
                });
            }
        }
    }
}

impl Default for HirValidator {
    fn default() -> Self {
        Self::new()
    }
}