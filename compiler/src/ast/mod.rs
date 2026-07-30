#[derive(Debug, Default)]
pub struct AstStage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Error(ErrorExpression),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Error(ErrorStatement),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorExpression;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorStatement;
