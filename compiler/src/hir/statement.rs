use super::expression::HirExpression;
use super::function::HirFunction;
use super::ids::{HirScopeId, HirSymbolId, HirVariableId};
use super::types::HirType;
use crate::source::SourceSpan;

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

#[derive(Debug, Clone)]
pub struct HirLocalVariable {
    pub id: HirVariableId,
    pub symbol_id: HirSymbolId,
    pub name: String,
    pub var_type: Option<HirType>,
    pub scope_id: HirScopeId,
    pub span: SourceSpan,
}
