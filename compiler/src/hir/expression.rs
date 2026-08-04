use super::ids::{HirSymbolId, HirVariableId};
use super::types::{HirBinaryOperator, HirBuiltinFunction, HirType, HirUnaryOperator};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct HirExpression {
    pub kind: HirExpressionKind,
    pub expr_type: Option<HirType>,
    pub symbol_id: Option<HirSymbolId>,
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

    // Closures - use a placeholder to avoid circular dependency
    // The actual function will be stored separately
    ClosurePlaceholder,

    // Built-in functions
    BuiltinCall {
        function: HirBuiltinFunction,
        arguments: Vec<HirExpression>,
    },

    // Error recovery
    Error,
}

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
