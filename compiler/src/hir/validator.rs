use std::collections::{HashMap, HashSet};

use super::expression::{
    HirExpression, HirExpressionKind, HirInterpolatedStringPart, HirTableField,
};
use super::function::HirFunction;
use super::ids::{HirScopeId, HirSymbolId};
use super::module::{HirGlobalVariable, HirModule, HirTypeAlias};
use super::statement::{HirStatement, HirStatementKind};
use super::symbol::HirSymbolKind;
use super::types::{HirBinaryOperator, HirFunctionSignature, HirType, HirUnaryOperator};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct HirValidator {
    errors: Vec<HirValidationError>,
    known_symbols: HashSet<HirSymbolId>,
    known_scopes: HashSet<HirScopeId>,
    symbol_kinds: HashMap<HirSymbolId, HirSymbolKind>,
    symbol_signatures: HashMap<HirSymbolId, HirFunctionSignature>,
    symbol_owners: HashMap<HirSymbolId, HirScopeId>,
    scope_parents: HashMap<HirScopeId, Option<HirScopeId>>,
    current_return_type: Option<HirType>,
    function_depth: usize,
    loop_depth: usize,
}

#[derive(Debug, Clone)]
pub enum HirValidationError {
    InvalidControlFlow { message: String, span: SourceSpan },
    InvalidExpression { message: String, span: SourceSpan },
    InvalidFunction { message: String, span: SourceSpan },
    InvalidModule { message: String, span: SourceSpan },
    InvalidScope { message: String, span: SourceSpan },
    InvalidSymbol { message: String, span: SourceSpan },
    InvalidType { message: String, span: SourceSpan },
}

impl HirValidator {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            known_symbols: HashSet::new(),
            known_scopes: HashSet::new(),
            symbol_kinds: HashMap::new(),
            symbol_signatures: HashMap::new(),
            symbol_owners: HashMap::new(),
            scope_parents: HashMap::new(),
            current_return_type: None,
            function_depth: 0,
            loop_depth: 0,
        }
    }

    pub fn validate(&mut self, module: &HirModule) -> Result<(), Vec<HirValidationError>> {
        self.reset();
        self.validate_module(module);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn report(&mut self, module: &HirModule) -> String {
        match self.validate(module) {
            Ok(()) => [
                "HIR Validation",
                "✔ Symbols",
                "✔ Scopes",
                "✔ Types",
                "✔ Functions",
                "✔ Statements",
                "✔ Expressions",
                "✔ Control Flow",
                "Validation Passed",
            ]
            .join("\n"),
            Err(errors) => {
                let mut report =
                    format!("HIR Validation\nValidation Failed: {} errors", errors.len());
                for error in errors {
                    report.push('\n');
                    report.push_str(&format!("{:?}", error));
                }
                report
            }
        }
    }

    fn reset(&mut self) {
        self.errors.clear();
        self.known_symbols.clear();
        self.known_scopes.clear();
        self.symbol_kinds.clear();
        self.symbol_signatures.clear();
        self.symbol_owners.clear();
        self.scope_parents.clear();
        self.current_return_type = None;
        self.function_depth = 0;
        self.loop_depth = 0;
    }

    fn validate_module(&mut self, module: &HirModule) {
        self.collect_module_indexes(module);

        if let Some(root_scope) = module.metadata.root_scope {
            self.validate_scope_id(root_scope, module.span, "module root scope");
            if let Some(scope) = module.scopes.iter().find(|scope| scope.id == root_scope) {
                if scope.parent.is_some() {
                    self.errors.push(HirValidationError::InvalidScope {
                        message: "Root scope must not have a parent".to_string(),
                        span: scope.span,
                    });
                }
            }
            self.validate_scope_tree_has_no_cycles(module, root_scope);
        } else {
            self.errors.push(HirValidationError::InvalidModule {
                message: "Module is missing a root scope".to_string(),
                span: module.span,
            });
        }

        for scope in &module.scopes {
            if let Some(parent) = scope.parent {
                self.validate_scope_id(parent, scope.span, "scope parent");
                if let Some(parent_scope) = module
                    .scopes
                    .iter()
                    .find(|candidate| candidate.id == parent)
                {
                    if !parent_scope.children.contains(&scope.id) {
                        self.errors.push(HirValidationError::InvalidScope {
                            message: format!(
                                "Scope #{} parent #{} does not list it as a child",
                                scope.id.0, parent.0
                            ),
                            span: scope.span,
                        });
                    }
                }
            }
            for child in &scope.children {
                self.validate_scope_id(*child, scope.span, "scope child");
                if self.scope_parents.get(child).copied().flatten() != Some(scope.id) {
                    self.errors.push(HirValidationError::InvalidScope {
                        message: format!(
                            "Scope #{} lists child #{} but child parent does not point back",
                            scope.id.0, child.0
                        ),
                        span: scope.span,
                    });
                }
            }
            for symbol_id in &scope.symbols {
                self.validate_symbol_id(*symbol_id, scope.span, "scope symbol");
                if let Some(symbol) = module.symbols.iter().find(|symbol| symbol.id == *symbol_id) {
                    if symbol.scope_id != scope.id {
                        self.errors.push(HirValidationError::InvalidSymbol {
                            message: format!(
                                "Symbol #{} is listed in scope #{} but declares scope #{}",
                                symbol.id.0, scope.id.0, symbol.scope_id.0
                            ),
                            span: symbol.span,
                        });
                    }
                }
            }
        }

        for symbol in &module.symbols {
            self.validate_scope_id(symbol.scope_id, symbol.span, "symbol scope");
            if self.symbol_owners.get(&symbol.id).copied() != Some(symbol.scope_id) {
                self.errors.push(HirValidationError::InvalidSymbol {
                    message: format!(
                        "Symbol #{} does not have exactly one matching scope owner",
                        symbol.id.0
                    ),
                    span: symbol.span,
                });
            }
            match symbol.kind {
                HirSymbolKind::Function
                | HirSymbolKind::BuiltinFunction
                | HirSymbolKind::NativeFunction => {
                    if symbol.function_signature.is_none() {
                        self.errors.push(HirValidationError::InvalidFunction {
                            message: format!(
                                "Function symbol '{}' is missing a signature",
                                symbol.name
                            ),
                            span: symbol.span,
                        });
                    }
                }
                _ => {}
            }
        }

        self.validate_required_builtins(module);

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

    fn collect_module_indexes(&mut self, module: &HirModule) {
        let mut seen_scopes = HashSet::new();
        for scope in &module.scopes {
            if !seen_scopes.insert(scope.id) {
                self.errors.push(HirValidationError::InvalidScope {
                    message: format!("Duplicate scope ID #{}", scope.id.0),
                    span: scope.span,
                });
            }
            self.known_scopes.insert(scope.id);
            self.scope_parents.insert(scope.id, scope.parent);
            for symbol_id in &scope.symbols {
                if self.symbol_owners.insert(*symbol_id, scope.id).is_some() {
                    self.errors.push(HirValidationError::InvalidSymbol {
                        message: format!("Symbol #{} is owned by multiple scopes", symbol_id.0),
                        span: scope.span,
                    });
                }
            }
        }

        let mut seen_symbols = HashSet::new();
        for symbol in &module.symbols {
            if !seen_symbols.insert(symbol.id) {
                self.errors.push(HirValidationError::InvalidSymbol {
                    message: format!("Duplicate symbol ID #{}", symbol.id.0),
                    span: symbol.span,
                });
            }
            self.known_symbols.insert(symbol.id);
            self.symbol_kinds.insert(symbol.id, symbol.kind.clone());
            if let Some(signature) = &symbol.function_signature {
                self.symbol_signatures.insert(symbol.id, signature.clone());
            }
        }
    }

    fn validate_scope_tree_has_no_cycles(&mut self, module: &HirModule, root_scope: HirScopeId) {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        self.visit_scope_tree(module, root_scope, &mut visiting, &mut visited);

        for scope in &module.scopes {
            let mut chain = HashSet::new();
            let mut cursor = Some(scope.id);
            while let Some(scope_id) = cursor {
                if !chain.insert(scope_id) {
                    self.errors.push(HirValidationError::InvalidScope {
                        message: format!("Scope parent cycle detected at #{}", scope_id.0),
                        span: scope.span,
                    });
                    break;
                }
                cursor = self.scope_parents.get(&scope_id).copied().flatten();
            }
        }
    }

    fn visit_scope_tree(
        &mut self,
        module: &HirModule,
        scope_id: HirScopeId,
        visiting: &mut HashSet<HirScopeId>,
        visited: &mut HashSet<HirScopeId>,
    ) {
        if visited.contains(&scope_id) {
            return;
        }

        if !visiting.insert(scope_id) {
            let span = module
                .scopes
                .iter()
                .find(|scope| scope.id == scope_id)
                .map(|scope| scope.span)
                .unwrap_or(module.span);
            self.errors.push(HirValidationError::InvalidScope {
                message: format!("Scope child cycle detected at #{}", scope_id.0),
                span,
            });
            return;
        }

        if let Some(scope) = module.scopes.iter().find(|scope| scope.id == scope_id) {
            for child in &scope.children {
                self.visit_scope_tree(module, *child, visiting, visited);
            }
        }

        visiting.remove(&scope_id);
        visited.insert(scope_id);
    }

    fn validate_required_builtins(&mut self, module: &HirModule) {
        for builtin_name in [
            "print", "type", "tonumber", "tostring", "error", "pairs", "ipairs", "require",
        ] {
            let valid_builtin = module.symbols.iter().any(|symbol| {
                symbol.name == builtin_name && symbol.kind == HirSymbolKind::BuiltinFunction
            });
            if !valid_builtin {
                self.errors.push(HirValidationError::InvalidModule {
                    message: format!("Missing built-in symbol '{builtin_name}'"),
                    span: module.span,
                });
            }
        }
    }

    fn validate_function(&mut self, function: &HirFunction) {
        self.validate_symbol_id(function.symbol_id, function.span, "function symbol");
        self.validate_scope_id(function.scope_id, function.span, "function scope");
        if !matches!(
            self.symbol_kinds.get(&function.symbol_id),
            Some(HirSymbolKind::Function)
        ) {
            self.errors.push(HirValidationError::InvalidFunction {
                message: format!(
                    "Function '{}' symbol #{} is not a function symbol",
                    function.name, function.symbol_id.0
                ),
                span: function.span,
            });
        }
        if let Some(signature) = self.symbol_signatures.get(&function.symbol_id) {
            if signature != &function.signature {
                self.errors.push(HirValidationError::InvalidFunction {
                    message: format!(
                        "Function '{}' signature does not match its symbol",
                        function.name
                    ),
                    span: function.span,
                });
            }
        }
        if function.signature.parameter_types.len() != function.parameters.len() {
            self.errors.push(HirValidationError::InvalidFunction {
                message: format!(
                    "Function '{}' signature parameter count does not match parameters",
                    function.name
                ),
                span: function.span,
            });
        }
        if let Some(return_type) = &function.return_type {
            if function.signature.return_type != *return_type {
                self.errors.push(HirValidationError::InvalidFunction {
                    message: format!(
                        "Function '{}' signature return type does not match return type",
                        function.name
                    ),
                    span: function.span,
                });
            }
        }

        for parameter in &function.parameters {
            self.validate_symbol_id(parameter.symbol_id, parameter.span, "parameter symbol");
            self.validate_scope_id(parameter.scope_id, parameter.span, "parameter scope");
            if parameter.param_type.is_none() {
                self.errors.push(HirValidationError::InvalidExpression {
                    message: format!("Parameter '{}' is missing a type", parameter.name),
                    span: parameter.span,
                });
            }
        }

        for variable in &function.local_variables {
            self.validate_symbol_id(variable.symbol_id, variable.span, "local variable symbol");
            self.validate_scope_id(variable.scope_id, variable.span, "local variable scope");
            if variable.var_type.is_none() {
                self.errors.push(HirValidationError::InvalidExpression {
                    message: format!("Local variable '{}' is missing a type", variable.name),
                    span: variable.span,
                });
            }
        }

        if function.return_type.is_none() {
            self.errors.push(HirValidationError::InvalidExpression {
                message: format!("Function '{}' is missing a return type", function.name),
                span: function.span,
            });
        }

        let previous_return_type = self.current_return_type.clone();
        let previous_loop_depth = self.loop_depth;
        self.function_depth += 1;
        self.loop_depth = 0;
        self.current_return_type = function.return_type.clone();
        for statement in &function.body {
            self.validate_statement(statement);
        }
        self.loop_depth = previous_loop_depth;
        self.function_depth -= 1;
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
                if variable.var_type.is_none() {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: format!("Local variable '{}' is missing a type", variable.name),
                        span: variable.span,
                    });
                }
                if let Some(init) = initializer {
                    self.validate_expression(init);
                    if let (Some(expected), Some(actual)) =
                        (variable.var_type.as_ref(), init.expr_type.as_ref())
                    {
                        if !Self::types_compatible(expected, actual) {
                            self.errors.push(HirValidationError::InvalidExpression {
                                message: format!(
                                    "Variable type mismatch: expected {:?}, got {:?}",
                                    expected, actual
                                ),
                                span: init.span,
                            });
                        }
                    }
                }
            }
            HirStatementKind::Assignment { targets, values } => {
                for target in targets {
                    self.validate_expression(target);
                }
                for value in values {
                    self.validate_expression(value);
                }
                for (target, value) in targets.iter().zip(values.iter()) {
                    if let (Some(expected), Some(actual)) =
                        (target.expr_type.as_ref(), value.expr_type.as_ref())
                    {
                        if !Self::types_compatible(expected, actual) {
                            self.errors.push(HirValidationError::InvalidExpression {
                                message: format!(
                                    "Assignment type mismatch: expected {:?}, got {:?}",
                                    expected, actual
                                ),
                                span: value.span,
                            });
                        }
                    }
                }
            }
            HirStatementKind::Expression(expr) => {
                self.validate_expression(expr);
            }
            HirStatementKind::Return(exprs) => {
                if self.function_depth == 0 {
                    self.errors.push(HirValidationError::InvalidControlFlow {
                        message: "Return statement may only appear inside a function".to_string(),
                        span: statement.span,
                    });
                }
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
                self.loop_depth += 1;
                for stmt in body {
                    self.validate_statement(stmt);
                }
                self.loop_depth -= 1;
            }
            HirStatementKind::RepeatUntil { body, condition } => {
                self.loop_depth += 1;
                for stmt in body {
                    self.validate_statement(stmt);
                }
                self.loop_depth -= 1;
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
                self.loop_depth += 1;
                for stmt in body {
                    self.validate_statement(stmt);
                }
                self.loop_depth -= 1;
            }
            HirStatementKind::ForGeneric {
                iterables,
                body,
                variables: _,
            } => {
                for iterable in iterables {
                    self.validate_expression(iterable);
                }
                self.loop_depth += 1;
                for stmt in body {
                    self.validate_statement(stmt);
                }
                self.loop_depth -= 1;
            }
            HirStatementKind::Break => {
                if self.loop_depth == 0 {
                    self.errors.push(HirValidationError::InvalidControlFlow {
                        message: "'break' may only appear inside a loop".to_string(),
                        span: statement.span,
                    });
                }
            }
            HirStatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.errors.push(HirValidationError::InvalidControlFlow {
                        message: "'continue' may only appear inside a loop".to_string(),
                        span: statement.span,
                    });
                }
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

        if !matches!(expression.kind, HirExpressionKind::Error) && expression.expr_type.is_none() {
            self.errors.push(HirValidationError::InvalidExpression {
                message: "Expression is missing a type".to_string(),
                span: expression.span,
            });
        }

        match &expression.kind {
            HirExpressionKind::Unary { operand, operator } => {
                self.validate_expression(operand);
                self.validate_unary_expression(*operator, operand, expression);
            }
            HirExpressionKind::Binary {
                left,
                right,
                operator,
            } => {
                self.validate_expression(left);
                self.validate_expression(right);
                self.validate_binary_expression(*operator, left, right, expression);
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
                if !matches!(
                    callee.expr_type.as_ref(),
                    Some(HirType::Function | HirType::Unknown | HirType::Any)
                ) {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: "Function call callee is not a function".to_string(),
                        span: callee.span,
                    });
                }
                if let Some(symbol_id) = callee.symbol_id {
                    if let Some(signature) = self.symbol_signatures.get(&symbol_id).cloned() {
                        self.validate_call_arguments(&signature, arguments, expression.span);
                        if let Some(actual) = expression.expr_type.as_ref() {
                            if !Self::types_compatible(&signature.return_type, actual) {
                                self.errors.push(HirValidationError::InvalidType {
                                    message: format!(
                                        "Function call result type mismatch: expected {:?}, got {:?}",
                                        signature.return_type, actual
                                    ),
                                    span: expression.span,
                                });
                            }
                        }
                    }
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
                if let Some(symbol_id) = expression.symbol_id {
                    if let Some(signature) = self.symbol_signatures.get(&symbol_id).cloned() {
                        self.validate_call_arguments(&signature, arguments, expression.span);
                    }
                    if !matches!(
                        self.symbol_kinds.get(&symbol_id),
                        Some(HirSymbolKind::BuiltinFunction)
                    ) {
                        self.errors.push(HirValidationError::InvalidSymbol {
                            message: "Builtin call does not reference a builtin function symbol"
                                .to_string(),
                            span: expression.span,
                        });
                    }
                }
            }
            HirExpressionKind::Nil
            | HirExpressionKind::Boolean(_)
            | HirExpressionKind::Number(_)
            | HirExpressionKind::String(_) => {}
            HirExpressionKind::GlobalVariable(_) => {
                if expression.symbol_id.is_none() {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: "Global reference is missing a symbol".to_string(),
                        span: expression.span,
                    });
                } else if let Some(symbol_id) = expression.symbol_id {
                    if matches!(
                        self.symbol_kinds.get(&symbol_id),
                        Some(HirSymbolKind::Local | HirSymbolKind::Parameter)
                    ) {
                        self.errors.push(HirValidationError::InvalidSymbol {
                            message: "Global reference points at a local symbol".to_string(),
                            span: expression.span,
                        });
                    }
                }
            }
            HirExpressionKind::LocalVariable(_) => {
                if expression.symbol_id.is_none() {
                    self.errors.push(HirValidationError::InvalidExpression {
                        message: "Local variable reference is missing a symbol".to_string(),
                        span: expression.span,
                    });
                } else if let Some(symbol_id) = expression.symbol_id {
                    if !matches!(
                        self.symbol_kinds.get(&symbol_id),
                        Some(HirSymbolKind::Local | HirSymbolKind::Parameter)
                    ) {
                        self.errors.push(HirValidationError::InvalidSymbol {
                            message: "Local variable reference does not point at a local or parameter symbol".to_string(),
                            span: expression.span,
                        });
                    }
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

    fn validate_call_arguments(
        &mut self,
        signature: &HirFunctionSignature,
        arguments: &[HirExpression],
        span: SourceSpan,
    ) {
        if !signature.is_variadic && arguments.len() != signature.parameter_types.len() {
            self.errors.push(HirValidationError::InvalidFunction {
                message: format!(
                    "Call argument count mismatch: expected {}, got {}",
                    signature.parameter_types.len(),
                    arguments.len()
                ),
                span,
            });
        }

        for (expected, argument) in signature.parameter_types.iter().zip(arguments.iter()) {
            if let Some(actual) = argument.expr_type.as_ref() {
                if !Self::types_compatible(expected, actual) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: format!(
                            "Call argument type mismatch: expected {:?}, got {:?}",
                            expected, actual
                        ),
                        span: argument.span,
                    });
                }
            }
        }
    }

    fn validate_unary_expression(
        &mut self,
        operator: HirUnaryOperator,
        operand: &HirExpression,
        expression: &HirExpression,
    ) {
        match operator {
            HirUnaryOperator::Negate | HirUnaryOperator::BitwiseNot => {
                if !matches!(
                    operand.expr_type.as_ref(),
                    Some(HirType::Integer | HirType::Number | HirType::Any | HirType::Unknown)
                ) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: format!("Unary operator {:?} requires numeric operand", operator),
                        span: operand.span,
                    });
                }
            }
            HirUnaryOperator::Not => {
                if !matches!(expression.expr_type.as_ref(), Some(HirType::Boolean)) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: "Unary 'not' must produce boolean".to_string(),
                        span: expression.span,
                    });
                }
            }
            HirUnaryOperator::Length => {
                if !matches!(expression.expr_type.as_ref(), Some(HirType::Integer)) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: "Length operator must produce integer".to_string(),
                        span: expression.span,
                    });
                }
            }
        }
    }

    fn validate_binary_expression(
        &mut self,
        operator: HirBinaryOperator,
        left: &HirExpression,
        right: &HirExpression,
        expression: &HirExpression,
    ) {
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
            | HirBinaryOperator::BitwiseShiftRight => {
                for operand in [left, right] {
                    if !matches!(
                        operand.expr_type.as_ref(),
                        Some(HirType::Integer | HirType::Number | HirType::Any | HirType::Unknown)
                    ) {
                        self.errors.push(HirValidationError::InvalidType {
                            message: format!(
                                "Binary operator {:?} requires numeric operands",
                                operator
                            ),
                            span: operand.span,
                        });
                    }
                }
            }
            HirBinaryOperator::Concatenate => {
                if !matches!(
                    expression.expr_type.as_ref(),
                    Some(HirType::String | HirType::Unknown)
                ) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: "Concatenation must produce string or unknown".to_string(),
                        span: expression.span,
                    });
                }
            }
            HirBinaryOperator::Equal
            | HirBinaryOperator::NotEqual
            | HirBinaryOperator::LessThan
            | HirBinaryOperator::LessEqual
            | HirBinaryOperator::GreaterThan
            | HirBinaryOperator::GreaterEqual
            | HirBinaryOperator::And
            | HirBinaryOperator::Or => {
                if !matches!(expression.expr_type.as_ref(), Some(HirType::Boolean)) {
                    self.errors.push(HirValidationError::InvalidType {
                        message: "Comparison/logical expression must produce boolean".to_string(),
                        span: expression.span,
                    });
                }
            }
        }
    }

    fn types_compatible(expected: &HirType, actual: &HirType) -> bool {
        expected == actual
            || matches!(expected, HirType::Any | HirType::Unknown)
            || matches!(actual, HirType::Any | HirType::Unknown)
            || matches!((expected, actual), (HirType::Number, HirType::Integer))
    }
}

impl Default for HirValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::hir::{HirBuilder, HirPrinter};
    use crate::lexer::{Lexer, TokenKind};
    use crate::parser::ast_builder::AstNode;
    use crate::parser::Parser;
    use crate::source::SourceManager;

    use super::*;

    fn lower_source(source: &str) -> HirModule {
        let ast = parse_source(source);
        HirBuilder::new().build(&ast).unwrap()
    }

    fn parse_source(source: &str) -> AstNode {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("validator_test.glu"), source.to_string());
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

    fn validation_errors(module: &HirModule) -> Vec<HirValidationError> {
        HirValidator::new().validate(module).unwrap_err()
    }

    fn has_error(errors: &[HirValidationError], pattern: &str) -> bool {
        errors
            .iter()
            .any(|error| format!("{:?}", error).contains(pattern))
    }

    #[test]
    fn reports_successful_validation_report() {
        let module = lower_source("print(\"hello\")");
        let report = HirValidator::new().report(&module);

        assert!(report.contains("HIR Validation"));
        assert!(report.contains("✔ Symbols"));
        assert!(report.contains("Validation Passed"));
    }

    #[test]
    fn rejects_duplicate_symbol_ids() {
        let mut module = lower_source("print(\"hello\")");
        module.symbols[1].id = module.symbols[0].id;
        let errors = validation_errors(&module);

        assert!(has_error(&errors, "Duplicate symbol ID"));
    }

    #[test]
    fn rejects_scope_parent_child_mismatches() {
        let mut module = lower_source("function outer()\nend");
        let root_scope_id = module.metadata.root_scope.unwrap();
        let root_scope = module
            .scopes
            .iter_mut()
            .find(|scope| scope.id == root_scope_id)
            .unwrap();
        root_scope.children.clear();
        let errors = validation_errors(&module);

        assert!(has_error(&errors, "does not list it as a child"));
    }

    #[test]
    fn rejects_symbols_without_scope_owners() {
        let mut module = lower_source("print(\"hello\")");
        let symbol_id = module.symbols[0].id;
        for scope in &mut module.scopes {
            scope.symbols.retain(|candidate| *candidate != symbol_id);
        }
        let errors = validation_errors(&module);

        assert!(has_error(
            &errors,
            "does not have exactly one matching scope owner"
        ));
    }

    #[test]
    fn rejects_break_outside_loop() {
        let mut module = lower_source("print(\"hello\")");
        let span = module.span;
        let main = module
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap();
        main.body.push(HirStatement {
            kind: HirStatementKind::Break,
            span,
        });
        let errors = validation_errors(&module);

        assert!(has_error(&errors, "break"));
    }

    #[test]
    fn rejects_missing_expression_types() {
        let mut module = lower_source("print(\"hello\")");
        let main = module
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .unwrap();
        let HirStatementKind::Expression(expression) = &mut main.body[0].kind else {
            panic!("expected expression");
        };
        expression.expr_type = None;
        let errors = validation_errors(&module);

        assert!(has_error(&errors, "Expression is missing a type"));
    }

    #[test]
    fn scope_debug_output_includes_tree_metadata() {
        let module = lower_source("function outer()\nfunction inner()\nend\nend");
        let output = HirPrinter::new().print_module(&module);

        assert!(output.contains("Scope #0 Global"));
        assert!(output.contains("children="));
        assert!(output.contains("symbols="));
    }
}
