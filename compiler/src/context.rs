use crate::diagnostics::DiagnosticBag;
use crate::source::SourceManager;

#[derive(Debug)]
pub struct CompilerContext {
    pub sources: SourceManager,
    pub diagnostics: DiagnosticBag,
    pub options: CompilerOptions,
}

impl CompilerContext {
    pub fn new(options: CompilerOptions) -> Self {
        Self {
            sources: SourceManager::new(),
            diagnostics: DiagnosticBag::new(),
            options,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerOptions {
    pub mode: BuildMode,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            mode: BuildMode::Debug,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
    Check,
}

impl BuildMode {
    pub fn description(self) -> &'static str {
        match self {
            Self::Debug => "debug build",
            Self::Release => "release build",
            Self::Check => "check mode",
        }
    }
}
