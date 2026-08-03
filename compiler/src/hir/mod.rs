use crate::parser::ast_builder::AstNode;
use crate::source::SourceSpan;
use std::fmt;

// ============================================================================
// HIR Module - Top-level container
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirModule {
    pub name: String,
    pub functions: Vec<HirFunction>,
    pub global_variables: Vec<HirGlobalVariable>,
    pub span: SourceSpan,
}

impl HirModule {
    pub fn new(name: String, span: SourceSpan) -> Self {
        Self {
            name,
            functions: Vec::new(),
            global_variables: Vec::new(),
            span,
        }
    }
}

// ============================================================================
// HIR Function - Function definitions
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub id: HirFunctionId,
    pub name: String,
    pub parameters: Vec<HirParameter>,
    pub body: Vec<HirStatement>,
    pub return_type: Option<HirType>,
    pub is_local: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirFunctionId(usize);

impl HirFunctionId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

// ============================================================================
// HIR Parameter - Function parameters
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirParameter {
    pub name: String,
    pub param_type: Option<HirType>,
    pub span: SourceSpan,
}

// ============================================================================
// HIR Global Variable - Module-level variables
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirGlobalVariable {
    pub name: String,
    pub var_type: Option<HirType>,
    pub initializer: Option<HirExpression>,
    pub span: SourceSpan,
}

// ============================================================================
// HIR Local Variable - Function-level variables
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirLocalVariable {
    pub id: HirVariableId,
    pub name: String,
    pub var_type: Option<HirType>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirVariableId(usize);

impl HirVariableId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

// ============================================================================
// HIR Statement - All statement types
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirStatement {
    pub kind: HirStatementKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum HirStatementKind {
    // Variable declaration
    LocalVariable {
        variable: HirLocalVariable,
        initializer: Option<HirExpression>,
    },
    
    // Assignment
    Assignment {
        targets: Vec<HirExpression>,
        values: Vec<HirExpression>,
    },
    
    // Function call
    Expression(HirExpression),
    
    // Return
    Return(Option<Vec<HirExpression>>),
    
    // Control flow
    Block(Vec<HirStatement>),
    If {
        condition: HirExpression,
        then_block: Vec<HirStatement>,
        else_block: Option<Vec<HirStatement>>,
    },
    While {
        condition: HirExpression,
        body: Vec<HirStatement>,
    },
    RepeatUntil {
        body: Vec<HirStatement>,
        condition: HirExpression,
    },
    ForNumeric {
        variable: HirLocalVariable,
        start: HirExpression,
        end: HirExpression,
        step: Option<HirExpression>,
        body: Vec<HirStatement>,
    },
    ForGeneric {
        variables: Vec<HirLocalVariable>,
        iterables: Vec<HirExpression>,
        body: Vec<HirStatement>,
    },
    
    // Loop control
    Break,
    Continue,
    
    // Function definition
    Function {
        function: HirFunction,
    },
    
    // Error recovery
    Error,
}

// ============================================================================
// HIR Expression - All expression types
// ============================================================================

#[derive(Debug, Clone)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub expr_type: Option<HirType>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum HirExpressionKind {
    // Literals
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    
    // Variables
    LocalVariable(HirVariableId),
    GlobalVariable(String),
    
    // Operators
    Unary {
        operator: HirUnaryOperator,
        operand: Box<HirExpression>,
    },
    Binary {
        left: Box<HirExpression>,
        operator: HirBinaryOperator,
        right: Box<HirExpression>,
    },
    
    // Tables
    TableConstructor(Vec<HirTableField>),
    
    // Table access
    Index {
        object: Box<HirExpression>,
        index: Box<HirExpression>,
    },
    FieldAccess {
        object: Box<HirExpression>,
        field: String,
    },
    
    // Functions
    FunctionCall {
        callee: Box<HirExpression>,
        arguments: Vec<HirExpression>,
    },
    MethodCall {
        receiver: Box<HirExpression>,
        method: String,
        arguments: Vec<HirExpression>,
    },
    
    // Closures
    Closure(Box<HirFunction>),
    
    // Built-in functions
    BuiltinCall {
        function: HirBuiltinFunction,
        arguments: Vec<HirExpression>,
    },
    
    // Error recovery
    Error,
}

// ============================================================================
// HIR Operators
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOperator {
    Negate,      // -
    Not,         // not
    Length,      // #
    BitwiseNot,  // ~
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinaryOperator {
    // Arithmetic
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    FloorDivide,  // //
    Modulo,       // %
    Exponent,     // ^
    
    // Comparison
    Equal,        // ==
    NotEqual,     // ~=
    LessThan,     // <
    LessEqual,    // <=
    GreaterThan,  // >
    GreaterEqual, // >=
    
    // Logical
    And,          // and
    Or,           // or
    
    // String/Concatenation
    Concatenate,  // ..
    
    // Bitwise
    BitwiseAnd,   // &
    BitwiseOr,    // |
    BitwiseXor,   // ^
    BitwiseShiftLeft,   // <<
    BitwiseShiftRight,  // >>
}

// ============================================================================
// HIR Table Fields
// ============================================================================

#[derive(Debug, Clone)]
pub enum HirTableField {
    Named {
        key: String,
        value: HirExpression,
    },
    Indexed {
        key: HirExpression,
        value: HirExpression,
    },
    Expression(HirExpression),
}

// ============================================================================
// HIR Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirType {
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function,
    Any,
    Unknown,
}

// ============================================================================
// HIR Built-in Functions
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirBuiltinFunction {
    Print,
    Type,
    ToNumber,
    ToString,
    Error,
    Pairs,
    Ipairs,
    Require,
}

// ============================================================================
// HIR Stage - Main entry point
// ============================================================================

#[derive(Debug, Default)]
pub struct HirStage;

impl HirStage {
    pub fn lower(ast: &AstNode) -> Result<HirModule, HirError> {
        let mut builder = HirBuilder::new();
        let module = builder.build(ast)?;
        
        // Validate the generated HIR
        let mut validator = HirValidator::new();
        if let Err(validation_errors) = validator.validate(&module) {
            return Err(HirError::LoweringError(format!(
                "HIR validation failed with {} errors",
                validation_errors.len()
            )));
        }
        
        Ok(module)
    }
}

// ============================================================================
// HIR Builder - AST to HIR lowering
// ============================================================================

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
        
        for statement in &program.statements {
            self.lower_statement_to_module(statement, &mut module)?;
        }
        
        Ok(module)
    }
    
    fn lower_statement_to_module(&mut self, stmt: &crate::parser::ast_builder::Statement, module: &mut HirModule) -> Result<(), HirError> {
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
            crate::parser::ast_builder::StatementKind::Local { names, initializers } => {
                if !names.is_empty() {
                    let global = HirGlobalVariable {
                        name: names[0].0.clone(),
                        var_type: None,
                        initializer: initializers.first().map(|expr| self.lower_expression(expr)),
                        span: stmt.span,
                    };
                    module.global_variables.push(global);
                }
            }
            _ => {
                // Skip other statements at module level for now
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

// ============================================================================
// HIR Error
// ============================================================================

#[derive(Debug, Clone)]
pub enum HirError {
    InvalidInput(String),
    LoweringError(String),
}

impl fmt::Display for HirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            HirError::LoweringError(msg) => write!(f, "Lowering error: {}", msg),
        }
    }
}

impl std::error::Error for HirError {}

// ============================================================================
// HIR Validation
// ============================================================================

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
            HirExpressionKind::Closure(function) => {
                self.validate_function(function);
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

// ============================================================================
// HIR Printer - Debug output
// ============================================================================

pub struct HirPrinter {
    indent: usize,
}

impl HirPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }
    
    pub fn print_module(&mut self, module: &HirModule) -> String {
        let mut output = String::new();
        
        output.push_str(&format!("Module '{}'\n", module.name));
        self.indent += 2;
        
        // Print global variables
        if !module.global_variables.is_empty() {
            output.push_str(&self.indent_str());
            output.push_str("Global Variables:\n");
            self.indent += 2;
            for global in &module.global_variables {
                output.push_str(&self.print_global_variable(global));
            }
            self.indent -= 2;
        }
        
        // Print functions
        if !module.functions.is_empty() {
            output.push_str(&self.indent_str());
            output.push_str("Functions:\n");
            self.indent += 2;
            for function in &module.functions {
                output.push_str(&self.print_function(function));
            }
            self.indent -= 2;
        }
        
        self.indent -= 2;
        output
    }
    
    fn print_global_variable(&mut self, global: &HirGlobalVariable) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        output.push_str(&format!("Global Variable: {}\n", global.name));
        
        if let Some(initializer) = &global.initializer {
            self.indent += 2;
            output.push_str(&self.indent_str());
            output.push_str("Initializer: ");
            output.push_str(&self.print_expression(initializer));
            output.push('\n');
            self.indent -= 2;
        }
        
        output
    }
    
    fn print_function(&mut self, function: &HirFunction) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        output.push_str(&format!("Function '{}'", function.name));
        
        if !function.parameters.is_empty() {
            output.push_str("(");
            let params: Vec<String> = function.parameters.iter()
                .map(|p| {
                    if let Some(param_type) = &p.param_type {
                        format!("{}: {:?}", p.name, param_type)
                    } else {
                        p.name.clone()
                    }
                })
                .collect();
            output.push_str(&params.join(", "));
            output.push(')');
        }
        
        if let Some(return_type) = &function.return_type {
            output.push_str(&format!(" -> {:?}", return_type));
        }
        
        output.push('\n');
        
        self.indent += 2;
        for statement in &function.body {
            output.push_str(&self.print_statement(statement));
        }
        self.indent -= 2;
        
        output
    }
    
    fn print_statement(&mut self, statement: &HirStatement) -> String {
        let mut output = String::new();
        output.push_str(&self.indent_str());
        
        match &statement.kind {
            HirStatementKind::LocalVariable { variable, initializer } => {
                output.push_str(&format!("Local Variable: {}", variable.name));
                if let Some(init) = initializer {
                    output.push_str(" = ");
                    output.push_str(&self.print_expression(init));
                }
                output.push('\n');
            }
            HirStatementKind::Assignment { targets, values } => {
                output.push_str("Assignment: ");
                let target_strs: Vec<String> = targets.iter()
                    .map(|t| self.print_expression(t))
                    .collect();
                output.push_str(&target_strs.join(", "));
                output.push_str(" = ");
                let value_strs: Vec<String> = values.iter()
                    .map(|v| self.print_expression(v))
                    .collect();
                output.push_str(&value_strs.join(", "));
                output.push('\n');
            }
            HirStatementKind::Expression(expr) => {
                output.push_str(&self.print_expression(expr));
                output.push('\n');
            }
            HirStatementKind::Return(exprs) => {
                output.push_str("Return");
                if let Some(exprs) = exprs {
                    output.push_str(" ");
                    let expr_strs: Vec<String> = exprs.iter()
                        .map(|e| self.print_expression(e))
                        .collect();
                    output.push_str(&expr_strs.join(", "));
                }
                output.push('\n');
            }
            HirStatementKind::Block(statements) => {
                output.push_str("Block\n");
                self.indent += 2;
                for stmt in statements {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::If { condition, then_block, else_block } => {
                output.push_str("If ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
                self.indent += 2;
                output.push_str(&self.indent_str());
                output.push_str("Then:\n");
                self.indent += 2;
                for stmt in then_block {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
                
                if let Some(else_block) = else_block {
                    output.push_str(&self.indent_str());
                    output.push_str("Else:\n");
                    self.indent += 2;
                    for stmt in else_block {
                        output.push_str(&self.print_statement(stmt));
                    }
                    self.indent -= 2;
                }
                self.indent -= 2;
            }
            HirStatementKind::While { condition, body } => {
                output.push_str("While ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::RepeatUntil { body, condition } => {
                output.push_str("Repeat\n");
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
                output.push_str(&self.indent_str());
                output.push_str("Until ");
                output.push_str(&self.print_expression(condition));
                output.push('\n');
            }
            HirStatementKind::ForNumeric { variable, start, end, step, body } => {
                output.push_str(&format!("For {} = ", variable.name));
                output.push_str(&self.print_expression(start));
                output.push_str(" to ");
                output.push_str(&self.print_expression(end));
                if let Some(step) = step {
                    output.push_str(" step ");
                    output.push_str(&self.print_expression(step));
                }
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::ForGeneric { variables, iterables, body } => {
                output.push_str("For ");
                let var_names: Vec<String> = variables.iter().map(|v| v.name.clone()).collect();
                output.push_str(&var_names.join(", "));
                output.push_str(" in ");
                let iterable_strs: Vec<String> = iterables.iter()
                    .map(|i| self.print_expression(i))
                    .collect();
                output.push_str(&iterable_strs.join(", "));
                output.push('\n');
                self.indent += 2;
                for stmt in body {
                    output.push_str(&self.print_statement(stmt));
                }
                self.indent -= 2;
            }
            HirStatementKind::Break => {
                output.push_str("Break\n");
            }
            HirStatementKind::Continue => {
                output.push_str("Continue\n");
            }
            HirStatementKind::Function { function } => {
                output.push_str(&self.print_function(function));
            }
            HirStatementKind::Error => {
                output.push_str("<Error>\n");
            }
        }
        
        output
    }
    
    fn print_expression(&mut self, expr: &HirExpression) -> String {
        match &expr.kind {
            HirExpressionKind::Nil => "nil".to_string(),
            HirExpressionKind::Boolean(b) => b.to_string(),
            HirExpressionKind::Number(n) => n.to_string(),
            HirExpressionKind::String(s) => format!("\"{}\"", s),
            HirExpressionKind::LocalVariable(id) => format!("local_{}", id.0),
            HirExpressionKind::GlobalVariable(name) => name.clone(),
            HirExpressionKind::Unary { operator, operand } => {
                let op_str = self.print_unary_operator(*operator);
                format!("{}({})", op_str, self.print_expression(operand))
            }
            HirExpressionKind::Binary { left, operator, right } => {
                let op_str = self.print_binary_operator(*operator);
                format!("({} {} {})", self.print_expression(left), op_str, self.print_expression(right))
            }
            HirExpressionKind::TableConstructor(fields) => {
                let mut field_strs = Vec::new();
                for field in fields {
                    match field {
                        HirTableField::Named { key, value } => {
                            field_strs.push(format!("{} = {}", key, self.print_expression(value)));
                        }
                        HirTableField::Indexed { key, value } => {
                            field_strs.push(format!("[{}] = {}", self.print_expression(key), self.print_expression(value)));
                        }
                        HirTableField::Expression(expr) => {
                            field_strs.push(self.print_expression(expr));
                        }
                    }
                }
                format!("{{{}}}", field_strs.join(", "))
            }
            HirExpressionKind::Index { object, index } => {
                format!("{}[{}]", self.print_expression(object), self.print_expression(index))
            }
            HirExpressionKind::FieldAccess { object, field } => {
                format!("{}.{}", self.print_expression(object), field)
            }
            HirExpressionKind::FunctionCall { callee, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("{}({})", self.print_expression(callee), arg_strs.join(", "))
            }
            HirExpressionKind::MethodCall { receiver, method, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("{}:{}({})", self.print_expression(receiver), method, arg_strs.join(", "))
            }
            HirExpressionKind::Closure(_) => "<closure>".to_string(),
            HirExpressionKind::BuiltinCall { function, arguments } => {
                let arg_strs: Vec<String> = arguments.iter()
                    .map(|a| self.print_expression(a))
                    .collect();
                format!("<builtin {:?}>({})", function, arg_strs.join(", "))
            }
            HirExpressionKind::Error => "<error>".to_string(),
        }
    }
    
    fn print_unary_operator(&self, op: HirUnaryOperator) -> &'static str {
        match op {
            HirUnaryOperator::Negate => "-",
            HirUnaryOperator::Not => "not",
            HirUnaryOperator::Length => "#",
            HirUnaryOperator::BitwiseNot => "~",
        }
    }
    
    fn print_binary_operator(&self, op: HirBinaryOperator) -> &'static str {
        match op {
            HirBinaryOperator::Add => "+",
            HirBinaryOperator::Subtract => "-",
            HirBinaryOperator::Multiply => "*",
            HirBinaryOperator::Divide => "/",
            HirBinaryOperator::FloorDivide => "//",
            HirBinaryOperator::Modulo => "%",
            HirBinaryOperator::Exponent => "^",
            HirBinaryOperator::Equal => "==",
            HirBinaryOperator::NotEqual => "~=",
            HirBinaryOperator::LessThan => "<",
            HirBinaryOperator::LessEqual => "<=",
            HirBinaryOperator::GreaterThan => ">",
            HirBinaryOperator::GreaterEqual => ">=",
            HirBinaryOperator::And => "and",
            HirBinaryOperator::Or => "or",
            HirBinaryOperator::Concatenate => "..",
            HirBinaryOperator::BitwiseAnd => "&",
            HirBinaryOperator::BitwiseOr => "|",
            HirBinaryOperator::BitwiseXor => "~",
            HirBinaryOperator::BitwiseShiftLeft => "<<",
            HirBinaryOperator::BitwiseShiftRight => ">>",
        }
    }
    
    fn indent_str(&self) -> String {
        " ".repeat(self.indent)
    }
}

impl Default for HirPrinter {
    fn default() -> Self {
        Self::new()
    }
}
