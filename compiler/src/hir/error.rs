use std::fmt;

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