use std::collections::HashMap;

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind};
use crate::semantic::symbol_table::SymbolTable;
use crate::semantic::type_resolution::ResolvedTypeKind;
use crate::source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantEvaluationStatus {
    Constant,
    NonConstant,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstantValue {
    Number(f64),
    Boolean(bool),
    String(String),
    Nil,
}

#[derive(Debug, Clone)]
pub struct ConstantEvaluationResult {
    pub span: SourceSpan,
    pub value: Option<ConstantValue>,
    pub resolved_type: ResolvedTypeKind,
    pub status: ConstantEvaluationStatus,
}

#[derive(Debug)]
pub struct ConstantEvaluator {
    _table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    cache: HashMap<SourceSpan, ConstantEvaluationResult>,
}

impl ConstantEvaluator {
    pub fn new(table: SymbolTable) -> Self {
        Self { _table: table, diagnostics: Vec::new(), cache: HashMap::new() }
    }

    pub fn evaluate(mut self, program: &AstNode) -> (Vec<Diagnostic>, Vec<ConstantEvaluationResult>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        (self.diagnostics, self.cache.into_values().collect())
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { initializers, .. } => {
                for initializer in initializers {
                    self.evaluate_expression(initializer);
                }
            }
            StatementKind::Assignment { targets, values, .. } => {
                for target in targets {
                    self.evaluate_expression(target);
                }
                for value in values {
                    self.evaluate_expression(value);
                }
            }
            StatementKind::Function { body, .. } => {
                self.process_statements(body);
            }
            StatementKind::Return(values) => {
                if let Some(values) = values {
                    for value in values {
                        self.evaluate_expression(value);
                    }
                }
            }
            StatementKind::Expression(expression) => {
                self.evaluate_expression(expression);
            }
            StatementKind::TypeAlias { .. } | StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> ConstantEvaluationResult {
        if let Some(result) = self.cache.get(&expression.span) {
            return result.clone();
        }

        let result = match &expression.kind {
            ExpressionKind::Identifier(_) => self.non_constant(expression.span),
            ExpressionKind::NumberLiteral(source) => {
                match source.parse::<f64>() {
                    Ok(value) => self.constant(expression.span, ConstantValue::Number(value), ResolvedTypeKind::Primitive("number".to_string())),
                    Err(_) => {
                        self.diagnostics.push(
                            Diagnostic::warning("Invalid numeric literal in constant expression.").with_span(expression.span),
                        );
                        self.error(expression.span)
                    }
                }
            }
            ExpressionKind::StringLiteral(value) => self.constant(expression.span, ConstantValue::String(value.clone()), ResolvedTypeKind::Primitive("string".to_string())),
            ExpressionKind::BooleanLiteral(value) => self.constant(expression.span, ConstantValue::Boolean(*value), ResolvedTypeKind::Primitive("boolean".to_string())),
            ExpressionKind::Nil => self.constant(expression.span, ConstantValue::Nil, ResolvedTypeKind::Primitive("nil".to_string())),
            ExpressionKind::Unary { operator, operand } => {
                let operand_result = self.evaluate_expression(operand);
                self.evaluate_unary(expression.span, operator, operand_result)
            }
            ExpressionKind::Binary { left, operator, right } => {
                let left_result = self.evaluate_expression(left);
                let right_result = self.evaluate_expression(right);
                self.evaluate_binary(expression.span, operator, left_result, right_result)
            }
            ExpressionKind::Call { callee, arguments } => {
                self.evaluate_expression(callee);
                for argument in arguments {
                    self.evaluate_expression(argument);
                }
                self.non_constant(expression.span)
            }
            ExpressionKind::TableConstructor(_) => self.non_constant(expression.span),
            ExpressionKind::MemberAccess { object, .. } => {
                self.evaluate_expression(object);
                self.non_constant(expression.span)
            }
            ExpressionKind::Index { object, index } => {
                self.evaluate_expression(object);
                self.evaluate_expression(index);
                self.non_constant(expression.span)
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.evaluate_expression(receiver);
                for argument in arguments {
                    self.evaluate_expression(argument);
                }
                self.non_constant(expression.span)
            }
            ExpressionKind::InterpolatedString(parts) => self.evaluate_interpolated_string(expression.span, parts),
            ExpressionKind::Error => self.error(expression.span),
        };

        self.cache.insert(expression.span, result.clone());
        result
    }

    fn evaluate_unary(&mut self, span: SourceSpan, operator: &str, operand: ConstantEvaluationResult) -> ConstantEvaluationResult {
        match operator {
            "-" => match operand.value {
                Some(ConstantValue::Number(value)) if operand.status == ConstantEvaluationStatus::Constant => {
                    self.constant(span, ConstantValue::Number(-value), ResolvedTypeKind::Primitive("number".to_string()))
                }
                Some(ConstantValue::Number(_)) => self.non_constant(span),
                _ => {
                    if operand.status == ConstantEvaluationStatus::Constant {
                        self.diagnostics.push(
                            Diagnostic::warning("Unary '-' requires a numeric constant operand.").with_span(span),
                        );
                        self.unsupported(span)
                    } else {
                        self.non_constant(span)
                    }
                }
            },
            "not" => match operand.value {
                Some(ConstantValue::Boolean(value)) if operand.status == ConstantEvaluationStatus::Constant => {
                    self.constant(span, ConstantValue::Boolean(!value), ResolvedTypeKind::Primitive("boolean".to_string()))
                }
                Some(ConstantValue::Boolean(_)) => self.non_constant(span),
                _ => {
                    if operand.status == ConstantEvaluationStatus::Constant {
                        self.diagnostics.push(
                            Diagnostic::warning("Unary 'not' requires a boolean constant operand.").with_span(span),
                        );
                        self.unsupported(span)
                    } else {
                        self.non_constant(span)
                    }
                }
            },
            _ => self.non_constant(span),
        }
    }

    fn evaluate_binary(
        &mut self,
        span: SourceSpan,
        operator: &str,
        left: ConstantEvaluationResult,
        right: ConstantEvaluationResult,
    ) -> ConstantEvaluationResult {
        let constant_left = left.status == ConstantEvaluationStatus::Constant;
        let constant_right = right.status == ConstantEvaluationStatus::Constant;

        if operator == "and" {
            if constant_left {
                match left.value {
                    Some(ConstantValue::Boolean(false)) => return self.constant(span, ConstantValue::Boolean(false), ResolvedTypeKind::Primitive("boolean".to_string())),
                    Some(ConstantValue::Boolean(true)) => {}
                    _ => {
                        if left.status == ConstantEvaluationStatus::Constant {
                            self.diagnostics.push(
                                Diagnostic::warning("Logical 'and' requires boolean operands.").with_span(span),
                            );
                            return self.unsupported(span);
                        }
                    }
                }
            }
            if constant_left && constant_right {
                return match right.value {
                    Some(ConstantValue::Boolean(value)) => self.constant(span, ConstantValue::Boolean(value), ResolvedTypeKind::Primitive("boolean".to_string())),
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::warning("Logical 'and' requires boolean operands.").with_span(span),
                        );
                        self.unsupported(span)
                    }
                };
            }
            return self.non_constant(span);
        }

        if operator == "or" {
            if constant_left {
                match left.value {
                    Some(ConstantValue::Boolean(true)) => return self.constant(span, ConstantValue::Boolean(true), ResolvedTypeKind::Primitive("boolean".to_string())),
                    Some(ConstantValue::Boolean(false)) => {}
                    _ => {
                        if left.status == ConstantEvaluationStatus::Constant {
                            self.diagnostics.push(
                                Diagnostic::warning("Logical 'or' requires boolean operands.").with_span(span),
                            );
                            return self.unsupported(span);
                        }
                    }
                }
            }
            if constant_left && constant_right {
                return match right.value {
                    Some(ConstantValue::Boolean(value)) => self.constant(span, ConstantValue::Boolean(value), ResolvedTypeKind::Primitive("boolean".to_string())),
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::warning("Logical 'or' requires boolean operands.").with_span(span),
                        );
                        self.unsupported(span)
                    }
                };
            }
            return self.non_constant(span);
        }

        if !constant_left || !constant_right {
            return self.non_constant(span);
        }

        let left_value = left.value;
        let right_value = right.value;

        match operator {
            "+" | "-" | "*" | "/" | "%" | "^" => {
                match (left_value, right_value) {
                    (Some(ConstantValue::Number(left_num)), Some(ConstantValue::Number(right_num))) => {
                        let value = match operator {
                            "+" => left_num + right_num,
                            "-" => left_num - right_num,
                            "*" => left_num * right_num,
                            "/" => left_num / right_num,
                            "%" => left_num % right_num,
                            "^" => left_num.powf(right_num),
                            _ => unreachable!(),
                        };
                        self.constant(span, ConstantValue::Number(value), ResolvedTypeKind::Primitive("number".to_string()))
                    }
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::warning(format!("Operator '{}' requires numeric constant operands.", operator)).with_span(span),
                        );
                        self.unsupported(span)
                    }
                }
            }
            ".." => match (left_value, right_value) {
                (Some(ConstantValue::String(left_text)), Some(ConstantValue::String(right_text))) => {
                    self.constant(span, ConstantValue::String(format!("{}{}", left_text, right_text)), ResolvedTypeKind::Primitive("string".to_string()))
                }
                _ => {
                    self.diagnostics.push(
                        Diagnostic::warning("Operator '..' requires constant string operands.").with_span(span),
                    );
                    self.unsupported(span)
                }
            },
            "==" | "~=" => {
                let equals = left_value == right_value;
                let result = if operator == "==" { equals } else { !equals };
                self.constant(span, ConstantValue::Boolean(result), ResolvedTypeKind::Primitive("boolean".to_string()))
            }
            "<" | "<=" | ">" | ">=" => {
                match (left_value, right_value) {
                    (Some(ConstantValue::Number(left_num)), Some(ConstantValue::Number(right_num))) => {
                        let result = match operator {
                            "<" => left_num < right_num,
                            "<=" => left_num <= right_num,
                            ">" => left_num > right_num,
                            ">=" => left_num >= right_num,
                            _ => unreachable!(),
                        };
                        self.constant(span, ConstantValue::Boolean(result), ResolvedTypeKind::Primitive("boolean".to_string()))
                    }
                    (Some(ConstantValue::String(left_text)), Some(ConstantValue::String(right_text))) => {
                        let result = match operator {
                            "<" => left_text < right_text,
                            "<=" => left_text <= right_text,
                            ">" => left_text > right_text,
                            ">=" => left_text >= right_text,
                            _ => unreachable!(),
                        };
                        self.constant(span, ConstantValue::Boolean(result), ResolvedTypeKind::Primitive("boolean".to_string()))
                    }
                    _ => {
                        self.diagnostics.push(
                            Diagnostic::warning("Comparison operators require comparable constant operands.").with_span(span),
                        );
                        self.unsupported(span)
                    }
                }
            }
            _ => self.non_constant(span),
        }
    }

    fn evaluate_interpolated_string(
        &mut self,
        span: SourceSpan,
        parts: &[InterpolatedStringPart],
    ) -> ConstantEvaluationResult {
        let mut resolved = String::new();
        for part in parts {
            match part {
                InterpolatedStringPart::Text(text) => resolved.push_str(text),
                InterpolatedStringPart::Expression(expression) => {
                    let part_result = self.evaluate_expression(expression);
                    if part_result.status != ConstantEvaluationStatus::Constant {
                        return self.non_constant(span);
                    }
                    if let Some(value) = part_result.value {
                        resolved.push_str(&self.constant_value_to_string(&value));
                    } else {
                        resolved.push_str("nil");
                    }
                }
            }
        }
        self.constant(span, ConstantValue::String(resolved), ResolvedTypeKind::Primitive("string".to_string()))
    }

    fn constant_value_to_string(&self, value: &ConstantValue) -> String {
        match value {
            ConstantValue::Number(value) => {
                if value.fract() == 0.0 {
                    format!("{:.0}", value)
                } else {
                    value.to_string()
                }
            }
            ConstantValue::Boolean(value) => value.to_string(),
            ConstantValue::String(value) => value.clone(),
            ConstantValue::Nil => "nil".to_string(),
        }
    }

    fn constant(&self, span: SourceSpan, value: ConstantValue, resolved_type: ResolvedTypeKind) -> ConstantEvaluationResult {
        ConstantEvaluationResult { span, value: Some(value), resolved_type, status: ConstantEvaluationStatus::Constant }
    }

    fn non_constant(&self, span: SourceSpan) -> ConstantEvaluationResult {
        ConstantEvaluationResult { span, value: None, resolved_type: ResolvedTypeKind::Unknown, status: ConstantEvaluationStatus::NonConstant }
    }

    fn unsupported(&self, span: SourceSpan) -> ConstantEvaluationResult {
        ConstantEvaluationResult { span, value: None, resolved_type: ResolvedTypeKind::Unknown, status: ConstantEvaluationStatus::Unsupported }
    }

    fn error(&self, span: SourceSpan) -> ConstantEvaluationResult {
        ConstantEvaluationResult { span, value: None, resolved_type: ResolvedTypeKind::Unknown, status: ConstantEvaluationStatus::Unsupported }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Program, Statement, StatementKind};
    use crate::source::{FileId, SourceSpan};
    use crate::semantic::symbol_table::SymbolTable;

    fn evaluate_program(statements: Vec<Statement>) -> Vec<ConstantEvaluationResult> {
        let program = Program { statements, span: SourceSpan::new(FileId::new(0), 0, 0) };
        let table = SymbolTable::new();
        let (_, results) = ConstantEvaluator::new(table).evaluate(&AstNode::Program(program));
        results
    }

    #[test]
    fn evaluates_number_and_string_literals() {
        let results = evaluate_program(vec![
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::NumberLiteral("42".to_string()), span: SourceSpan::new(FileId::new(0), 0, 2) }), span: SourceSpan::new(FileId::new(0), 0, 2) },
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::StringLiteral("hello".to_string()), span: SourceSpan::new(FileId::new(0), 3, 10) }), span: SourceSpan::new(FileId::new(0), 3, 10) },
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::BooleanLiteral(true), span: SourceSpan::new(FileId::new(0), 11, 15) }), span: SourceSpan::new(FileId::new(0), 11, 15) },
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::Nil, span: SourceSpan::new(FileId::new(0), 16, 19) }), span: SourceSpan::new(FileId::new(0), 16, 19) },
        ]);

        assert_eq!(results.len(), 4);
        assert!(matches!(results[0].status, ConstantEvaluationStatus::Constant));
        assert!(matches!(results[1].status, ConstantEvaluationStatus::Constant));
        assert!(matches!(results[2].status, ConstantEvaluationStatus::Constant));
        assert!(matches!(results[3].status, ConstantEvaluationStatus::Constant));
    }

    fn find_result(results: &[ConstantEvaluationResult], span: SourceSpan) -> &ConstantEvaluationResult {
        results
            .iter()
            .find(|result| result.span == span)
            .expect("expected constant evaluation result for span")
    }

    #[test]
    fn folds_arithmetic_and_string_concatenation() {
        let results = evaluate_program(vec![
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::Binary {
                left: Box::new(Expression { kind: ExpressionKind::NumberLiteral("1".to_string()), span: SourceSpan::new(FileId::new(0), 0, 1) }),
                operator: "+".to_string(),
                right: Box::new(Expression { kind: ExpressionKind::NumberLiteral("2".to_string()), span: SourceSpan::new(FileId::new(0), 4, 5) }),
            }, span: SourceSpan::new(FileId::new(0), 0, 5) }), span: SourceSpan::new(FileId::new(0), 0, 5) },
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::Binary {
                left: Box::new(Expression { kind: ExpressionKind::StringLiteral("foo".to_string()), span: SourceSpan::new(FileId::new(0), 6, 11) }),
                operator: "..".to_string(),
                right: Box::new(Expression { kind: ExpressionKind::StringLiteral("bar".to_string()), span: SourceSpan::new(FileId::new(0), 15, 20) }),
            }, span: SourceSpan::new(FileId::new(0), 6, 20) }), span: SourceSpan::new(FileId::new(0), 6, 20) },
        ]);

        let plus_result = find_result(&results, SourceSpan::new(FileId::new(0), 0, 5));
        assert!(matches!(plus_result.status, ConstantEvaluationStatus::Constant));
        assert_eq!(plus_result.value, Some(ConstantValue::Number(3.0)));

        let concat_result = find_result(&results, SourceSpan::new(FileId::new(0), 6, 20));
        assert!(matches!(concat_result.status, ConstantEvaluationStatus::Constant));
        assert_eq!(concat_result.value, Some(ConstantValue::String("foobar".to_string())));
    }

    #[test]
    fn folds_interpolated_strings_when_all_parts_are_constant() {
        let results = evaluate_program(vec![
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::InterpolatedString(vec![
                InterpolatedStringPart::Text("Hello ".to_string()),
                InterpolatedStringPart::Expression(Expression { kind: ExpressionKind::NumberLiteral("3".to_string()), span: SourceSpan::new(FileId::new(0), 7, 8) }),
            ]), span: SourceSpan::new(FileId::new(0), 0, 8) }), span: SourceSpan::new(FileId::new(0), 0, 8) },
        ]);

        let interpolated_result = find_result(&results, SourceSpan::new(FileId::new(0), 0, 8));
        assert!(matches!(interpolated_result.status, ConstantEvaluationStatus::Constant));
        assert_eq!(interpolated_result.value, Some(ConstantValue::String("Hello 3".to_string())));
    }

    #[test]
    fn does_not_evaluate_function_calls() {
        let results = evaluate_program(vec![
            Statement { kind: StatementKind::Expression(Expression { kind: ExpressionKind::Call {
                callee: Box::new(Expression { kind: ExpressionKind::Identifier("foo".to_string()), span: SourceSpan::new(FileId::new(0), 0, 3) }),
                arguments: vec![Expression { kind: ExpressionKind::NumberLiteral("1".to_string()), span: SourceSpan::new(FileId::new(0), 4, 5) }],
            }, span: SourceSpan::new(FileId::new(0), 0, 5) }), span: SourceSpan::new(FileId::new(0), 0, 5) },
        ]);

        let call_result = find_result(&results, SourceSpan::new(FileId::new(0), 0, 5));
        assert!(matches!(call_result.status, ConstantEvaluationStatus::NonConstant));
    }
}
