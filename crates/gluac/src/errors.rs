use std::fmt::{Display, Formatter};

pub type CompilerResult<T> = Result<T, CompilerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub stage: CompilerStage,
    pub message: String,
}

impl CompilerError {
    pub fn new(stage: CompilerStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

impl Display for CompilerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} error: {}", self.stage, self.message)
    }
}

impl std::error::Error for CompilerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    Lexing,
    Parsing,
    Semantic,
    Codegen,
}

impl Display for CompilerStage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexing => formatter.write_str("lexing"),
            Self::Parsing => formatter.write_str("parsing"),
            Self::Semantic => formatter.write_str("semantic"),
            Self::Codegen => formatter.write_str("codegen"),
        }
    }
}
