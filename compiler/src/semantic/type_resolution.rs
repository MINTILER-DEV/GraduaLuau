use crate::parser::ast_builder::{AstNode, Statement, StatementKind, TypeExpression, TypeExpressionKind};
use crate::semantic::symbol_table::{SymbolId, SymbolKind, SymbolNamespace, SymbolTable};
use crate::source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTypeKind {
    Primitive(String),
    Alias(SymbolId),
    Optional(Box<ResolvedTypeKind>),
    Union(Vec<ResolvedTypeKind>),
    Intersection(Vec<ResolvedTypeKind>),
    Function {
        params: Vec<ResolvedTypeKind>,
        return_type: Box<ResolvedTypeKind>,
    },
    Tuple(Vec<ResolvedTypeKind>),
    Table(Vec<(String, ResolvedTypeKind, SourceSpan)>),
    Array(Box<ResolvedTypeKind>),
    Variadic(Box<ResolvedTypeKind>),
    Parenthesized(Box<ResolvedTypeKind>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    pub kind: ResolvedTypeKind,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub struct TypeResolver {
    table: SymbolTable,
    resolved_types: Vec<ResolvedType>,
}

impl TypeResolver {
    pub fn new(table: SymbolTable) -> Self {
        Self { table, resolved_types: Vec::new() }
    }

    pub fn resolve(&mut self, type_expression: &TypeExpression) -> ResolvedType {
        let kind = self.resolve_type_expression(type_expression);
        ResolvedType { kind, span: type_expression.span }
    }

    pub fn analyze(&mut self, program: &AstNode) -> Vec<ResolvedType> {
        if let AstNode::Program(program) = program {
            self.process_statements(&program.statements);
        }
        std::mem::take(&mut self.resolved_types)
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { names, initializers } => {
                for initializer in initializers {
                    self.process_expression(initializer);
                }
                for (_, annotation) in names {
                    if let Some(annotation) = annotation {
                        self.push_resolved_type(annotation);
                    }
                }
            }
            StatementKind::Function { params, return_type, body, .. } => {
                for (_, annotation) in params {
                    if let Some(annotation) = annotation {
                        self.push_resolved_type(annotation);
                    }
                }
                if let Some(return_type) = return_type {
                    self.push_resolved_type(return_type);
                }
                self.process_statements(body);
            }
            StatementKind::TypeAlias { alias, .. } => {
                self.push_resolved_type(alias);
            }
            StatementKind::Return(values) => {
                if let Some(values) = values {
                    for value in values {
                        self.process_expression(value);
                    }
                }
            }
            StatementKind::Assignment { targets, values, .. } => {
                for target in targets {
                    self.process_expression(target);
                }
                for value in values {
                    self.process_expression(value);
                }
            }
            StatementKind::Expression(expression) => self.process_expression(expression),
            StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn process_expression(&mut self, expression: &crate::parser::ast_builder::Expression) {
        match &expression.kind {
            crate::parser::ast_builder::ExpressionKind::Identifier(_) => {}
            crate::parser::ast_builder::ExpressionKind::Unary { operand, .. } => self.process_expression(operand),
            crate::parser::ast_builder::ExpressionKind::Binary { left, right, .. } => {
                self.process_expression(left);
                self.process_expression(right);
            }
            crate::parser::ast_builder::ExpressionKind::Call { callee, arguments } => {
                self.process_expression(callee);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            crate::parser::ast_builder::ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        crate::parser::ast_builder::TableField::Named { value, .. } => self.process_expression(value),
                        crate::parser::ast_builder::TableField::Indexed { key, value } => {
                            self.process_expression(key);
                            self.process_expression(value);
                        }
                        crate::parser::ast_builder::TableField::Expression(expr) => self.process_expression(expr),
                    }
                }
            }
            crate::parser::ast_builder::ExpressionKind::MemberAccess { object, .. } => {
                self.process_expression(object);
            }
            crate::parser::ast_builder::ExpressionKind::Index { object, index } => {
                self.process_expression(object);
                self.process_expression(index);
            }
            crate::parser::ast_builder::ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.process_expression(receiver);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            crate::parser::ast_builder::ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let crate::parser::ast_builder::InterpolatedStringPart::Expression(expr) = part {
                        self.process_expression(expr);
                    }
                }
            }
            crate::parser::ast_builder::ExpressionKind::NumberLiteral(_)
            | crate::parser::ast_builder::ExpressionKind::StringLiteral(_)
            | crate::parser::ast_builder::ExpressionKind::BooleanLiteral(_)
            | crate::parser::ast_builder::ExpressionKind::Nil
            | crate::parser::ast_builder::ExpressionKind::Error => {}
        }
    }

    fn push_resolved_type(&mut self, type_expression: &TypeExpression) {
        let kind = self.resolve_type_expression(type_expression);
        self.resolved_types.push(ResolvedType { kind, span: type_expression.span });
    }

    fn resolve_type_expression(&mut self, type_expression: &TypeExpression) -> ResolvedTypeKind {
        match &type_expression.kind {
            TypeExpressionKind::Named(name) => {
                if let Some(symbol) = self.table.lookup(name, SymbolNamespace::Type, self.table.root_scope()) {
                    match symbol.kind {
                        SymbolKind::BuiltinType => ResolvedTypeKind::Primitive(name.clone()),
                        SymbolKind::TypeAlias => ResolvedTypeKind::Alias(symbol.id),
                        _ => ResolvedTypeKind::Unknown,
                    }
                } else {
                    ResolvedTypeKind::Unknown
                }
            }
            TypeExpressionKind::Optional(inner) => ResolvedTypeKind::Optional(Box::new(self.resolve_type_expression(inner))),
            TypeExpressionKind::Union(types) => ResolvedTypeKind::Union(types.iter().map(|typ| self.resolve_type_expression(typ)).collect()),
            TypeExpressionKind::Intersection(types) => ResolvedTypeKind::Intersection(types.iter().map(|typ| self.resolve_type_expression(typ)).collect()),
            TypeExpressionKind::Table(fields) => ResolvedTypeKind::Table(fields.iter().map(|(name, typ, span)| (name.clone(), self.resolve_type_expression(typ), *span)).collect()),
            TypeExpressionKind::Array(element_type) => ResolvedTypeKind::Array(Box::new(self.resolve_type_expression(element_type))),
            TypeExpressionKind::Function { params, return_type } => ResolvedTypeKind::Function { params: params.iter().map(|param| self.resolve_type_expression(param)).collect(), return_type: Box::new(self.resolve_type_expression(return_type)) },
            TypeExpressionKind::Tuple(types) => ResolvedTypeKind::Tuple(types.iter().map(|typ| self.resolve_type_expression(typ)).collect()),
            TypeExpressionKind::Variadic(element_type) => ResolvedTypeKind::Variadic(Box::new(self.resolve_type_expression(element_type))),
            TypeExpressionKind::Parenthesized(inner) => ResolvedTypeKind::Parenthesized(Box::new(self.resolve_type_expression(inner))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast_builder::{TypeExpression, TypeExpressionKind};
    use crate::source::{FileId, SourceSpan};

    #[test]
    fn resolves_builtin_number_type() {
        let table = SymbolTable::new();
        let mut resolver = TypeResolver::new(table);
        let type_expr = TypeExpression {
            kind: TypeExpressionKind::Named("number".to_string()),
            span: SourceSpan::new(FileId::new(0), 0, 6),
        };

        let resolved = resolver.resolve(&type_expr);
        assert_eq!(resolved.kind, ResolvedTypeKind::Primitive("number".to_string()));
    }
}
