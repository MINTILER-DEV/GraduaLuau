use std::collections::{HashMap, HashSet};

use super::expression::{
    HirExpression, HirExpressionKind, HirInterpolatedStringPart, HirTableField,
};
use super::ids::HirSymbolId;
use super::module::HirModule;
use super::statement::{HirStatement, HirStatementKind};
use super::types::{HirBinaryOperator, HirType, HirUnaryOperator};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HirOptimizationStats {
    pub constant_folds: usize,
    pub constants_propagated: usize,
    pub expressions_simplified: usize,
    pub dead_expressions_removed: usize,
    pub control_flow_simplified: usize,
    pub unreachable_statements_removed: usize,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::hir::{HirBuilder, HirValidator};
    use crate::lexer::{Lexer, TokenKind};
    use crate::parser::ast_builder::AstNode;
    use crate::parser::Parser;
    use crate::source::{FileId, SourceManager, SourceSpan};

    use super::*;

    fn optimize_source(source: &str) -> HirOptimizationResult {
        let ast = parse_source(source);
        let module = HirBuilder::new().build(&ast).unwrap();
        let result = HirOptimizer::new().optimize(&module);
        HirValidator::new().validate(&result.module).unwrap();
        result
    }

    fn parse_source(source: &str) -> AstNode {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("optimizer_test.glu"), source.to_string());
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

        Parser::new(&tokens).parse_program()
    }

    fn main_body(result: &HirOptimizationResult) -> &[HirStatement] {
        &result
            .module
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap()
            .body
    }

    #[test]
    fn folds_constant_arithmetic() {
        let result = optimize_source("local x = 2 + 3");
        let HirStatementKind::LocalVariable {
            initializer: Some(initializer),
            ..
        } = &main_body(&result)[0].kind
        else {
            panic!("expected optimized local initializer");
        };

        assert!(matches!(initializer.kind, HirExpressionKind::Number(5.0)));
        assert_eq!(result.stats.constant_folds, 1);
    }

    #[test]
    fn folds_boolean_and_string_constants() {
        let result = optimize_source("local truth = true and false\nlocal text = \"a\" .. \"b\"");
        let HirStatementKind::LocalVariable {
            initializer: Some(first),
            ..
        } = &main_body(&result)[0].kind
        else {
            panic!("expected boolean local");
        };
        let HirStatementKind::LocalVariable {
            initializer: Some(second),
            ..
        } = &main_body(&result)[1].kind
        else {
            panic!("expected string local");
        };

        assert!(matches!(first.kind, HirExpressionKind::Boolean(false)));
        assert!(matches!(second.kind, HirExpressionKind::String(ref text) if text == "ab"));
        assert_eq!(result.stats.constant_folds, 2);
    }

    #[test]
    fn propagates_unassigned_local_constants() {
        let result = optimize_source("local x = 7\nlocal y = x");
        let HirStatementKind::LocalVariable {
            initializer: Some(initializer),
            ..
        } = &main_body(&result)[1].kind
        else {
            panic!("expected propagated local");
        };

        assert!(matches!(initializer.kind, HirExpressionKind::Number(7.0)));
        assert_eq!(result.stats.constants_propagated, 1);
    }

    #[test]
    fn simplifies_identity_expressions() {
        let result = optimize_source("local x = 4\nlocal y = x + 0");
        let HirStatementKind::LocalVariable {
            initializer: Some(initializer),
            ..
        } = &main_body(&result)[1].kind
        else {
            panic!("expected simplified local");
        };

        assert!(matches!(initializer.kind, HirExpressionKind::Number(4.0)));
        assert!(result.stats.total_changes() >= 1);
    }

    #[test]
    fn removes_dead_expression_statements_but_keeps_calls() {
        let result = optimize_source("1 + 2\nprint(\"alive\")");
        let body = main_body(&result);

        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].kind, HirStatementKind::Expression(_)));
        assert_eq!(result.stats.dead_expressions_removed, 1);
    }

    #[test]
    fn removes_unreachable_statements_after_return() {
        let result = optimize_source("function f()\nreturn 1\nprint(\"nope\")\nend");
        let function = result
            .module
            .functions
            .iter()
            .find(|function| function.name == "f")
            .unwrap();

        assert_eq!(function.body.len(), 1);
        assert_eq!(result.stats.unreachable_statements_removed, 1);
    }

    #[test]
    fn simplifies_constant_if_conditions() {
        let span = SourceSpan::new(FileId::new(0), 0, 1);
        let statement = HirStatement {
            span,
            kind: HirStatementKind::If {
                condition: HirExpression {
                    kind: HirExpressionKind::Boolean(true),
                    expr_type: Some(HirType::Boolean),
                    symbol_id: None,
                    span,
                },
                then_block: vec![HirStatement {
                    kind: HirStatementKind::Return(Some(vec![HirExpression {
                        kind: HirExpressionKind::Number(1.0),
                        expr_type: Some(HirType::Integer),
                        symbol_id: None,
                        span,
                    }])),
                    span,
                }],
                else_block: Some(Vec::new()),
            },
        };
        let mut optimizer = HirOptimizer::new();
        let optimized =
            optimizer.optimize_statements(vec![statement], &mut HashMap::new(), &HashSet::new());

        assert!(matches!(optimized[0].kind, HirStatementKind::Block(_)));
        assert_eq!(optimizer.stats.control_flow_simplified, 1);
    }

    #[test]
    fn reports_optimization_statistics() {
        let result = optimize_source("1 + 2");
        let report = result.stats.report();

        assert!(report.contains("HIR Optimization"));
        assert!(report.contains("Constant Folds: 1"));
        assert!(report.contains("Dead Expressions Removed: 1"));
    }
}

impl HirOptimizationStats {
    pub fn total_changes(&self) -> usize {
        self.constant_folds
            + self.constants_propagated
            + self.expressions_simplified
            + self.dead_expressions_removed
            + self.control_flow_simplified
            + self.unreachable_statements_removed
    }

    pub fn report(&self) -> String {
        format!(
            "HIR Optimization\nConstant Folds: {}\nConstants Propagated: {}\nExpressions Simplified: {}\nDead Expressions Removed: {}\nControl Flow Simplified: {}\nUnreachable Statements Removed: {}",
            self.constant_folds,
            self.constants_propagated,
            self.expressions_simplified,
            self.dead_expressions_removed,
            self.control_flow_simplified,
            self.unreachable_statements_removed
        )
    }
}

#[derive(Debug, Clone)]
pub struct HirOptimizationResult {
    pub module: HirModule,
    pub stats: HirOptimizationStats,
}

#[derive(Debug, Clone, Default)]
pub struct HirOptimizer {
    stats: HirOptimizationStats,
}

impl HirOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn optimize(&mut self, module: &HirModule) -> HirOptimizationResult {
        self.stats = HirOptimizationStats::default();
        let mut optimized = module.clone();

        for global in &mut optimized.global_variables {
            if let Some(initializer) = &mut global.initializer {
                *initializer = self.optimize_expression(initializer.clone(), &HashMap::new());
            }
        }

        for function in &mut optimized.functions {
            self.optimize_function(function);
        }

        HirOptimizationResult {
            module: optimized,
            stats: self.stats.clone(),
        }
    }

    fn optimize_function(&mut self, function: &mut super::function::HirFunction) {
        let assigned_symbols = Self::collect_assigned_symbols(&function.body);
        let mut constants = HashMap::new();
        function.body =
            self.optimize_statements(function.body.clone(), &mut constants, &assigned_symbols);
        function.local_variables = Self::collect_local_variables(&function.body);
        function.metadata.has_explicit_return =
            function.body.iter().any(Self::statement_contains_return);
    }

    fn optimize_statements(
        &mut self,
        statements: Vec<HirStatement>,
        constants: &mut HashMap<HirSymbolId, HirExpression>,
        assigned_symbols: &HashSet<HirSymbolId>,
    ) -> Vec<HirStatement> {
        let mut optimized = Vec::new();
        let mut terminated = false;

        for statement in statements {
            if terminated {
                self.stats.unreachable_statements_removed += 1;
                continue;
            }

            let Some(statement) = self.optimize_statement(statement, constants, assigned_symbols)
            else {
                continue;
            };

            terminated = Self::statement_terminates(&statement);
            optimized.push(statement);
        }

        optimized
    }

    fn optimize_statement(
        &mut self,
        statement: HirStatement,
        constants: &mut HashMap<HirSymbolId, HirExpression>,
        assigned_symbols: &HashSet<HirSymbolId>,
    ) -> Option<HirStatement> {
        let span = statement.span;
        let kind = match statement.kind {
            HirStatementKind::LocalVariable {
                variable,
                initializer,
            } => {
                let initializer =
                    initializer.map(|expression| self.optimize_expression(expression, constants));

                if let Some(initializer) = &initializer {
                    if !assigned_symbols.contains(&variable.symbol_id)
                        && Self::constant_literal(initializer).is_some()
                    {
                        constants.insert(variable.symbol_id, initializer.clone());
                    } else {
                        constants.remove(&variable.symbol_id);
                    }
                } else {
                    constants.remove(&variable.symbol_id);
                }

                HirStatementKind::LocalVariable {
                    variable,
                    initializer,
                }
            }
            HirStatementKind::Assignment { targets, values } => {
                let targets: Vec<_> = targets
                    .into_iter()
                    .map(|expression| self.optimize_expression(expression, constants))
                    .collect();
                let values: Vec<_> = values
                    .into_iter()
                    .map(|expression| self.optimize_expression(expression, constants))
                    .collect();

                for target in &targets {
                    if let Some(symbol_id) = target.symbol_id {
                        constants.remove(&symbol_id);
                    }
                }

                HirStatementKind::Assignment { targets, values }
            }
            HirStatementKind::Expression(expression) => {
                let expression = self.optimize_expression(expression, constants);
                if Self::expression_is_side_effect_free(&expression) {
                    self.stats.dead_expressions_removed += 1;
                    return None;
                }
                HirStatementKind::Expression(expression)
            }
            HirStatementKind::Return(expressions) => {
                HirStatementKind::Return(expressions.map(|values| {
                    values
                        .into_iter()
                        .map(|expression| self.optimize_expression(expression, constants))
                        .collect()
                }))
            }
            HirStatementKind::Block(statements) => {
                let mut nested_constants = constants.clone();
                let statements =
                    self.optimize_statements(statements, &mut nested_constants, assigned_symbols);
                HirStatementKind::Block(statements)
            }
            HirStatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.optimize_expression(condition, constants);
                let mut then_constants = constants.clone();
                let then_block =
                    self.optimize_statements(then_block, &mut then_constants, assigned_symbols);
                let else_block = else_block.map(|else_block| {
                    let mut else_constants = constants.clone();
                    self.optimize_statements(else_block, &mut else_constants, assigned_symbols)
                });

                if let HirExpressionKind::Boolean(value) = condition.kind {
                    self.stats.control_flow_simplified += 1;
                    return Some(HirStatement {
                        kind: HirStatementKind::Block(if value {
                            then_block
                        } else {
                            else_block.unwrap_or_default()
                        }),
                        span,
                    });
                }

                HirStatementKind::If {
                    condition,
                    then_block,
                    else_block,
                }
            }
            HirStatementKind::While { condition, body } => {
                let condition = self.optimize_expression(condition, constants);
                let mut body_constants = constants.clone();
                let body = self.optimize_statements(body, &mut body_constants, assigned_symbols);

                if matches!(condition.kind, HirExpressionKind::Boolean(false)) {
                    self.stats.control_flow_simplified += 1;
                    return None;
                }

                HirStatementKind::While { condition, body }
            }
            HirStatementKind::RepeatUntil { body, condition } => {
                let mut body_constants = constants.clone();
                let body = self.optimize_statements(body, &mut body_constants, assigned_symbols);
                let condition = self.optimize_expression(condition, constants);
                HirStatementKind::RepeatUntil { body, condition }
            }
            HirStatementKind::ForNumeric {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let start = self.optimize_expression(start, constants);
                let end = self.optimize_expression(end, constants);
                let step = step.map(|step| self.optimize_expression(step, constants));
                let mut body_constants = constants.clone();
                let body = self.optimize_statements(body, &mut body_constants, assigned_symbols);
                HirStatementKind::ForNumeric {
                    variable,
                    start,
                    end,
                    step,
                    body,
                }
            }
            HirStatementKind::ForGeneric {
                variables,
                iterables,
                body,
            } => {
                let iterables = iterables
                    .into_iter()
                    .map(|expression| self.optimize_expression(expression, constants))
                    .collect();
                let mut body_constants = constants.clone();
                let body = self.optimize_statements(body, &mut body_constants, assigned_symbols);
                HirStatementKind::ForGeneric {
                    variables,
                    iterables,
                    body,
                }
            }
            HirStatementKind::Function { mut function } => {
                self.optimize_function(&mut function);
                HirStatementKind::Function { function }
            }
            HirStatementKind::Break => HirStatementKind::Break,
            HirStatementKind::Continue => HirStatementKind::Continue,
            HirStatementKind::Error => HirStatementKind::Error,
        };

        Some(HirStatement { kind, span })
    }

    fn optimize_expression(
        &mut self,
        expression: HirExpression,
        constants: &HashMap<HirSymbolId, HirExpression>,
    ) -> HirExpression {
        if let Some(symbol_id) = expression.symbol_id {
            if matches!(
                expression.kind,
                HirExpressionKind::LocalVariable(_) | HirExpressionKind::GlobalVariable(_)
            ) {
                if let Some(constant) = constants.get(&symbol_id) {
                    self.stats.constants_propagated += 1;
                    let mut propagated = constant.clone();
                    propagated.span = expression.span;
                    return propagated;
                }
            }
        }

        let span = expression.span;
        let expr_type = expression.expr_type.clone();
        let symbol_id = expression.symbol_id;
        let kind = match expression.kind {
            HirExpressionKind::Unary { operator, operand } => {
                let operand = self.optimize_expression(*operand, constants);
                return self.fold_or_simplify_unary(operator, operand, expr_type, symbol_id, span);
            }
            HirExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.optimize_expression(*left, constants);
                let right = self.optimize_expression(*right, constants);
                return self
                    .fold_or_simplify_binary(left, operator, right, expr_type, symbol_id, span);
            }
            HirExpressionKind::TableConstructor(fields) => HirExpressionKind::TableConstructor(
                fields
                    .into_iter()
                    .map(|field| self.optimize_table_field(field, constants))
                    .collect(),
            ),
            HirExpressionKind::Index { object, index } => HirExpressionKind::Index {
                object: Box::new(self.optimize_expression(*object, constants)),
                index: Box::new(self.optimize_expression(*index, constants)),
            },
            HirExpressionKind::FieldAccess { object, field } => HirExpressionKind::FieldAccess {
                object: Box::new(self.optimize_expression(*object, constants)),
                field,
            },
            HirExpressionKind::FunctionCall { callee, arguments } => {
                HirExpressionKind::FunctionCall {
                    callee: Box::new(self.optimize_expression(*callee, constants)),
                    arguments: arguments
                        .into_iter()
                        .map(|argument| self.optimize_expression(argument, constants))
                        .collect(),
                }
            }
            HirExpressionKind::MethodCall {
                receiver,
                method,
                arguments,
            } => HirExpressionKind::MethodCall {
                receiver: Box::new(self.optimize_expression(*receiver, constants)),
                method,
                arguments: arguments
                    .into_iter()
                    .map(|argument| self.optimize_expression(argument, constants))
                    .collect(),
            },
            HirExpressionKind::InterpolatedString(parts) => HirExpressionKind::InterpolatedString(
                parts
                    .into_iter()
                    .map(|part| match part {
                        HirInterpolatedStringPart::Text(text) => {
                            HirInterpolatedStringPart::Text(text)
                        }
                        HirInterpolatedStringPart::Expression(expression) => {
                            HirInterpolatedStringPart::Expression(
                                self.optimize_expression(expression, constants),
                            )
                        }
                    })
                    .collect(),
            ),
            HirExpressionKind::BuiltinCall {
                function,
                arguments,
            } => HirExpressionKind::BuiltinCall {
                function,
                arguments: arguments
                    .into_iter()
                    .map(|argument| self.optimize_expression(argument, constants))
                    .collect(),
            },
            other => other,
        };

        HirExpression {
            kind,
            expr_type,
            symbol_id,
            span,
        }
    }

    fn optimize_table_field(
        &mut self,
        field: HirTableField,
        constants: &HashMap<HirSymbolId, HirExpression>,
    ) -> HirTableField {
        match field {
            HirTableField::Named { key, value } => HirTableField::Named {
                key,
                value: self.optimize_expression(value, constants),
            },
            HirTableField::Indexed { key, value } => HirTableField::Indexed {
                key: self.optimize_expression(key, constants),
                value: self.optimize_expression(value, constants),
            },
            HirTableField::Expression(expression) => {
                HirTableField::Expression(self.optimize_expression(expression, constants))
            }
        }
    }

    fn fold_or_simplify_unary(
        &mut self,
        operator: HirUnaryOperator,
        operand: HirExpression,
        expr_type: Option<HirType>,
        symbol_id: Option<HirSymbolId>,
        span: crate::source::SourceSpan,
    ) -> HirExpression {
        match (operator, &operand.kind) {
            (HirUnaryOperator::Negate, HirExpressionKind::Number(value)) => {
                self.stats.constant_folds += 1;
                return Self::literal_number(-value, expr_type, span);
            }
            (HirUnaryOperator::Not, HirExpressionKind::Boolean(value)) => {
                self.stats.constant_folds += 1;
                return Self::literal_boolean(!value, span);
            }
            (
                HirUnaryOperator::Not,
                HirExpressionKind::Unary {
                    operator: HirUnaryOperator::Not,
                    operand,
                },
            ) => {
                self.stats.expressions_simplified += 1;
                return (**operand).clone();
            }
            _ => {}
        }

        HirExpression {
            kind: HirExpressionKind::Unary {
                operator,
                operand: Box::new(operand),
            },
            expr_type,
            symbol_id,
            span,
        }
    }

    fn fold_or_simplify_binary(
        &mut self,
        left: HirExpression,
        operator: HirBinaryOperator,
        right: HirExpression,
        expr_type: Option<HirType>,
        symbol_id: Option<HirSymbolId>,
        span: crate::source::SourceSpan,
    ) -> HirExpression {
        if let Some(folded) =
            self.fold_binary_constants(&left, operator, &right, expr_type.clone(), span)
        {
            return folded;
        }

        if let Some(simplified) = self.simplify_binary_identity(&left, operator, &right) {
            self.stats.expressions_simplified += 1;
            return simplified;
        }

        HirExpression {
            kind: HirExpressionKind::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            },
            expr_type,
            symbol_id,
            span,
        }
    }

    fn fold_binary_constants(
        &mut self,
        left: &HirExpression,
        operator: HirBinaryOperator,
        right: &HirExpression,
        expr_type: Option<HirType>,
        span: crate::source::SourceSpan,
    ) -> Option<HirExpression> {
        match (&left.kind, operator, &right.kind) {
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Add,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_number(left + right, expr_type, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Subtract,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_number(left - right, expr_type, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Multiply,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_number(left * right, expr_type, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Divide,
                HirExpressionKind::Number(right),
            ) if *right != 0.0 => {
                self.stats.constant_folds += 1;
                Some(Self::literal_number(
                    left / right,
                    Some(HirType::Number),
                    span,
                ))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Modulo,
                HirExpressionKind::Number(right),
            ) if *right != 0.0 => {
                self.stats.constant_folds += 1;
                Some(Self::literal_number(left % right, expr_type, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::Equal,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean((left - right).abs() == 0.0, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::NotEqual,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean((left - right).abs() != 0.0, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::LessThan,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(left < right, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::LessEqual,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(left <= right, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::GreaterThan,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(left > right, span))
            }
            (
                HirExpressionKind::Number(left),
                HirBinaryOperator::GreaterEqual,
                HirExpressionKind::Number(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(left >= right, span))
            }
            (
                HirExpressionKind::Boolean(left),
                HirBinaryOperator::And,
                HirExpressionKind::Boolean(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(*left && *right, span))
            }
            (
                HirExpressionKind::Boolean(left),
                HirBinaryOperator::Or,
                HirExpressionKind::Boolean(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_boolean(*left || *right, span))
            }
            (
                HirExpressionKind::String(left),
                HirBinaryOperator::Concatenate,
                HirExpressionKind::String(right),
            ) => {
                self.stats.constant_folds += 1;
                Some(Self::literal_string(format!("{left}{right}"), span))
            }
            _ => None,
        }
    }

    fn simplify_binary_identity(
        &self,
        left: &HirExpression,
        operator: HirBinaryOperator,
        right: &HirExpression,
    ) -> Option<HirExpression> {
        match operator {
            HirBinaryOperator::Add if Self::is_number(right, 0.0) => Some(left.clone()),
            HirBinaryOperator::Add if Self::is_number(left, 0.0) => Some(right.clone()),
            HirBinaryOperator::Subtract if Self::is_number(right, 0.0) => Some(left.clone()),
            HirBinaryOperator::Multiply if Self::is_number(right, 1.0) => Some(left.clone()),
            HirBinaryOperator::Multiply if Self::is_number(left, 1.0) => Some(right.clone()),
            HirBinaryOperator::Divide if Self::is_number(right, 1.0) => Some(left.clone()),
            HirBinaryOperator::And if Self::is_boolean(right, true) => Some(left.clone()),
            HirBinaryOperator::And if Self::is_boolean(left, true) => Some(right.clone()),
            HirBinaryOperator::Or if Self::is_boolean(right, false) => Some(left.clone()),
            HirBinaryOperator::Or if Self::is_boolean(left, false) => Some(right.clone()),
            _ => None,
        }
    }

    fn collect_assigned_symbols(statements: &[HirStatement]) -> HashSet<HirSymbolId> {
        let mut assigned = HashSet::new();
        Self::collect_assigned_symbols_from_statements(statements, &mut assigned);
        assigned
    }

    fn collect_assigned_symbols_from_statements(
        statements: &[HirStatement],
        assigned: &mut HashSet<HirSymbolId>,
    ) {
        for statement in statements {
            match &statement.kind {
                HirStatementKind::Assignment { targets, .. } => {
                    for target in targets {
                        if let Some(symbol_id) = target.symbol_id {
                            assigned.insert(symbol_id);
                        }
                    }
                }
                HirStatementKind::Block(statements)
                | HirStatementKind::While {
                    body: statements, ..
                }
                | HirStatementKind::RepeatUntil {
                    body: statements, ..
                }
                | HirStatementKind::ForNumeric {
                    body: statements, ..
                }
                | HirStatementKind::ForGeneric {
                    body: statements, ..
                } => {
                    Self::collect_assigned_symbols_from_statements(statements, assigned);
                }
                HirStatementKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    Self::collect_assigned_symbols_from_statements(then_block, assigned);
                    if let Some(else_block) = else_block {
                        Self::collect_assigned_symbols_from_statements(else_block, assigned);
                    }
                }
                HirStatementKind::Function { .. }
                | HirStatementKind::LocalVariable { .. }
                | HirStatementKind::Expression(_)
                | HirStatementKind::Return(_)
                | HirStatementKind::Break
                | HirStatementKind::Continue
                | HirStatementKind::Error => {}
            }
        }
    }

    fn collect_local_variables(
        statements: &[HirStatement],
    ) -> Vec<super::statement::HirLocalVariable> {
        let mut variables = Vec::new();
        for statement in statements {
            match &statement.kind {
                HirStatementKind::LocalVariable { variable, .. } => {
                    variables.push(variable.clone())
                }
                HirStatementKind::Block(statements)
                | HirStatementKind::While {
                    body: statements, ..
                }
                | HirStatementKind::RepeatUntil {
                    body: statements, ..
                }
                | HirStatementKind::ForNumeric {
                    body: statements, ..
                }
                | HirStatementKind::ForGeneric {
                    body: statements, ..
                } => {
                    variables.extend(Self::collect_local_variables(statements));
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
                HirStatementKind::Function { .. }
                | HirStatementKind::Assignment { .. }
                | HirStatementKind::Expression(_)
                | HirStatementKind::Return(_)
                | HirStatementKind::Break
                | HirStatementKind::Continue
                | HirStatementKind::Error => {}
            }
        }
        variables
    }

    fn statement_contains_return(statement: &HirStatement) -> bool {
        match &statement.kind {
            HirStatementKind::Return(_) => true,
            HirStatementKind::Block(statements)
            | HirStatementKind::While {
                body: statements, ..
            }
            | HirStatementKind::RepeatUntil {
                body: statements, ..
            }
            | HirStatementKind::ForNumeric {
                body: statements, ..
            }
            | HirStatementKind::ForGeneric {
                body: statements, ..
            } => statements.iter().any(Self::statement_contains_return),
            HirStatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(Self::statement_contains_return)
                    || else_block.as_ref().is_some_and(|statements| {
                        statements.iter().any(Self::statement_contains_return)
                    })
            }
            _ => false,
        }
    }

    fn statement_terminates(statement: &HirStatement) -> bool {
        matches!(
            statement.kind,
            HirStatementKind::Return(_) | HirStatementKind::Break | HirStatementKind::Continue
        )
    }

    fn expression_is_side_effect_free(expression: &HirExpression) -> bool {
        match &expression.kind {
            HirExpressionKind::Nil
            | HirExpressionKind::Boolean(_)
            | HirExpressionKind::Number(_)
            | HirExpressionKind::String(_)
            | HirExpressionKind::LocalVariable(_)
            | HirExpressionKind::GlobalVariable(_)
            | HirExpressionKind::ClosurePlaceholder => true,
            HirExpressionKind::Unary { operand, .. } => {
                Self::expression_is_side_effect_free(operand)
            }
            HirExpressionKind::Binary { left, right, .. } => {
                Self::expression_is_side_effect_free(left)
                    && Self::expression_is_side_effect_free(right)
            }
            HirExpressionKind::TableConstructor(fields) => fields.iter().all(|field| match field {
                HirTableField::Named { value, .. } => Self::expression_is_side_effect_free(value),
                HirTableField::Indexed { key, value } => {
                    Self::expression_is_side_effect_free(key)
                        && Self::expression_is_side_effect_free(value)
                }
                HirTableField::Expression(expression) => {
                    Self::expression_is_side_effect_free(expression)
                }
            }),
            HirExpressionKind::Index { object, index } => {
                Self::expression_is_side_effect_free(object)
                    && Self::expression_is_side_effect_free(index)
            }
            HirExpressionKind::FieldAccess { object, .. } => {
                Self::expression_is_side_effect_free(object)
            }
            HirExpressionKind::InterpolatedString(parts) => parts.iter().all(|part| match part {
                HirInterpolatedStringPart::Text(_) => true,
                HirInterpolatedStringPart::Expression(expression) => {
                    Self::expression_is_side_effect_free(expression)
                }
            }),
            HirExpressionKind::FunctionCall { .. }
            | HirExpressionKind::MethodCall { .. }
            | HirExpressionKind::BuiltinCall { .. }
            | HirExpressionKind::Error => false,
        }
    }

    fn constant_literal(expression: &HirExpression) -> Option<&HirExpression> {
        match expression.kind {
            HirExpressionKind::Nil
            | HirExpressionKind::Boolean(_)
            | HirExpressionKind::Number(_)
            | HirExpressionKind::String(_) => Some(expression),
            _ => None,
        }
    }

    fn is_number(expression: &HirExpression, expected: f64) -> bool {
        matches!(expression.kind, HirExpressionKind::Number(value) if value == expected)
    }

    fn is_boolean(expression: &HirExpression, expected: bool) -> bool {
        matches!(expression.kind, HirExpressionKind::Boolean(value) if value == expected)
    }

    fn literal_number(
        value: f64,
        expr_type: Option<HirType>,
        span: crate::source::SourceSpan,
    ) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Number(value),
            expr_type,
            symbol_id: None,
            span,
        }
    }

    fn literal_boolean(value: bool, span: crate::source::SourceSpan) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::Boolean(value),
            expr_type: Some(HirType::Boolean),
            symbol_id: None,
            span,
        }
    }

    fn literal_string(value: String, span: crate::source::SourceSpan) -> HirExpression {
        HirExpression {
            kind: HirExpressionKind::String(value),
            expr_type: Some(HirType::String),
            symbol_id: None,
            span,
        }
    }
}
