#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    O0, // No optimization
    O1, // Basic optimizations
    O2, // Recommended optimizations
    O3, // Aggressive optimizations
}

impl OptimizationLevel {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s {
            "0" | "O0" => Ok(OptimizationLevel::O0),
            "1" | "O1" => Ok(OptimizationLevel::O1),
            "2" | "O2" => Ok(OptimizationLevel::O2),
            "3" | "O3" => Ok(OptimizationLevel::O3),
            _ => Err(format!("Unknown optimization level: {}", s)),
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            OptimizationLevel::O0 => "O0",
            OptimizationLevel::O1 => "O1",
            OptimizationLevel::O2 => "O2",
            OptimizationLevel::O3 => "O3",
        }
    }
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        OptimizationLevel::O2
    }
}