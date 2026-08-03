use crate::parser::ast_builder::AstNode;
use crate::source::SourceSpan;
use super::error::HirError;
use super::module::HirModule;
use super::statement::{HirStatement, HirStatementKind, HirLocalVariable};
use super::function::{HirFunction, HirParameter};
use super::expression::{HirExpression, HirExpressionKind, HirTableField};
use super::ids::{HirFunctionId, HirVariableId};
use super::types::{HirType, HirUnaryOperator, HirBinaryOperator, HirBuiltinFunction};

pub struct HirBuilder {
    function_counter: usize,
    variable_counter: usize,
}

impl HirBuilder {
    pub fn new() -> Self {
        Self {
            function_counter: 0,
            variable_counter: 0,
        }
    }
    
    pub fn build(&mut self, ast: &AstNode) -> Result<HirModule, HirError> {
        match ast {
            AstNode::Program(program) => self.lower_program(program),
            _ => Err(HirError::InvalidInput("Expected program node".to_string())),
        }
    }
    
    fn lower_program(&mut self, program: &crate::parser::ast_builder::Program) -> Result<HirModule, HirError> {
        let mut module = HirModule::new("main".to_string(), program.span);
        let mut entry_statements = Vec::new();
        
        for statement in &program.statements {
            self.lower_statement_to_module(statement, &mut module, &mut entry_statements)?;
        }

        if !entry_statements.is_empty() && !module.functions.iter().any(|function| function.name == "main") {
            let entry_function = HirFunction {
                id: HirFunctionId::new(self.function_counter),
                name: "main".to_string(),
                parameters: Vec::new(),
                body: entry_statements,
                return_type: None,
                is_local: false,
                span: program.span,
            };
            self.function_counter += 1;
            module.functions.push(entry_function);
        }
        
        Ok(module)
    }
    
    fn lower_statement_to_module(
        &mut self,
        stmt: &crate::parser::ast_builder::Statement,
        module: &mut HirModule,
        entry_statements: &mut Vec<HirStatement>,
    ) -> Result<(), HirError> {
        match &stmt.kind {
            crate::parser::ast_builder::StatementKind::Function {
                name,
                receiver,
                params,
                return_type,
                body,
                is_local,
            } => {
                let function = self.lower_function(name, receiver, params, return_type, body, *is_local, stmt.span)?;
                module.functions.push(function);
            }
            _ => {
                let lowered = self.lower_statement(stmt)?;
                if !matches!(lowered.kind, HirStatementKind::Block(ref statements) if statements.is_empty()) {
                    entry_statements.push(lowered);
                }
            }
        }
        
        Ok(())
    }
    
    fn lower_function(
        &mut self,
        name: &str,
        _receiver: &Option<String>,
        params: &[(String, Option<crate::parser::ast_builder::TypeExpression>)],
        return_type: &Option<crate::parser::ast_builder::TypeExpression>,
        body: &[crate::parser::ast_builder::Statement],
        is_local: bool,
        span: SourceSpan,
    ) -> Result<HirFunction, HirError> {
        let id = HirFunctionId::new(self.function_counter);
        self.function_counter += 1;
        
        let mut parameters = Vec::new();
        for (param_name, param_type) in params {
            parameters.push(HirParameter {
                name: param_name.clone(),
                param_type: param_type.as_ref().map(|t| self.lower_type(t)),
                span: span,
            });
        }
        
        let mut function_body = Vec::new();
        for stmt in body {
            function_body.push(self.lower_statement(stmt)?);
        }
        
        Ok(HirFunction {
            id,
            name: name.to_string(),
            parameters,
            body: function_body,
            return_type: return_type.as_ref().map(|t| self.lower_type(t)),
            is_local,
            span,
        })
    }
    
    fn lower_statement(&mut self, stmt: &crate::parser::ast_builder::Statement) -> Result<HirStatement, HirError> {
        let kind = match &stmt.kind {
            crate::parser::ast_builder::StatementKind::Empty => HirStatementKind::Block(Vec::new()),
            
            crate::parser::ast_builder::StatementKind::Expression(expr) => {
                HirStatementKind::Expression(self.lower_expression(expr))
            }
            
            crate::parser::ast_builder::StatementKind::Return(exprs) => {
                let lowered_exprs = exprs.as_ref().map(|exprs| {
                    exprs.iter()
                        .map(|e| self.lower_expression(e))
                        .collect()
                });
                HirStatementKind::Return(lowered_exprs)
            }
            
            crate::parser::ast_builder::StatementKind::Break => HirStatementKind::Break,
            crate::parser::ast_builder::StatementKind::Continue => HirStatementKind::Continue,
            
            crate::parser::ast_builder::StatementKind::Local { names, initializers } => {
                let variable = HirLocalVariable {
                    id: HirVariableId::new(self.variable_counter),
                    name: names.first().map(|n| n.0.clone()).unwrap_or_else(|| "_".to_string()),
                    var_type: names.first().and_then(|(_, t)| t.as_ref()).map(|t| self.lower_type(t)),
                    span: stmt.span,
                };
                self.variable_counter += 1;
                
                let initializer = initializers.first().map(|e| self.lower_expression(e));
                HirStatementKind::LocalVariable { variable, initializer }
            }
            
            crate::parser::ast_builder::StatementKind::Assignment { targets, values, operator: _ } => {
                let lowered_targets = targets.iter().map(|e| self.lower_expression(e)).collect();
                let lowered_values = values.iter().map(|e| self.lower_expression(e)).collect();
                HirStatementKind::Assignment {
                    targets: lowered_targets,
                    values: lowered_values,
                }
            }
            
            crate::parser::ast_builder::StatementKind::Function { .. } => {
                // Nested functions are handled as closures
                HirStatementKind::Error
            }
            
            crate::parser::ast_builder::StatementKind::TypeAlias { .. } => {
                // Type aliases are handled during semantic analysis
                HirStatementKind::Block(Vec::new())
            }
            
            crate::parser::ast_builder::StatementKind::Error => HirStatementKind::Error,
        };
        
        Ok(HirStatement {
            kind,
            span: stmt.span,
        })
    }
    
    fn lower_expression(&mut self, expr: &crate::parser::ast_builder::Expression) -> HirExpression {
        let kind = match &expr.kind {
            crate::parser::ast_builder::ExpressionKind::Identifier(name) => {
                // For now, treat all identifiers as global variables
                HirExpressionKind::GlobalVariable(name.clone())
            }
            
            crate::parser::ast_builder::ExpressionKind::NumberLiteral(n) => {
                HirExpressionKind::Number(n.parse().unwrap_or(0.0))
            }
            
            crate::parser::ast_builder::ExpressionKind::StringLiteral(s) => {
                HirExpressionKind::String(s.clone())
            }
            
            crate::parser::ast_builder::ExpressionKind::BooleanLiteral(b) => {
                HirExpressionKind::Boolean(*b)
            }
            
            crate::parser::ast_builder::ExpressionKind::Nil => {
                HirExpressionKind::Nil
            }
            
            crate::parser::ast_builder::ExpressionKind::Unary { operator, operand } => {
                let op = self.lower_unary_operator(operator);
                HirExpressionKind::Unary {
                    operator: op,
                    operand: Box::new(self.lower_expression(operand)),
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::Binary { left, operator, right } => {
                let op = self.lower_binary_operator(operator);
                HirExpressionKind::Binary {
                    left: Box::new(self.lower_expression(left)),
                    operator: op,
                    right: Box::new(self.lower_expression(right)),
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::Call { callee, arguments } => {
                let lowered_callee = self.lower_expression(callee);
                let lowered_args = arguments.iter().map(|a| self.lower_expression(a)).collect();
                
                // Check if this is a built-in function call
                if let HirExpressionKind::GlobalVariable(name) = &lowered_callee.kind {
                    if let Some(builtin) = self.recognize_builtin(name) {
                        return HirExpression {
                            kind: HirExpressionKind::BuiltinCall {
                                function: builtin,
                                arguments: lowered_args,
                            },
                            expr_type: None,
                            span: expr.span,
                        };
                    }
                }
                
                HirExpressionKind::FunctionCall {
                    callee: Box::new(lowered_callee),
                    arguments: lowered_args,
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::TableConstructor(fields) => {
                let lowered_fields = fields.iter().map(|f| self.lower_table_field(f)).collect();
                HirExpressionKind::TableConstructor(lowered_fields)
            }
            
            crate::parser::ast_builder::ExpressionKind::MemberAccess { object, property } => {
                HirExpressionKind::FieldAccess {
                    object: Box::new(self.lower_expression(object)),
                    field: property.clone(),
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::Index { object, index } => {
                HirExpressionKind::Index {
                    object: Box::new(self.lower_expression(object)),
                    index: Box::new(self.lower_expression(index)),
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::MethodCall { receiver, method, arguments } => {
                HirExpressionKind::MethodCall {
                    receiver: Box::new(self.lower_expression(receiver)),
                    method: method.clone(),
                    arguments: arguments.iter().map(|a| self.lower_expression(a)).collect(),
                }
            }
            
            crate::parser::ast_builder::ExpressionKind::InterpolatedString(parts) => {
                // For now, just concatenate string parts
                let mut result = String::new();
                for part in parts {
                    match part {
                        crate::parser::ast_builder::InterpolatedStringPart::Text(text) => {
                            result.push_str(text);
                        }
                        crate::parser::ast_builder::InterpolatedStringPart::Expression(_) => {
                            // TODO: Proper handling of interpolated expressions
                            result.push_str("?");
                        }
                    }
                }
                HirExpressionKind::String(result)
            }
            
            crate::parser::ast_builder::ExpressionKind::Error => HirExpressionKind::Error,
        };
        
        HirExpression {
            kind,
            expr_type: None,
            span: expr.span,
        }
    }
    
    fn lower_table_field(&mut self, field: &crate::parser::ast_builder::TableField) -> HirTableField {
        match field {
            crate::parser::ast_builder::TableField::Named { key, value } => {
                HirTableField::Named {
                    key: key.clone(),
                    value: self.lower_expression(value),
                }
            }
            crate::parser::ast_builder::TableField::Indexed { key, value } => {
                HirTableField::Indexed {
                    key: self.lower_expression(key),
                    value: self.lower_expression(value),
                }
            }
            crate::parser::ast_builder::TableField::Expression(expr) => {
                HirTableField::Expression(self.lower_expression(expr))
            }
        }
    }
    
    fn lower_unary_operator(&self, op: &str) -> HirUnaryOperator {
        match op {
            "-" => HirUnaryOperator::Negate,
            "not" => HirUnaryOperator::Not,
            "#" => HirUnaryOperator::Length,
            "~" => HirUnaryOperator::BitwiseNot,
            _ => HirUnaryOperator::Negate, // Default fallback
        }
    }
    
    fn lower_binary_operator(&self, op: &str) -> HirBinaryOperator {
        match op {
            "+" => HirBinaryOperator::Add,
            "-" => HirBinaryOperator::Subtract,
            "*" => HirBinaryOperator::Multiply,
            "/" => HirBinaryOperator::Divide,
            "//" => HirBinaryOperator::FloorDivide,
            "%" => HirBinaryOperator::Modulo,
            "^" => HirBinaryOperator::Exponent,
            "==" => HirBinaryOperator::Equal,
            "~=" => HirBinaryOperator::NotEqual,
            "<" => HirBinaryOperator::LessThan,
            "<=" => HirBinaryOperator::LessEqual,
            ">" => HirBinaryOperator::GreaterThan,
            ">=" => HirBinaryOperator::GreaterEqual,
            "and" => HirBinaryOperator::And,
            "or" => HirBinaryOperator::Or,
            ".." => HirBinaryOperator::Concatenate,
            "&" => HirBinaryOperator::BitwiseAnd,
            "|" => HirBinaryOperator::BitwiseOr,
            "~" => HirBinaryOperator::BitwiseXor,
            "<<" => HirBinaryOperator::BitwiseShiftLeft,
            ">>" => HirBinaryOperator::BitwiseShiftRight,
            _ => HirBinaryOperator::Add, // Default fallback
        }
    }
    
    fn lower_type(&self, type_expr: &crate::parser::ast_builder::TypeExpression) -> HirType {
        match &type_expr.kind {
            crate::parser::ast_builder::TypeExpressionKind::Named(name) => {
                match name.as_str() {
                    "nil" => HirType::Nil,
                    "boolean" => HirType::Boolean,
                    "number" => HirType::Number,
                    "string" => HirType::String,
                    "table" => HirType::Table,
                    "function" => HirType::Function,
                    "any" => HirType::Any,
                    _ => HirType::Unknown,
                }
            }
            _ => HirType::Unknown,
        }
    }
    
    fn recognize_builtin(&self, name: &str) -> Option<HirBuiltinFunction> {
        match name {
            "print" => Some(HirBuiltinFunction::Print),
            "type" => Some(HirBuiltinFunction::Type),
            "tonumber" => Some(HirBuiltinFunction::ToNumber),
            "tostring" => Some(HirBuiltinFunction::ToString),
            "error" => Some(HirBuiltinFunction::Error),
            "pairs" => Some(HirBuiltinFunction::Pairs),
            "ipairs" => Some(HirBuiltinFunction::Ipairs),
            "require" => Some(HirBuiltinFunction::Require),
            _ => None,
        }
    }
}
