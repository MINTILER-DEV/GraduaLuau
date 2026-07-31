use crate::source::SourceSpan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    Empty,
    Expression(Expression),
    Return(Option<Expression>),
    Local {
        name: String,
        initializer: Option<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionKind {
    Identifier(String),
    NumberLiteral(String),
    StringLiteral(String),
    BooleanLiteral(bool),
    Nil,
    Unary {
        operator: String,
        operand: Box<Expression>,
    },
    Binary {
        left: Box<Expression>,
        operator: String,
        right: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstNode {
    Program(Program),
    Statement(Statement),
    Expression(Expression),
    Error,
}

pub fn make_program(statements: Vec<Statement>, span: SourceSpan) -> AstNode {
    AstNode::Program(Program { statements, span })
}
