use std::path::PathBuf;

use crate::diagnostics::DiagnosticBag;
use crate::semantic::ModuleMetadata;
use crate::source::SourceManager;
use crate::utils::LogLevel;

#[derive(Debug)]
pub struct CompilerContext {
    pub sources: SourceManager,
    pub diagnostics: DiagnosticBag,
    pub resolved_modules: Vec<ModuleMetadata>,
    pub options: CompilerOptions,
}

impl CompilerContext {
    pub fn new(options: CompilerOptions) -> Self {
        Self {
            sources: SourceManager::new(),
            diagnostics: DiagnosticBag::new(),
            resolved_modules: Vec::new(),
            options,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerOptions {
    pub build_mode: BuildMode,
    pub target_triple: String,
    pub output_path: PathBuf,
    pub debug_symbols: bool,
    pub warnings: WarningMode,
    pub runtime_mode: RuntimeMode,
    pub emit: EmitOptions,
    pub log_level: Option<LogLevel>,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            build_mode: BuildMode::Debug,
            target_triple: default_target_triple(),
            output_path: PathBuf::from("build"),
            debug_symbols: true,
            warnings: WarningMode::Default,
            runtime_mode: RuntimeMode::Native,
            emit: EmitOptions::default(),
            log_level: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

impl BuildMode {
    pub fn description(self) -> &'static str {
        match self {
            Self::Debug => "debug build",
            Self::Release => "release build",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningMode {
    Default,
    Deny,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Native,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EmitOptions {
    pub llvm: bool,
    pub hir: bool,
    pub mir: bool,
    pub ast: bool,
    pub tokens: bool,
}

impl EmitOptions {
    pub fn any(self) -> bool {
        self.llvm || self.hir || self.mir || self.ast || self.tokens
    }
}

fn default_target_triple() -> String {
    if cfg!(all(target_os = "windows", target_env = "msvc")) {
        return format!("{}-pc-windows-msvc", std::env::consts::ARCH);
    }

    if cfg!(all(target_os = "windows", target_env = "gnu")) {
        return format!("{}-pc-windows-gnu", std::env::consts::ARCH);
    }

    if cfg!(target_os = "macos") {
        return format!("{}-apple-darwin", std::env::consts::ARCH);
    }

    if cfg!(target_os = "linux") {
        return format!("{}-unknown-linux-gnu", std::env::consts::ARCH);
    }

    format!(
        "{}-unknown-{}",
        std::env::consts::ARCH,
        std::env::consts::OS
    )
}
