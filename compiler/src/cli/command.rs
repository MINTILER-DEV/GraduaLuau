use std::path::PathBuf;
use std::time::Instant;

use crate::context::{CompilerContext, CompilerOptions};
use crate::diagnostics::Diagnostic;
use crate::pipeline;
use crate::utils::LogLevel;

use super::exit_code::CliExitCode;
use super::output::{emit_single, print_help, print_success, print_version};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Command {
    Build(FileCommand),
    Run(FileCommand),
    Check(FileCommand),
    Format(FormatCommand),
    Version,
    Help,
    Invalid { diagnostic: Diagnostic },
}

impl Command {
    pub(super) fn execute(self) -> CliExitCode {
        match self {
            Self::Build(command) => command.execute(FileAction::Build),
            Self::Run(command) => command.execute(FileAction::Run),
            Self::Check(command) => command.execute(FileAction::Check),
            Self::Format(command) => command.execute(),
            Self::Version => {
                print_version();
                CliExitCode::Success
            }
            Self::Help => {
                print_help();
                CliExitCode::Success
            }
            Self::Invalid { diagnostic } => emit_single(diagnostic, CliExitCode::InvalidUsage),
        }
    }
}

impl From<Diagnostic> for Command {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Invalid { diagnostic }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FileCommand {
    pub(super) source_path: PathBuf,
    pub(super) options: CompilerOptions,
}

impl FileCommand {
    fn execute(self, action: FileAction) -> CliExitCode {
        let mut context = CompilerContext::new(self.options);
        let started_at = Instant::now();

        let pipeline_result = match action {
            FileAction::Build => pipeline::build(&mut context, &self.source_path),
            FileAction::Run => pipeline::run(&mut context, &self.source_path),
            FileAction::Check => pipeline::check(&mut context, &self.source_path),
        };

        match pipeline_result {
            Ok(_) => {
                print_success(action, &context, started_at);
                CliExitCode::Success
            }
            Err(diagnostic) => {
                context.diagnostics.push(diagnostic);
                context.diagnostics.emit_to_stderr();
                CliExitCode::CompilationError
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FormatCommand {
    pub(super) path: PathBuf,
    pub(super) log_level: Option<LogLevel>,
}

impl FormatCommand {
    fn execute(self) -> CliExitCode {
        let _ = self.log_level;
        println!(
            "Formatting is reserved for a future GraduaLuau tooling phase: {}",
            self.path.display()
        );
        CliExitCode::Success
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileAction {
    Build,
    Run,
    Check,
}

impl FileAction {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Run => "run",
            Self::Check => "check",
        }
    }
}
