use std::collections::HashMap;

use crate::diagnostics::Diagnostic;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, Statement, StatementKind, TableField, TypeExpression, TypeExpressionKind};
use crate::source::{FileId, SourceSpan};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Method,
    TypeAlias,
    BuiltinFunction,
    BuiltinType,
    Module,
    NativeFunction,
    GenericParameter,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub declaring_scope: ScopeId,
    pub declaration: Option<Statement>,
    pub declared_type: Option<TypeExpression>,
    pub span: SourceSpan,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SymbolNamespace {
    Value,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeOwner {
    Root {
        span: SourceSpan,
    },
    Function {
        name: String,
        span: SourceSpan,
    },
    Block {
        kind: String,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub id: ScopeId,
    pub parent: Option<ScopeId>,
    pub children: Vec<ScopeId>,
    pub owner: ScopeOwner,
    pub value_symbols: HashMap<String, SymbolId>,
    pub type_symbols: HashMap<String, SymbolId>,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub root_scope: ScopeId,
    pub scopes: Vec<Scope>,
    pub symbols: Vec<Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let root_scope = ScopeId(0);
        let mut scopes = Vec::new();
        scopes.push(Scope {
            id: root_scope,
            parent: None,
            children: Vec::new(),
            owner: ScopeOwner::Root { span: SourceSpan::new(FileId::new(0), 0, 0) },
            value_symbols: HashMap::new(),
            type_symbols: HashMap::new(),
        });

        let mut table = SymbolTable { root_scope, scopes, symbols: Vec::new() };
        table.register_builtin_symbols();
        table
    }

    pub fn root_scope(&self) -> ScopeId {
        self.root_scope
    }

    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    pub fn symbol_count(&self) -> usize {
        self.symbols.len()
    }

    pub fn symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.0)
    }

    pub fn lookup(&self, name: &str, namespace: SymbolNamespace, start_scope: ScopeId) -> Option<&Symbol> {
        let mut current = Some(start_scope);
        while let Some(scope_id) = current {
            let scope = &self.scopes[scope_id.0];
            let symbol_id = match namespace {
                SymbolNamespace::Value => scope.value_symbols.get(name),
                SymbolNamespace::Type => scope.type_symbols.get(name),
            };
            if let Some(&symbol_id) = symbol_id {
                return self.symbol(symbol_id);
            }
            current = scope.parent;
        }
        None
    }

    pub fn symbol_by_name(&self, name: &str, namespace: SymbolNamespace, start_scope: ScopeId) -> Option<SymbolId> {
        self.lookup(name, namespace, start_scope).map(|symbol| symbol.id)
    }

    pub fn scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0]
    }

    fn next_symbol_id(&self) -> SymbolId {
        SymbolId(self.symbols.len())
    }

    fn insert_symbol(&mut self, scope_id: ScopeId, symbol: Symbol) -> SymbolId {
        let id = symbol.id;
        let scope = &mut self.scopes[scope_id.0];
        match symbol.kind {
            SymbolKind::TypeAlias | SymbolKind::BuiltinType | SymbolKind::GenericParameter => {
                scope.type_symbols.insert(symbol.name.clone(), id);
            }
            _ => {
                scope.value_symbols.insert(symbol.name.clone(), id);
            }
        }
        self.symbols.push(symbol);
        id
    }

    fn register_builtin_symbols(&mut self) {
        let builtins = [
            ("print", SymbolKind::BuiltinFunction),
            ("warn", SymbolKind::BuiltinFunction),
            ("require", SymbolKind::BuiltinFunction),
            ("pairs", SymbolKind::BuiltinFunction),
            ("ipairs", SymbolKind::BuiltinFunction),
            ("math", SymbolKind::BuiltinFunction),
            ("string", SymbolKind::BuiltinFunction),
            ("table", SymbolKind::BuiltinFunction),
            ("coroutine", SymbolKind::BuiltinFunction),
            ("task", SymbolKind::BuiltinFunction),
            ("os", SymbolKind::BuiltinFunction),
        ];

        for (name, kind) in builtins {
            let id = self.next_symbol_id();
            let symbol = Symbol {
                id,
                name: name.to_string(),
                kind,
                declaring_scope: self.root_scope,
                declaration: None,
                declared_type: None,
                span: SourceSpan::new(FileId::new(0), 0, 0),
            };
            self.insert_symbol(self.root_scope, symbol);
        }

        let builtin_types = [
            "number",
            "string",
            "boolean",
            "nil",
            "thread",
            "buffer",
            "any",
            "unknown",
            "never",
        ];

        for name in builtin_types {
            let id = self.next_symbol_id();
            let symbol = Symbol {
                id,
                name: name.to_string(),
                kind: SymbolKind::BuiltinType,
                declaring_scope: self.root_scope,
                declaration: None,
                declared_type: None,
                span: SourceSpan::new(FileId::new(0), 0, 0),
            };
            self.insert_symbol(self.root_scope, symbol);
        }
    }
}

#[derive(Debug)]
pub struct SymbolTableBuilder {
    pub table: SymbolTable,
    scope_stack: Vec<ScopeId>,
    diagnostics: Vec<Diagnostic>,
}

impl SymbolTableBuilder {
    pub fn new() -> Self {
        Self { table: SymbolTable::new(), scope_stack: vec![ScopeId(0)], diagnostics: Vec::new() }
    }

    pub fn build(mut self, program: &AstNode) -> (SymbolTable, Vec<Diagnostic>) {
        if let AstNode::Program(program) = program {
            self.table.scopes[0].owner = ScopeOwner::Root { span: program.span };
            self.process_statements(&program.statements);
        }
        (self.table, self.diagnostics)
    }

    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap()
    }

    fn enter_scope(&mut self, owner: ScopeOwner) -> ScopeId {
        let id = ScopeId(self.table.scopes.len());
        let parent = Some(self.current_scope());
        self.table.scopes.push(Scope {
            id,
            parent,
            children: Vec::new(),
            owner,
            value_symbols: HashMap::new(),
            type_symbols: HashMap::new(),
        });
        self.table.scopes[parent.unwrap().0].children.push(id);
        self.scope_stack.push(id);
        id
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn process_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.process_statement(statement);
        }
    }

    fn insert_symbol(
        &mut self,
        name: String,
        kind: SymbolKind,
        declaration: Option<Statement>,
        declared_type: Option<TypeExpression>,
        span: SourceSpan,
    ) {
        let current_scope = self.current_scope();
        let scope = &self.table.scopes[current_scope.0];
        let exists = match kind {
            SymbolKind::TypeAlias | SymbolKind::BuiltinType | SymbolKind::GenericParameter => {
                scope.type_symbols.contains_key(&name)
            }
            _ => scope.value_symbols.contains_key(&name),
        };
        if exists {
            self.diagnostics.push(
                Diagnostic::error(format!("Duplicate declaration of '{name}'."))
                    .with_span(span),
            );
            return;
        }

        let symbol = Symbol {
            id: self.table.next_symbol_id(),
            name: name.clone(),
            kind,
            declaring_scope: current_scope,
            declaration,
            declared_type,
            span,
        };

        self.table.insert_symbol(current_scope, symbol);
    }

    fn process_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Local { names, initializers } => {
                for initializer in initializers {
                    self.process_expression(initializer);
                }
                for (name, annotation) in names {
                    self.insert_symbol(
                        name.clone(),
                        SymbolKind::Variable,
                        Some(statement.clone()),
                        annotation.clone(),
                        statement.span,
                    );
                }
            }
            StatementKind::Function { name, receiver, params, return_type, body, .. } => {
                let kind = if receiver.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                self.insert_symbol(
                    name.clone(),
                    kind,
                    Some(statement.clone()),
                    return_type.clone(),
                    statement.span,
                );

                let _function_scope = self.enter_scope(ScopeOwner::Function { name: name.clone(), span: statement.span });
                for (name, annotation) in params {
                    self.insert_symbol(
                        name.clone(),
                        SymbolKind::Parameter,
                        Some(statement.clone()),
                        annotation.clone(),
                        statement.span,
                    );
                }
                self.process_statements(body);
                self.exit_scope();
            }
            StatementKind::TypeAlias { name, alias } => {
                self.process_type_expression(alias);
                self.insert_symbol(
                    name.clone(),
                    SymbolKind::TypeAlias,
                    Some(statement.clone()),
                    Some(alias.clone()),
                    statement.span,
                );
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

    fn process_expression(&mut self, expression: &Expression) {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                let current_scope = self.current_scope();
                self.table.lookup(name, SymbolNamespace::Value, current_scope);
            }
            ExpressionKind::Unary { operand, .. } => self.process_expression(operand),
            ExpressionKind::Binary { left, right, .. } => {
                self.process_expression(left);
                self.process_expression(right);
            }
            ExpressionKind::Call { callee, arguments } => {
                self.process_expression(callee);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        TableField::Named { value, .. } => self.process_expression(value),
                        TableField::Indexed { key, value } => {
                            self.process_expression(key);
                            self.process_expression(value);
                        }
                        TableField::Expression(expr) => self.process_expression(expr),
                    }
                }
            }
            ExpressionKind::MemberAccess { object, .. } => self.process_expression(object),
            ExpressionKind::Index { object, index } => {
                self.process_expression(object);
                self.process_expression(index);
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.process_expression(receiver);
                for argument in arguments {
                    self.process_expression(argument);
                }
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let crate::parser::ast_builder::InterpolatedStringPart::Expression(expr) = part {
                        self.process_expression(expr);
                    }
                }
            }
            ExpressionKind::NumberLiteral(_) | ExpressionKind::StringLiteral(_) | ExpressionKind::BooleanLiteral(_) | ExpressionKind::Nil | ExpressionKind::Error => {}
        }
    }

    fn process_type_expression(&mut self, type_expression: &TypeExpression) {
        match &type_expression.kind {
            TypeExpressionKind::Named(name) => {
                self.table.lookup(name, SymbolNamespace::Type, self.current_scope());
            }
            TypeExpressionKind::Optional(inner)
            | TypeExpressionKind::Array(inner)
            | TypeExpressionKind::Variadic(inner)
            | TypeExpressionKind::Parenthesized(inner) => {
                self.process_type_expression(inner);
            }
            TypeExpressionKind::Union(types) | TypeExpressionKind::Intersection(types) | TypeExpressionKind::Tuple(types) => {
                for typ in types {
                    self.process_type_expression(typ);
                }
            }
            TypeExpressionKind::Function { params, return_type } => {
                for param in params {
                    self.process_type_expression(param);
                }
                self.process_type_expression(return_type);
            }
            TypeExpressionKind::Table(fields) => {
                for (_, field_type, _) in fields {
                    self.process_type_expression(field_type);
                }
            }
        }
    }
}
