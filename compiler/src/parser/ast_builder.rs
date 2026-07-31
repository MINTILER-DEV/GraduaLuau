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
    Return(Option<Vec<Expression>>),
    Break,
    Continue,
    Local {
        names: Vec<(String, Option<TypeExpression>)>,
        initializers: Vec<Expression>,
    },
    Assignment {
        targets: Vec<Expression>,
        values: Vec<Expression>,
        operator: String,
    },
    Function {
        name: String,
        receiver: Option<String>,
        params: Vec<(String, Option<TypeExpression>)>,
        return_type: Option<TypeExpression>,
        body: Vec<Statement>,
        is_local: bool,
    },
    TypeAlias {
        name: String,
        alias: TypeExpression,
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
pub struct TypeExpression {
    pub kind: TypeExpressionKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpressionKind {
    Named(String),
    Optional(Box<TypeExpression>),
    Union(Vec<TypeExpression>),
    Intersection(Vec<TypeExpression>),
    Table(Vec<(String, TypeExpression, SourceSpan)>),
    Array(Box<TypeExpression>),
    Function {
        params: Vec<TypeExpression>,
        return_type: Box<TypeExpression>,
    },
    Tuple(Vec<TypeExpression>),
    Variadic(Box<TypeExpression>),
    Parenthesized(Box<TypeExpression>),
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
