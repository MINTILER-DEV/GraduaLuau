use std::collections::HashMap;

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Statement, StatementKind, TypeExpression, TypeExpressionKind};
use crate::semantic::symbol_table::{ScopeId, SymbolId, SymbolKind, SymbolNamespace, SymbolTable};
use crate::semantic::type_resolution::ResolvedTypeKind;

#[derive(Debug)]
pub struct TypeChecker {
    table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    scope_stack: Vec<ScopeId>,
    child_index_stack: Vec<usize>,
    symbol_types: HashMap<SymbolId, ResolvedTypeKind>,
    current_return_type: Vec<Option<ResolvedTypeKind>>,
    has_return_stack: Vec<bool>,
}

impl TypeChecker {
    pub fn new(table: SymbolTable) -> Self {
        let root_scope = table.root_scope();
        Self {
            table,
            diagnostics: Vec::new(),
            scope_stack: vec![root_scope],
            child_index_stack: vec![0],
            symbol_types: HashMap::new(),
            current_return_type: vec![None],
            has_return_stack: vec![false],
        }
    }

    pub fn check(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>) {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        (self.table, self.diagnostics)
    }

    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap()
    }

    fn enter_scope(&mut self) {
        let parent = self.current_scope();
        let index = *self.child_index_stack.last().unwrap();
        let child_id = self.table.scope(parent).children[index];
        self.child_index_stack.last_mut().map(|idx| *idx += 1);
        self.scope_stack.push(child_id);
        self.child_index_stack.push(0);
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
        self.child_index_stack.pop();
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { names, initializers } => {
                let initializer_types: Vec<ResolvedTypeKind> = initializers
                    .iter()
                    .map(|expr| self.type_of_expression(expr))
                    .collect();

                for (index, (name, annotation)) in names.iter().enumerate() {
                    let declared_type = annotation
                        .as_ref()
                        .map(|annotation| self.resolve_type_expression(annotation));
                    if let Some(declared_type) = &declared_type {
                        if let Some(initializer_type) = initializer_types.get(index) {
                            if !self.is_assignable(initializer_type, declared_type) {
                                self.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "Cannot assign value of type '{source}' to variable '{name}' of type '{dest}'.",
                                        source = self.type_name(initializer_type),
                                        name = name,
                                        dest = self.type_name(declared_type)
                                    ))
                                    .with_span(statement.span),
                                );
                            }
                        }
                    }

                    if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, self.current_scope()) {
                        if let Some(declared_type) = declared_type {
                            self.symbol_types.insert(symbol.id, declared_type);
                        } else if let Some(initializer_type) = initializer_types.get(index) {
                            if !matches!(initializer_type, ResolvedTypeKind::Unknown) {
                                self.symbol_types.insert(symbol.id, initializer_type.clone());
                            }
                        }
                    }
                }
            }
            StatementKind::Function { params, return_type, body, .. } => {
                let resolved_return_type = return_type
                    .as_ref()
                    .map(|return_type| self.resolve_type_expression(return_type));

                self.enter_scope();

                for (name, annotation) in params {
                    if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, self.current_scope()) {
                        let param_type = annotation
                            .as_ref()
                            .map(|annotation| self.resolve_type_expression(annotation));
                        if let Some(param_type) = param_type {
                            self.symbol_types.insert(symbol.id, param_type);
                        }
                    }
                }

                self.current_return_type.push(resolved_return_type.clone());
                self.has_return_stack.push(false);
                self.process_statements(body);

                let has_return = self.has_return_stack.pop().unwrap_or(false);
                let current_return_type = self.current_return_type.pop().unwrap_or(None);
                self.exit_scope();

                if Self::requires_return(&current_return_type) && !has_return {
                    self.diagnostics.push(
                        Diagnostic::error("Not all execution paths return a value.").with_span(statement.span),
                    );
                }

                if let Some(resolved_return_type) = &current_return_type {
                    if let Some(symbol) = self.table.lookup(
                        &self.function_name(statement),
                        SymbolNamespace::Value,
                        self.current_scope(),
                    ) {
                        self.symbol_types.insert(symbol.id, resolved_return_type.clone());
                    }
                }
            }
            StatementKind::TypeAlias { .. } => {}
            StatementKind::Return(values) => {
                self.has_return_stack.last_mut().map(|val| *val = true);
                let return_type = self.current_return_type.last().cloned().flatten();

                if let Some(values) = values {
                    for value in values {
                        let value_type = self.type_of_expression(value);
                        if let Some(expected) = &return_type {
                            if !self.is_assignable(&value_type, expected) {
                                self.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "Function returns '{source}', expected '{dest}'.",
                                        source = self.type_name(&value_type),
                                        dest = self.type_name(expected)
                                    ))
                                    .with_span(value.span),
                                );
                            }
                        }
                    }
                } else if let Some(expected) = &return_type {
                    if !Self::type_allows_nil(expected) {
                        self.diagnostics.push(
                            Diagnostic::error("Function returns 'nil', expected a non-nil value.").with_span(statement.span),
                        );
                    }
                }
            }
            StatementKind::Assignment { targets, values, .. } => {
                let target_types: Vec<ResolvedTypeKind> = targets.iter().map(|target| self.type_of_expression(target)).collect();
                let value_types: Vec<ResolvedTypeKind> = values.iter().map(|value| self.type_of_expression(value)).collect();

                for (target_type, value_type) in target_types.iter().zip(value_types.iter()) {
                    if !matches!(target_type, ResolvedTypeKind::Unknown)
                        && !matches!(value_type, ResolvedTypeKind::Unknown)
                        && !self.is_assignable(value_type, target_type)
                    {
                        self.diagnostics.push(
                            Diagnostic::error(format!(
                                "Cannot assign value of type '{source}' to target of type '{dest}'.",
                                source = self.type_name(value_type),
                                dest = self.type_name(target_type)
                            ))
                            .with_span(statement.span),
                        );
                    }
                }
            }
            StatementKind::Expression(expression) => {
                self.type_of_expression(expression);
            }
            StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn function_name(&self, statement: &Statement) -> String {
        if let StatementKind::Function { name, .. } = &statement.kind {
            name.clone()
        } else {
            String::new()
        }
    }

    fn type_of_expression(&mut self, expression: &Expression) -> ResolvedTypeKind {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Value, self.current_scope()) {
                    if let Some(resolved) = self.symbol_types.get(&symbol.id) {
                        return resolved.clone();
                    }
                    if let Some(declared_type) = &symbol.declared_type {
                        return self.resolve_type_expression(declared_type);
                    }
                }
                ResolvedTypeKind::Unknown
            }
            ExpressionKind::Unary { operator, operand } => {
                let operand_type = self.type_of_expression(operand);
                match operator.as_str() {
                    "-" => {
                        if self.is_number(&operand_type) {
                            ResolvedTypeKind::Primitive("number".to_string())
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error("Operator '-' requires a numeric operand.").with_span(expression.span),
                            );
                            ResolvedTypeKind::Unknown
                        }
                    }
                    "not" => ResolvedTypeKind::Primitive("boolean".to_string()),
                    _ => ResolvedTypeKind::Unknown,
                }
            }
            ExpressionKind::Binary { left, operator, right } => {
                let left_type = self.type_of_expression(left);
                let right_type = self.type_of_expression(right);
                match operator.as_str() {
                    "+" | "-" | "*" | "/" | "%" | "^" => {
                        if self.is_number(&left_type) && self.is_number(&right_type) {
                            ResolvedTypeKind::Primitive("number".to_string())
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error(format!(
                                    "Operator '{}' cannot be applied to '{}' and '{}'.",
                                    operator,
                                    self.type_name(&left_type),
                                    self.type_name(&right_type)
                                ))
                                .with_span(expression.span),
                            );
                            ResolvedTypeKind::Unknown
                        }
                    }
                    ".." => {
                        if self.is_string(&left_type) && self.is_string(&right_type) {
                            ResolvedTypeKind::Primitive("string".to_string())
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error("Operator '..' requires string operands.").with_span(expression.span),
                            );
                            ResolvedTypeKind::Unknown
                        }
                    }
                    "==" | "~=" | "<" | "<=" | ">" | ">=" => {
                        ResolvedTypeKind::Primitive("boolean".to_string())
                    }
                    "and" | "or" => {
                        if self.is_boolean(&left_type) && self.is_boolean(&right_type) {
                            ResolvedTypeKind::Primitive("boolean".to_string())
                        } else {
                            self.diagnostics.push(
                                Diagnostic::error("Logical operators require boolean operands.").with_span(expression.span),
                            );
                            ResolvedTypeKind::Unknown
                        }
                    }
                    _ => ResolvedTypeKind::Unknown,
                }
            }
            ExpressionKind::Call { callee, arguments } => {
                let callee_type = self.type_of_expression(callee);
                if let ResolvedTypeKind::Function { params, return_type } = callee_type {
                    if params.len() != arguments.len() {
                        self.diagnostics.push(
                            Diagnostic::error(format!(
                                "Function expects {} arguments, but {} were provided.",
                                params.len(),
                                arguments.len()
                            ))
                            .with_span(expression.span),
                        );
                    } else {
                        for (param_type, arg) in params.iter().zip(arguments.iter()) {
                            let arg_type = self.type_of_expression(arg);
                            if !self.is_assignable(&arg_type, param_type) {
                                self.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "Argument type '{}' is not assignable to parameter type '{}'.",
                                        self.type_name(&arg_type),
                                        self.type_name(param_type)
                                    ))
                                    .with_span(arg.span),
                                );
                            }
                        }
                    }
                    *return_type
                } else {
                    ResolvedTypeKind::Unknown
                }
            }
            ExpressionKind::TableConstructor(fields) => {
                let resolved_fields = fields
                    .iter()
                    .filter_map(|field| match field {
                        crate::parser::ast_builder::TableField::Named { key, value } => {
                            Some((key.clone(), self.type_of_expression(value), value.span))
                        }
                        crate::parser::ast_builder::TableField::Indexed { .. } => None,
                        crate::parser::ast_builder::TableField::Expression(_) => None,
                    })
                    .collect();
                ResolvedTypeKind::Table(resolved_fields)
            }
            ExpressionKind::MemberAccess { object, .. } => {
                self.type_of_expression(object);
                ResolvedTypeKind::Unknown
            }
            ExpressionKind::Index { object, index } => {
                self.type_of_expression(object);
                self.type_of_expression(index);
                ResolvedTypeKind::Unknown
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.type_of_expression(receiver);
                for argument in arguments {
                    self.type_of_expression(argument);
                }
                ResolvedTypeKind::Unknown
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let crate::parser::ast_builder::InterpolatedStringPart::Expression(expr) = part {
                        self.type_of_expression(expr);
                    }
                }
                ResolvedTypeKind::Primitive("string".to_string())
            }
            ExpressionKind::NumberLiteral(_) => ResolvedTypeKind::Primitive("number".to_string()),
            ExpressionKind::StringLiteral(_) => ResolvedTypeKind::Primitive("string".to_string()),
            ExpressionKind::BooleanLiteral(_) => ResolvedTypeKind::Primitive("boolean".to_string()),
            ExpressionKind::Nil => ResolvedTypeKind::Primitive("nil".to_string()),
            ExpressionKind::Error => ResolvedTypeKind::Unknown,
        }
    }

    fn resolve_type_expression(&self, type_expression: &TypeExpression) -> ResolvedTypeKind {
        match &type_expression.kind {
            TypeExpressionKind::Named(name) => {
                if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Type, self.current_scope()) {
                    match symbol.kind {
                        SymbolKind::BuiltinType => ResolvedTypeKind::Primitive(name.clone()),
                        SymbolKind::TypeAlias => ResolvedTypeKind::Alias(symbol.id),
                        _ => ResolvedTypeKind::Unknown,
                    }
                } else {
                    ResolvedTypeKind::Unknown
                }
            }
            TypeExpressionKind::Optional(inner) => {
                ResolvedTypeKind::Optional(Box::new(self.resolve_type_expression(inner)))
            }
            TypeExpressionKind::Union(types) => ResolvedTypeKind::Union(
                types.iter().map(|typ| self.resolve_type_expression(typ)).collect(),
            ),
            TypeExpressionKind::Intersection(types) => ResolvedTypeKind::Intersection(
                types.iter().map(|typ| self.resolve_type_expression(typ)).collect(),
            ),
            TypeExpressionKind::Table(fields) => ResolvedTypeKind::Table(
                fields
                    .iter()
                    .map(|(name, typ, span)| (name.clone(), self.resolve_type_expression(typ), *span))
                    .collect(),
            ),
            TypeExpressionKind::Array(element_type) => {
                ResolvedTypeKind::Array(Box::new(self.resolve_type_expression(element_type)))
            }
            TypeExpressionKind::Function { params, return_type } => ResolvedTypeKind::Function {
                params: params.iter().map(|param| self.resolve_type_expression(param)).collect(),
                return_type: Box::new(self.resolve_type_expression(return_type)),
            },
            TypeExpressionKind::Tuple(types) => ResolvedTypeKind::Tuple(
                types.iter().map(|typ| self.resolve_type_expression(typ)).collect(),
            ),
            TypeExpressionKind::Variadic(element_type) => {
                ResolvedTypeKind::Variadic(Box::new(self.resolve_type_expression(element_type)))
            }
            TypeExpressionKind::Parenthesized(inner) => {
                ResolvedTypeKind::Parenthesized(Box::new(self.resolve_type_expression(inner)))
            }
        }
    }

    fn normalize_type(&self, kind: &ResolvedTypeKind, depth: usize) -> ResolvedTypeKind {
        if depth > 32 {
            return ResolvedTypeKind::Unknown;
        }

        match kind {
            ResolvedTypeKind::Alias(symbol_id) => {
                if let Some(symbol) = self.table.symbol(*symbol_id) {
                    if symbol.kind == SymbolKind::TypeAlias {
                        if let Some(alias_type) = &symbol.declared_type {
                            return self.normalize_type(&self.resolve_type_expression(alias_type), depth + 1);
                        }
                    }
                }
                ResolvedTypeKind::Unknown
            }
            ResolvedTypeKind::Parenthesized(inner) => self.normalize_type(inner, depth + 1),
            ResolvedTypeKind::Optional(inner) => {
                ResolvedTypeKind::Optional(Box::new(self.normalize_type(inner, depth + 1)))
            }
            ResolvedTypeKind::Union(types) => ResolvedTypeKind::Union(
                types
                    .iter()
                    .map(|typ| self.normalize_type(typ, depth + 1))
                    .collect(),
            ),
            ResolvedTypeKind::Intersection(types) => ResolvedTypeKind::Intersection(
                types
                    .iter()
                    .map(|typ| self.normalize_type(typ, depth + 1))
                    .collect(),
            ),
            ResolvedTypeKind::Array(element_type) => {
                ResolvedTypeKind::Array(Box::new(self.normalize_type(element_type, depth + 1)))
            }
            ResolvedTypeKind::Function { params, return_type } => ResolvedTypeKind::Function {
                params: params
                    .iter()
                    .map(|param| self.normalize_type(param, depth + 1))
                    .collect(),
                return_type: Box::new(self.normalize_type(return_type, depth + 1)),
            },
            ResolvedTypeKind::Tuple(types) => ResolvedTypeKind::Tuple(
                types
                    .iter()
                    .map(|typ| self.normalize_type(typ, depth + 1))
                    .collect(),
            ),
            ResolvedTypeKind::Table(fields) => ResolvedTypeKind::Table(
                fields
                    .iter()
                    .map(|(name, typ, span)| (name.clone(), self.normalize_type(typ, depth + 1), *span))
                    .collect(),
            ),
            ResolvedTypeKind::Variadic(element_type) => {
                ResolvedTypeKind::Variadic(Box::new(self.normalize_type(element_type, depth + 1)))
            }
            other => other.clone(),
        }
    }

    fn is_assignable(&self, source: &ResolvedTypeKind, destination: &ResolvedTypeKind) -> bool {
        let source = self.normalize_type(source, 0);
        let destination = self.normalize_type(destination, 0);

        if matches!(source, ResolvedTypeKind::Unknown) || matches!(destination, ResolvedTypeKind::Unknown) {
            return true;
        }

        if source == destination {
            return true;
        }

        match (&source, &destination) {
            (ResolvedTypeKind::Primitive(src), ResolvedTypeKind::Primitive(dest)) => src == dest,
            (ResolvedTypeKind::Primitive(src), ResolvedTypeKind::Optional(inner)) => {
                self.is_assignable(&ResolvedTypeKind::Primitive(src.clone()), inner) || src == "nil"
            }
            (ResolvedTypeKind::Primitive(src), ResolvedTypeKind::Union(variants)) => {
                variants.iter().any(|variant| self.is_assignable(&ResolvedTypeKind::Primitive(src.clone()), variant))
            }
            (ResolvedTypeKind::Union(variants), dest) => {
                variants.iter().all(|variant| self.is_assignable(variant, dest))
            }
            (ResolvedTypeKind::Table(fields), ResolvedTypeKind::Table(dest_fields)) => {
                dest_fields.iter().all(|(name, dest_type, _)| {
                    fields
                        .iter()
                        .find(|(field_name, _, _)| field_name == name)
                        .map(|(_, field_type, _)| self.is_assignable(field_type, dest_type))
                        .unwrap_or(false)
                })
            }
            (ResolvedTypeKind::Function { params: src_params, return_type: src_return }, ResolvedTypeKind::Function { params: dest_params, return_type: dest_return }) => {
                if src_params.len() != dest_params.len() {
                    return false;
                }
                src_params
                    .iter()
                    .zip(dest_params.iter())
                    .all(|(src, dest)| self.is_assignable(src, dest))
                    && self.is_assignable(src_return, dest_return)
            }
            (ResolvedTypeKind::Optional(src), dest) => self.is_assignable(src, dest),
            (_, ResolvedTypeKind::Optional(dest)) => self.is_assignable(&source, dest) || matches!(source, ResolvedTypeKind::Primitive(ref name) if name == "nil"),
            (_, ResolvedTypeKind::Union(variants)) => variants.iter().any(|variant| self.is_assignable(&source, variant)),
            _ => false,
        }
    }

    fn type_name(&self, kind: &ResolvedTypeKind) -> String {
        match kind {
            ResolvedTypeKind::Primitive(name) => name.clone(),
            ResolvedTypeKind::Alias(symbol_id) => {
                if let Some(symbol) = self.table.symbol(*symbol_id) {
                    symbol.name.clone()
                } else {
                    "unknown".to_string()
                }
            }
            ResolvedTypeKind::Optional(inner) => format!("{}?", self.type_name(inner)),
            ResolvedTypeKind::Union(types) => {
                types.iter().map(|typ| self.type_name(typ)).collect::<Vec<_>>().join("|")
            }
            ResolvedTypeKind::Intersection(types) => {
                types.iter().map(|typ| self.type_name(typ)).collect::<Vec<_>>().join("&")
            }
            ResolvedTypeKind::Function { params, return_type } => format!(
                "({}) -> {}",
                params.iter().map(|typ| self.type_name(typ)).collect::<Vec<_>>().join(", "),
                self.type_name(return_type)
            ),
            ResolvedTypeKind::Tuple(types) => {
                format!("({})", types.iter().map(|typ| self.type_name(typ)).collect::<Vec<_>>().join(", "))
            }
            ResolvedTypeKind::Table(fields) => {
                let entries = fields.iter().map(|(name, typ, _)| format!("{}: {}", name, self.type_name(typ))).collect::<Vec<_>>();
                format!("{{{}}}", entries.join(", "))
            }
            ResolvedTypeKind::Array(element_type) => format!("{}[]", self.type_name(element_type)),
            ResolvedTypeKind::Variadic(element_type) => format!("...{}", self.type_name(element_type)),
            ResolvedTypeKind::Parenthesized(inner) => self.type_name(inner),
            ResolvedTypeKind::Unknown => "unknown".to_string(),
        }
    }

    fn is_number(&self, kind: &ResolvedTypeKind) -> bool {
        matches!(self.normalize_type(kind, 0), ResolvedTypeKind::Primitive(name) if name == "number")
    }

    fn is_string(&self, kind: &ResolvedTypeKind) -> bool {
        matches!(self.normalize_type(kind, 0), ResolvedTypeKind::Primitive(name) if name == "string")
    }

    fn is_boolean(&self, kind: &ResolvedTypeKind) -> bool {
        matches!(self.normalize_type(kind, 0), ResolvedTypeKind::Primitive(name) if name == "boolean")
    }

    fn type_allows_nil(kind: &ResolvedTypeKind) -> bool {
        match kind {
            ResolvedTypeKind::Primitive(name) => name == "nil",
            ResolvedTypeKind::Optional(_) => true,
            ResolvedTypeKind::Union(types) => types.iter().any(Self::type_allows_nil),
            ResolvedTypeKind::Parenthesized(inner) => Self::type_allows_nil(inner),
            _ => false,
        }
    }

    fn requires_return(return_type: &Option<ResolvedTypeKind>) -> bool {
        match return_type {
            Some(kind) => !Self::type_allows_nil(kind),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Program, Statement, StatementKind, TypeExpression, TypeExpressionKind};
    use crate::semantic::symbol_table::SymbolTableBuilder;
    use crate::source::{FileId, SourceSpan};

    #[test]
    fn reports_local_assignment_type_mismatch() {
        let program = Program {
            statements: vec![Statement {
                kind: StatementKind::Local {
                    names: vec![("x".to_string(), Some(TypeExpression {
                        kind: TypeExpressionKind::Named("number".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 6),
                    }))],
                    initializers: vec![Expression {
                        kind: ExpressionKind::StringLiteral("hello".to_string()),
                        span: SourceSpan::new(FileId::new(0), 0, 7),
                    }],
                },
                span: SourceSpan::new(FileId::new(0), 0, 7),
            }],
            span: SourceSpan::new(FileId::new(0), 0, 7),
        };

        let (table, diagnostics) = SymbolTableBuilder::new().build(&AstNode::Program(program.clone()));
        assert!(diagnostics.is_empty());

        let (_, diagnostics) = TypeChecker::new(table).check(&AstNode::Program(program));
        assert!(diagnostics.iter().any(|diag| diag.message().contains("Cannot assign value of type 'string' to variable 'x'")));
    }
}
