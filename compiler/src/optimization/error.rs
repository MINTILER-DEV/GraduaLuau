use std::fmt;

#[derive(Debug, Clone)]
pub enum OptimizationError {
    OptimizationFailed(String),
    VerificationError(String),
    InvalidOptimizationLevel(String),
}

impl fmt::Display for OptimizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizationError::OptimizationFailed(msg) => write!(f, "Optimization failed: {}", msg),
            OptimizationError::VerificationError(msg) => write!(f, "Verification error: {}", msg),
            OptimizationError::InvalidOptimizationLevel(msg) => write!(f, "Invalid optimization level: {}", msg),
        }
    }
}

impl std::error::Error for OptimizationError {}