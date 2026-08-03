use std::fmt;

#[derive(Debug, Clone)]
pub enum LlvmError {
    GenerationError(String),
    VerificationError(String),
    TypeError(String),
}

impl fmt::Display for LlvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlvmError::GenerationError(msg) => write!(f, "LLVM generation error: {}", msg),
            LlvmError::VerificationError(msg) => write!(f, "LLVM verification error: {}", msg),
            LlvmError::TypeError(msg) => write!(f, "LLVM type error: {}", msg),
        }
    }
}

impl std::error::Error for LlvmError {}