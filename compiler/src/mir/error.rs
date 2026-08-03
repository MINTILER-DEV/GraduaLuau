use std::fmt;

#[derive(Debug, Clone)]
pub enum MirError {
    InvalidInput(String),
    LoweringError(String),
    ValidationError(String),
}

impl fmt::Display for MirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            MirError::LoweringError(msg) => write!(f, "Lowering error: {}", msg),
            MirError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for MirError {}