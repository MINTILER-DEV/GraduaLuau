use std::collections::HashMap;

use crate::parser::ast_builder::AstNode;
use crate::source::SourceSpan;

use super::error::HirError;
use super::expression::{HirExpression, HirExpressionKind, HirTableField};
use super::function::{HirFunction, HirFunctionMetadata, HirParameter};
use super::ids::{HirFunctionId, HirScopeId, HirSymbolId, HirVariableId};
use super::module::HirModule;
use super::statement::{HirLocalVariable, HirStatement, HirStatementKind};
use super::symbol::{HirScope, HirSymbol, HirSymbolKind};
use super::types::{HirBinaryOperator, HirBuiltinFunction, HirType, HirUnaryOperator};

#[derive(Debug, Clone)]
struct HirBinding {
    symbol_id: HirSymbolId,
    variable_id: Option<HirVariableId>,
    value_type: Option<HirType>,
}

pub struct HirBuilder {
    function_counter: usize,
    variable_counter: usize,
    symbol_counter: usize,
    scope_counter: usize,
    scope_ids: Vec<HirScopeId>,
    bindings: Vec<HashMap<String, HirBinding>>,
    scopes: Vec<HirScope>,
    symbols: Vec<HirSymbol>,
}

impl HirBuilder {
    pub fn new() -> Self {
        let root_scope = HirScopeId::new(0);

        Self {
            function_counter: 0,
            variable_counter: 0,
            symbol_counter: 0,
            scope_counter: 1,
            scope_ids: vec![root_scope],
            bindings: vec![HashMap::new()],
            scopes: vec![HirScope::new(root_scope, None)],
            symbols: Vec::new(),
        }
    }

    pub fn build(&mut self, ast: &AstNode) -> Result<HirModule, HirError> {
        match ast {
            AstNode::Program(program) => self.lower_program(program),
            _ => Err(HirError::InvalidInput("Expected program node".to_string())),
        }
    }

    fn lower_program(
        &mut self,
        program: &crate::parser::ast_builder::Program,
    ) -> Result<HirModule, HirError> {
        let mut module = HirModule::new("main".to_string(), program.span);
        module.metadata.root_scope = Some(self.current_scope_id());

        let mut entry_statements = Vec::new();

        for statement in &program.statements {
            self.lower_statement_to_module(statement, &mut module, &mut entry_statements)?;
        }

        if !entry_statements.is_empty()
            && !module
                .functions
                .iter()
                .any(|function| function.name == "main")
        {
            let main_symbol = self.declare_symbol(
                "main".to_string(),
                HirSymbolKind::Function,
                None,
                program.span,
            );
            let entry_function = HirFunction {
                id: self.next_function_id(),
                symbol_id: main_symbol,
                name: "main".to_string(),
                parameters: Vec::new(),
                local_variables: Self::collect_local_variables(&entry_statements),
                body: entry_statements,
                return_type: None,
                scope_id: self.current_scope_id(),
                is_local: false,
                metadata: HirFunctionMetadata {
                    has_explicit_return: false,
                },
                span: program.span,
            };
            module.functions.push(entry_function);
        }

        module.scopes = self.scopes.clone();
        module.symbols = self.symbols.clone();

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
                let function = self.lower_function(
                    name,
                    receiver,
                    params,
                    return_type,
                    body,
                    *is_local,
                    stmt.span,
                )?;
                module.functions.push(function);
            }
            _ => {
                let lowered = self.lower_statement(stmt)?;
                if !matches!(lowered.kind, HirStatementKind::Block(ref statements) if statements.is_empty())
                {
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
        let return_type = return_type.as_ref().map(|typ| self.lower_type(typ));
        let function_symbol = self.declare_symbol(
            name.to_string(),
            HirSymbolKind::Function,
            Some(HirType::Function),
            span,
        );
        let function_scope = self.enter_scope();

        let mut parameters = Vec::new();
        for (param_name, param_type) in params {
            let param_type = param_type.as_ref().map(|typ| self.lower_type(typ));
            let variable_id = self.next_variable_id();
            let symbol_id = self.declare_variable_symbol(
                param_name.clone(),
                variable_id,
                HirSymbolKind::Parameter,
                param_type.clone(),
                span,
            );
            parameters.push(HirParameter {
                id: variable_id,
                symbol_id,
                name: param_name.clone(),
                param_type,
                scope_id: function_scope,
                span,
            });
        }

        let mut function_body = Vec::new();
        for statement in body {
            function_body.push(self.lower_statement(statement)?);
        }

        self.exit_scope();
        let has_explicit_return = function_body.iter().any(Self::statement_contains_return);
        let local_variables = Self::collect_local_variables(&function_body);

        Ok(HirFunction {
            id: self.next_function_id(),
            symbol_id: function_symbol,
            name: name.to_string(),
            parameters,
            local_variables,
            body: function_body,
            return_type,
            scope_id: function_scope,
            is_local,
            metadata: HirFunctionMetadata {
                has_explicit_return,
            },
            span,
        })
    }

    fn lower_statement(
        &mut self,
        stmt: &crate::parser::ast_builder::Statement,
    ) -> Result<HirStatement, HirError> {
        let kind = match &stmt.kind {
            crate::parser::ast_builder::StatementKind::Empty => HirStatementKind::Block(Vec::new()),
            crate::parser::ast_builder::StatementKind::Expression(expr) => {
                HirStatementKind::Expression(self.lower_expression(expr))
            }
            crate::parser::ast_builder::StatementKind::Return(exprs) => {
                let lowered_exprs = exprs.as_ref().map(|exprs| {
                    exprs
                        .iter()
                        .map(|expr| self.lower_expression(expr))
                        .collect()
                });
                HirStatementKind::Return(lowered_exprs)
            }
            crate::parser::ast_builder::StatementKind::Break => HirStatementKind::Break,
            crate::parser::ast_builder::StatementKind::Continue => HirStatementKind::Continue,
            crate::parser::ast_builder::StatementKind::Local {
                names,
                initializers,
            } => {
                let initializer = initializers.first().map(|expr| self.lower_expression(expr));
                let declared_type = names
                    .first()
                    .and_then(|(_, typ)| typ.as_ref())
                    .map(|typ| self.lower_type(typ));
                let inferred_type = declared_type
                    .clone()
                    .or_else(|| initializer.as_ref().and_then(|expr| expr.expr_type.clone()));
                let variable_id = self.next_variable_id();
                let name = names
                    .first()
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "_".to_string());
                let symbol_id = self.declare_variable_symbol(
                    name.clone(),
                    variable_id,
                    HirSymbolKind::Local,
                    inferred_type.clone(),
                    stmt.span,
                );
                let variable = HirLocalVariable {
                    id: variable_id,
                    symbol_id,
                    name,
                    var_type: inferred_type,
                    scope_id: self.current_scope_id(),
                    span: stmt.span,
                };
                HirStatementKind::LocalVariable {
                    variable,
                    initializer,
                }
            }
            crate::parser::ast_builder::StatementKind::Assignment {
                targets,
                values,
                operator: _,
            } => {
                let lowered_targets = targets
                    .iter()
                    .map(|expr| self.lower_expression(expr))
                    .collect();
                let lowered_values = values
                    .iter()
                    .map(|expr| self.lower_expression(expr))
                    .collect();
                HirStatementKind::Assignment {
                    targets: lowered_targets,
                    values: lowered_values,
                }
            }
            crate::parser::ast_builder::StatementKind::Function { .. } => HirStatementKind::Error,
            crate::parser::ast_builder::StatementKind::TypeAlias { .. } => {
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
        match &expr.kind {
            crate::parser::ast_builder::ExpressionKind::Identifier(name) => {
                if let Some(binding) = self.resolve_binding(name) {
                    if let Some(variable_id) = binding.variable_id {
                        return self.expression(
                            HirExpressionKind::LocalVariable(variable_id),
                            binding.value_type,
                            Some(binding.symbol_id),
                            expr.span,
                        );
                    }

                    return self.expression(
                        HirExpressionKind::GlobalVariable(name.clone()),
                        binding.value_type,
                        Some(binding.symbol_id),
                        expr.span,
                    );
                }

                self.expression(
                    HirExpressionKind::GlobalVariable(name.clone()),
                    None,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::NumberLiteral(number) => self.expression(
                HirExpressionKind::Number(number.parse().unwrap_or(0.0)),
                Some(HirType::Number),
                None,
                expr.span,
            ),
            crate::parser::ast_builder::ExpressionKind::StringLiteral(value) => self.expression(
                HirExpressionKind::String(value.clone()),
                Some(HirType::String),
                None,
                expr.span,
            ),
            crate::parser::ast_builder::ExpressionKind::BooleanLiteral(value) => self.expression(
                HirExpressionKind::Boolean(*value),
                Some(HirType::Boolean),
                None,
                expr.span,
            ),
            crate::parser::ast_builder::ExpressionKind::Nil => {
                self.expression(HirExpressionKind::Nil, Some(HirType::Nil), None, expr.span)
            }
            crate::parser::ast_builder::ExpressionKind::Unary { operator, operand } => {
                let operator = self.lower_unary_operator(operator);
                let operand = self.lower_expression(operand);
                let expr_type = self.unary_result_type(operator, operand.expr_type.as_ref());
                self.expression(
                    HirExpressionKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                    expr_type,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let operator = self.lower_binary_operator(operator);
                let left = self.lower_expression(left);
                let right = self.lower_expression(right);
                let expr_type = self.binary_result_type(
                    operator,
                    left.expr_type.as_ref(),
                    right.expr_type.as_ref(),
                );
                self.expression(
                    HirExpressionKind::Binary {
                        left: Box::new(left),
                        operator,
                        right: Box::new(right),
                    },
                    expr_type,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::Call { callee, arguments } => {
                let lowered_callee = self.lower_expression(callee);
                let lowered_args = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect();

                if let HirExpressionKind::GlobalVariable(name) = &lowered_callee.kind {
                    if let Some(builtin) = self.recognize_builtin(name) {
                        let expr_type = self.builtin_result_type(&builtin);
                        return self.expression(
                            HirExpressionKind::BuiltinCall {
                                function: builtin,
                                arguments: lowered_args,
                            },
                            expr_type,
                            lowered_callee.symbol_id,
                            expr.span,
                        );
                    }
                }

                self.expression(
                    HirExpressionKind::FunctionCall {
                        callee: Box::new(lowered_callee),
                        arguments: lowered_args,
                    },
                    None,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::TableConstructor(fields) => {
                let lowered_fields = fields
                    .iter()
                    .map(|field| self.lower_table_field(field))
                    .collect();
                self.expression(
                    HirExpressionKind::TableConstructor(lowered_fields),
                    Some(HirType::Table),
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::MemberAccess { object, property } => {
                let object = self.lower_expression(object);
                self.expression(
                    HirExpressionKind::FieldAccess {
                        object: Box::new(object),
                        field: property.clone(),
                    },
                    None,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::Index { object, index } => {
                let object = self.lower_expression(object);
                let index = self.lower_expression(index);
                self.expression(
                    HirExpressionKind::Index {
                        object: Box::new(object),
                        index: Box::new(index),
                    },
                    None,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                let receiver = self.lower_expression(receiver);
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect();
                self.expression(
                    HirExpressionKind::MethodCall {
                        receiver: Box::new(receiver),
                        method: method.clone(),
                        arguments,
                    },
                    None,
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::InterpolatedString(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        crate::parser::ast_builder::InterpolatedStringPart::Text(text) => {
                            result.push_str(text)
                        }
                        crate::parser::ast_builder::InterpolatedStringPart::Expression(_) => {
                            result.push('?')
                        }
                    }
                }

                self.expression(
                    HirExpressionKind::String(result),
                    Some(HirType::String),
                    None,
                    expr.span,
                )
            }
            crate::parser::ast_builder::ExpressionKind::Error => {
                self.expression(HirExpressionKind::Error, None, None, expr.span)
            }
        }
    }

    fn lower_table_field(
        &mut self,
        field: &crate::parser::ast_builder::TableField,
    ) -> HirTableField {
        match field {
            crate::parser::ast_builder::TableField::Named { key, value } => HirTableField::Named {
                key: key.clone(),
                value: self.lower_expression(value),
            },
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

    fn expression(
        &self,
        kind: HirExpressionKind,
        expr_type: Option<HirType>,
        symbol_id: Option<HirSymbolId>,
        span: SourceSpan,
    ) -> HirExpression {
        HirExpression {
            kind,
            expr_type,
            symbol_id,
            span,
        }
    }

    fn enter_scope(&mut self) -> HirScopeId {
        let parent = self.current_scope_id();
        let scope_id = HirScopeId::new(self.scope_counter);
        self.scope_counter += 1;
        self.scope_ids.push(scope_id);
        self.bindings.push(HashMap::new());
        self.scopes.push(HirScope::new(scope_id, Some(parent)));
        scope_id
    }

    fn exit_scope(&mut self) {
        self.scope_ids.pop();
        self.bindings.pop();
    }

    fn current_scope_id(&self) -> HirScopeId {
        *self
            .scope_ids
            .last()
            .expect("HIR builder must always have a scope")
    }

    fn declare_symbol(
        &mut self,
        name: String,
        kind: HirSymbolKind,
        value_type: Option<HirType>,
        span: SourceSpan,
    ) -> HirSymbolId {
        let symbol_id = HirSymbolId::new(self.symbol_counter);
        self.symbol_counter += 1;
        let scope_id = self.current_scope_id();
        self.symbols.push(HirSymbol::new(
            symbol_id,
            name.clone(),
            kind,
            scope_id,
            value_type.clone(),
            span,
        ));

        if let Some(scope) = self.scopes.iter_mut().find(|scope| scope.id == scope_id) {
            scope.symbols.push(symbol_id);
        }

        self.bindings
            .last_mut()
            .expect("HIR builder must always have a scope")
            .insert(
                name,
                HirBinding {
                    symbol_id,
                    variable_id: None,
                    value_type,
                },
            );
        symbol_id
    }

    fn declare_variable_symbol(
        &mut self,
        name: String,
        variable_id: HirVariableId,
        kind: HirSymbolKind,
        value_type: Option<HirType>,
        span: SourceSpan,
    ) -> HirSymbolId {
        let symbol_id = self.declare_symbol(name.clone(), kind, value_type.clone(), span);
        self.bindings
            .last_mut()
            .expect("HIR builder must always have a scope")
            .insert(
                name,
                HirBinding {
                    symbol_id,
                    variable_id: Some(variable_id),
                    value_type,
                },
            );
        symbol_id
    }

    fn resolve_binding(&self, name: &str) -> Option<HirBinding> {
        self.bindings
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn next_function_id(&mut self) -> HirFunctionId {
        let id = HirFunctionId::new(self.function_counter);
        self.function_counter += 1;
        id
    }

    fn next_variable_id(&mut self) -> HirVariableId {
        let id = HirVariableId::new(self.variable_counter);
        self.variable_counter += 1;
        id
    }

    fn collect_local_variables(statements: &[HirStatement]) -> Vec<HirLocalVariable> {
        let mut variables = Vec::new();
        for statement in statements {
            match &statement.kind {
                HirStatementKind::LocalVariable { variable, .. } => {
                    variables.push(variable.clone())
                }
                HirStatementKind::Block(statements) => {
                    variables.extend(Self::collect_local_variables(statements))
                }
                HirStatementKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    variables.extend(Self::collect_local_variables(then_block));
                    if let Some(else_block) = else_block {
                        variables.extend(Self::collect_local_variables(else_block));
                    }
                }
                HirStatementKind::While { body, .. }
                | HirStatementKind::RepeatUntil { body, .. }
                | HirStatementKind::ForNumeric { body, .. }
                | HirStatementKind::ForGeneric { body, .. } => {
                    variables.extend(Self::collect_local_variables(body));
                }
                _ => {}
            }
        }
        variables
    }

    fn statement_contains_return(statement: &HirStatement) -> bool {
        match &statement.kind {
            HirStatementKind::Return(_) => true,
            HirStatementKind::Block(statements) => {
                statements.iter().any(Self::statement_contains_return)
            }
            HirStatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(Self::statement_contains_return)
                    || else_block
                        .as_ref()
                        .map(|statements| statements.iter().any(Self::statement_contains_return))
                        .unwrap_or(false)
            }
            HirStatementKind::While { body, .. }
            | HirStatementKind::RepeatUntil { body, .. }
            | HirStatementKind::ForNumeric { body, .. }
            | HirStatementKind::ForGeneric { body, .. } => {
                body.iter().any(Self::statement_contains_return)
            }
            _ => false,
        }
    }

    fn lower_unary_operator(&self, op: &str) -> HirUnaryOperator {
        match op {
            "-" => HirUnaryOperator::Negate,
            "not" => HirUnaryOperator::Not,
            "#" => HirUnaryOperator::Length,
            "~" => HirUnaryOperator::BitwiseNot,
            _ => HirUnaryOperator::Negate,
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
            _ => HirBinaryOperator::Add,
        }
    }

    fn lower_type(&self, type_expr: &crate::parser::ast_builder::TypeExpression) -> HirType {
        match &type_expr.kind {
            crate::parser::ast_builder::TypeExpressionKind::Named(name) => match name.as_str() {
                "nil" => HirType::Nil,
                "boolean" => HirType::Boolean,
                "number" => HirType::Number,
                "string" => HirType::String,
                "table" => HirType::Table,
                "function" => HirType::Function,
                "any" => HirType::Any,
                _ => HirType::Unknown,
            },
            _ => HirType::Unknown,
        }
    }

    fn unary_result_type(
        &self,
        operator: HirUnaryOperator,
        operand_type: Option<&HirType>,
    ) -> Option<HirType> {
        match operator {
            HirUnaryOperator::Negate => operand_type.cloned().or(Some(HirType::Number)),
            HirUnaryOperator::Not => Some(HirType::Boolean),
            HirUnaryOperator::Length => Some(HirType::Number),
            HirUnaryOperator::BitwiseNot => Some(HirType::Number),
        }
    }

    fn binary_result_type(
        &self,
        operator: HirBinaryOperator,
        left_type: Option<&HirType>,
        right_type: Option<&HirType>,
    ) -> Option<HirType> {
        match operator {
            HirBinaryOperator::Add
            | HirBinaryOperator::Subtract
            | HirBinaryOperator::Multiply
            | HirBinaryOperator::Divide
            | HirBinaryOperator::FloorDivide
            | HirBinaryOperator::Modulo
            | HirBinaryOperator::Exponent
            | HirBinaryOperator::BitwiseAnd
            | HirBinaryOperator::BitwiseOr
            | HirBinaryOperator::BitwiseXor
            | HirBinaryOperator::BitwiseShiftLeft
            | HirBinaryOperator::BitwiseShiftRight => Some(HirType::Number),
            HirBinaryOperator::Equal
            | HirBinaryOperator::NotEqual
            | HirBinaryOperator::LessThan
            | HirBinaryOperator::LessEqual
            | HirBinaryOperator::GreaterThan
            | HirBinaryOperator::GreaterEqual
            | HirBinaryOperator::And
            | HirBinaryOperator::Or => Some(HirType::Boolean),
            HirBinaryOperator::Concatenate => {
                if matches!(left_type, Some(HirType::String))
                    || matches!(right_type, Some(HirType::String))
                {
                    Some(HirType::String)
                } else {
                    Some(HirType::Unknown)
                }
            }
        }
    }

    fn builtin_result_type(&self, function: &HirBuiltinFunction) -> Option<HirType> {
        match function {
            HirBuiltinFunction::Print | HirBuiltinFunction::Error | HirBuiltinFunction::Require => {
                None
            }
            HirBuiltinFunction::Type | HirBuiltinFunction::ToString => Some(HirType::String),
            HirBuiltinFunction::ToNumber => Some(HirType::Number),
            HirBuiltinFunction::Pairs | HirBuiltinFunction::Ipairs => Some(HirType::Function),
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::lexer::{Lexer, TokenKind};
    use crate::parser::Parser;
    use crate::source::SourceManager;

    use super::*;

    fn lower_source(source: &str) -> HirModule {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("test.glu"), source.to_string());
        let file = sources.get(file_id).unwrap();
        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();

        loop {
            let token = lexer.next_token();
            let done = matches!(token.kind, TokenKind::EOF);
            tokens.push(token);
            if done {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_program();
        HirBuilder::new().build(&ast).unwrap()
    }

    #[test]
    fn local_references_store_symbol_and_type_metadata() {
        let module = lower_source("local x = 5\nprint(x)");
        let local_symbol = module
            .symbols
            .iter()
            .find(|symbol| symbol.name == "x")
            .unwrap();
        let main = module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap();

        assert_eq!(local_symbol.kind, HirSymbolKind::Local);
        assert_eq!(local_symbol.value_type, Some(HirType::Number));
        assert_eq!(main.local_variables.len(), 1);

        let HirStatementKind::Expression(expression) = &main.body[1].kind else {
            panic!("expected print expression");
        };
        let HirExpressionKind::BuiltinCall { arguments, .. } = &expression.kind else {
            panic!("expected builtin call");
        };

        assert!(matches!(
            arguments[0].kind,
            HirExpressionKind::LocalVariable(_)
        ));
        assert_eq!(arguments[0].symbol_id, Some(local_symbol.id));
        assert_eq!(arguments[0].expr_type, Some(HirType::Number));
    }

    #[test]
    fn function_parameters_store_symbol_and_type_metadata() {
        let module = lower_source("function square(x: number): number\nreturn x * x\nend");
        let function = module
            .functions
            .iter()
            .find(|function| function.name == "square")
            .unwrap();

        assert_eq!(function.parameters.len(), 1);
        assert_eq!(function.parameters[0].param_type, Some(HirType::Number));
        assert_eq!(function.return_type, Some(HirType::Number));

        let HirStatementKind::Return(Some(values)) = &function.body[0].kind else {
            panic!("expected return statement");
        };
        let HirExpressionKind::Binary { left, right, .. } = &values[0].kind else {
            panic!("expected binary expression");
        };

        assert_eq!(left.symbol_id, Some(function.parameters[0].symbol_id));
        assert_eq!(right.symbol_id, Some(function.parameters[0].symbol_id));
        assert_eq!(values[0].expr_type, Some(HirType::Number));
    }
}
