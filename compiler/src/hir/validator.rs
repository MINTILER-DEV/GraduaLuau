use std::collections::HashSet;

use super::expression::{
    HirExpression, HirExpressionKind, HirInterpolatedStringPart, HirTableField,
};
use super::function::HirFunction;
use super::ids::{HirScopeId, HirSymbolId};
use super::module::{HirGlobalVariable, HirModule, HirTypeAlias};
use super::statement::{HirStatement, HirStatementKind};
use super::types::HirType;
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct HirValidator {
    errors: Vec<HirValidationError>,
    known_symbols: HashSet<HirSymbolId>,
    known_scopes: HashSet<HirScopeId>,
    current_return_type: Option<HirType>,
}

#[derive(Debug, Clone)]
pub enum HirValidationError {
    InvalidControlFlow { message: String, span: SourceSpan },
    InvalidExpression { message: String, span: SourceSpan },
}

impl HirValidator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            known_symbols: HashSet::new(),
            known_scopes: HashSet::new(),
            current_return_type: None,
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
        self.known_symbols = module.symbols.iter().map(|symbol| symbol.id).collect();
        self.known_scopes = module.scopes.iter().map(|scope| scope.id).collect();

        if let Some(root_scope) = module.metadata.root_scope {
            self.validate_scope_id(root_scope, module.span, "module root scope");
        }

        for scope in &module.scopes {
            if let Some(parent) = scope.parent {
                self.validate_scope_id(parent, module.span, "scope parent");
            }
            for symbol_id in &scope.symbols {
                self.validate_symbol_id(*symbol_id, module.span, "scope symbol");
            }
        }

        for symbol in &module.symbols {
            self.validate_scope_id(symbol.scope_id, symbol.span, "symbol scope");
        }

        for function in &module.functions {
            self.validate_function(function);
        }

        for global in &module.global_variables {
            self.validate_global_variable(global);
        }

        for type_alias in &module.type_aliases {
            self.validate_type_alias(type_alias);
        }
    }

    fn validate_function(&mut self, function: &HirFunction) {
        self.validate_symbol_id(function.symbol_id, function.span, "function symbol");
        self.validate_scope_id(function.scope_id, function.span, "function scope");

        for parameter in &function.parameters {
            self.validate_symbol_id(parameter.symbol_id, parameter.span, "parameter symbol");
            self.validate_scope_id(parameter.scope_id, parameter.span, "parameter scope");
        }

        for variable in &function.local_variables {
            self.validate_symbol_id(variable.symbol_id, variable.span, "local variable symbol");
            self.validate_scope_id(variable.scope_id, variable.span, "local variable scope");
        }

        let previous_return_type = self.current_return_type.clone();
        self.current_return_type = function.return_type.clone();
        for statement in &function.body {
            self.validate_statement(statement);
        }
        self.current_return_type = previous_return_type;
    }

    fn validate_global_variable(&mut self, global: &HirGlobalVariable) {
        self.validate_symbol_id(global.symbol_id, global.span, "global variable symbol");
        self.validate_scope_id(global.scope_id, global.span, "global variable scope");

        if let Some(initializer) = &global.initializer {
            self.validate_expression(initializer);
        }
    }

    fn validate_type_alias(&mut self, type_alias: &HirTypeAlias) {
        self.validate_symbol_id(type_alias.symbol_id, type_alias.span, "type alias symbol");
        self.validate_scope_id(type_alias.scope_id, type_alias.span, "type alias scope");
    }

    fn validate_statement(&mut self, statement: &HirStatement) {
        match &statement.kind {
            HirStatementKind::LocalVariable {
                initializer,
                variable,
            } => {
                self.validate_symbol_id(variable.symbol_id, variable.span, "local variable symbol");
                self.validate_scope_id(variable.scope_id, variable.span, "local variable scope");
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
                        if let (Some(expected), Some(actual)) =
                            (self.current_return_type.as_ref(), expr.expr_type.as_ref())
                        {
                            if !Self::types_compatible(expected, actual) {
                                self.errors.push(HirValidationError::InvalidExpression {
                                    message: format!(
                                        "Return type mismatch: expected {:?}, got {:?}",
                                        expected, actual
                                    ),
                                    span: expr.span,
                                });
                            }
                        }
                    }
                }
            }
            HirStatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
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
            HirStatementKind::ForNumeric {
                start,
                end,
                step,
                body,
                variable: _,
            } => {
                self.validate_expression(start);
                self.validate_expression(end);
                if let Some(step) = step {
                    self.validate_expression(step);
                }
                for stmt in body {
                    self.validate_statement(stmt);
                }
            }
            HirStatementKind::ForGeneric {
                iterables,
                body,
                variables: _,
            } => {
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
        if let Some(symbol_id) = expression.symbol_id {
            self.validate_symbol_id(symbol_id, expression.span, "expression symbol");
        }

        match &expression.kind {
            HirExpressionKind::Unary {
                operand,
                operator: _,
            } => {
                self.validate_expression(operand);
            }
            HirExpressionKind::Binary {
                left,
                right,
                operator: _,
            } => {
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
            HirExpressionKind::MethodCall {
                receiver,
                arguments,
                method: _,
            } => {
                self.validate_expression(receiver);
                for arg in arguments {
                    self.validate_expression(arg);
                }
            }
            HirExpressionKind::ClosurePlaceholder => {
                // Closures are stored separately to avoid circular dependencies
                // Validation would need to be done on the actual function storage
            }
            HirExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let HirInterpolatedStringPart::Expression(expr) = part {
                        self.validate_expression(expr);
                    }
                }
            }
            HirExpressionKind::BuiltinCall {
                arguments,
                function: _,
            } => {
                for arg in arguments {
                    self.validate_expression(arg);
                }
            }
            HirExpressionKind::Nil
            | HirExpressionKind::Boolean(_)
            | HirExpressionKind::Number(_)
            | HirExpressionKind::String(_)
            | HirExpressionKind::GlobalVariable(_) => {}
            HirExpressionKind::LocalVariable(_) => {
                if expression.symbol_id.is_none() {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: "Local variable reference is missing a symbol".to_string(),
                        span: expression.span,
                    });
                }
                if expression.expr_type.is_none() {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: "Local variable reference is missing a type".to_string(),
                        span: expression.span,
                    });
                }
            }
            HirExpressionKind::Error => {
                self.errors.push(HirValidationError::InvalidExpression {
                    message: "Expression contains error".to_string(),
                    span: expression.span,
                });
            }
        }
    }

    fn validate_symbol_id(&mut self, symbol_id: HirSymbolId, span: SourceSpan, context: &str) {
        if !self.known_symbols.contains(&symbol_id) {
            self.errors.push(HirValidationError::InvalidExpression {
                message: format!("Invalid {context}: unknown symbol #{}", symbol_id.0),
                span,
            });
        }
    }

    fn validate_scope_id(&mut self, scope_id: HirScopeId, span: SourceSpan, context: &str) {
        if !self.known_scopes.contains(&scope_id) {
            self.errors.push(HirValidationError::InvalidExpression {
                message: format!("Invalid {context}: unknown scope #{}", scope_id.0),
                span,
            });
        }
    }

    fn types_compatible(expected: &HirType, actual: &HirType) -> bool {
        expected == actual
            || matches!(expected, HirType::Any | HirType::Unknown)
            || matches!(actual, HirType::Any | HirType::Unknown)
    }
}

impl Default for HirValidator {
    fn default() -> Self {
        Self::new()
    }
}
