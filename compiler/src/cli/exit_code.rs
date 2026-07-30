use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliExitCode {
    Success,
    CompilationError,
    InvalidUsage,
    InternalCompilerError,
}

impl From<CliExitCode> for ExitCode {
    fn from(value: CliExitCode) -> Self {
        match value {
            CliExitCode::Success => ExitCode::from(0),
            CliExitCode::CompilationError => ExitCode::from(1),
            CliExitCode::InvalidUsage => ExitCode::from(2),
            CliExitCode::InternalCompilerError => ExitCode::from(3),
        }
    }
}
