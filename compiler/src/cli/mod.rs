use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::context::{BuildMode, CompilerContext, CompilerOptions};
use crate::diagnostics::{Diagnostic, DiagnosticBag, Severity};
use crate::source::SourceError;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cli {
    command: Command,
}

impl Cli {
    pub fn parse() -> Self {
        Self::parse_from(env::args_os())
    }

    pub fn parse_from<I>(args: I) -> Self
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let executable = args
            .next()
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| String::from("gluauc"));

        let command = match args.next().and_then(|value| value.into_string().ok()) {
            Some(command) => Command::parse(command, args.collect(), executable),
            None => Command::Help,
        };

        Self { command }
    }

    pub fn execute(self) -> ExitCode {
        match self.command {
            Command::Build { file } => execute_file_command(BuildMode::Debug, file),
            Command::Run { file } => execute_file_command(BuildMode::Debug, file),
            Command::Check { file } => execute_file_command(BuildMode::Check, file),
            Command::Version => {
                println!("gluauc {VERSION}");
                ExitCode::SUCCESS
            }
            Command::Help => {
                print_help();
                ExitCode::SUCCESS
            }
            Command::Invalid { diagnostic } => {
                let mut diagnostics = DiagnosticBag::new();
                diagnostics.push(diagnostic);
                diagnostics.emit_to_stderr();
                ExitCode::FAILURE
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Build { file: PathBuf },
    Run { file: PathBuf },
    Check { file: PathBuf },
    Version,
    Help,
    Invalid { diagnostic: Diagnostic },
}

impl Command {
    fn parse(command: String, args: Vec<OsString>, executable: String) -> Self {
        match command.as_str() {
            "build" => parse_file_command(args, "build"),
            "run" => parse_file_command(args, "run"),
            "check" => parse_file_command(args, "check"),
            "version" | "--version" | "-V" => expect_no_args(args, Self::Version, "version"),
            "help" | "--help" | "-h" => expect_no_args(args, Self::Help, "help"),
            unknown => Self::Invalid {
                diagnostic: Diagnostic::error(format!("unknown command '{unknown}'"))
                    .with_note(format!("run '{executable} help' to see available commands")),
            },
        }
    }
}

fn parse_file_command(args: Vec<OsString>, command: &'static str) -> Command {
    if args.len() != 1 {
        return Command::Invalid {
            diagnostic: Diagnostic::error(format!("'{command}' expects exactly one .glu file"))
                .with_note(format!("usage: gluauc {command} <file.glu>")),
        };
    }

    let file = PathBuf::from(&args[0]);
    if file.extension().and_then(|extension| extension.to_str()) != Some("glu") {
        return Command::Invalid {
            diagnostic: Diagnostic::error("GraduaLuau source files must use the .glu extension")
                .with_note(format!("received '{}'", file.display())),
        };
    }

    match command {
        "build" => Command::Build { file },
        "run" => Command::Run { file },
        "check" => Command::Check { file },
        _ => unreachable!("validated file command"),
    }
}

fn expect_no_args(args: Vec<OsString>, command: Command, name: &'static str) -> Command {
    if args.is_empty() {
        return command;
    }

    Command::Invalid {
        diagnostic: Diagnostic::error(format!("'{name}' does not accept arguments")),
    }
}

fn execute_file_command(mode: BuildMode, file: PathBuf) -> ExitCode {
    let mut context = CompilerContext::new(CompilerOptions { mode });

    match context.sources.load_file(file.clone()) {
        Ok(file_id) => {
            let source = context
                .sources
                .get(file_id)
                .expect("loaded source file should be present");

            println!(
                "Loaded {} ({} bytes, {} lines).",
                source.path().display(),
                source.text().len(),
                source.line_count()
            );
            println!("{} is not implemented yet.", mode.description());
            ExitCode::SUCCESS
        }
        Err(error) => {
            context.diagnostics.push(source_error_to_diagnostic(error));
            context.diagnostics.emit_to_stderr();
            ExitCode::FAILURE
        }
    }
}

fn source_error_to_diagnostic(error: SourceError) -> Diagnostic {
    match error {
        SourceError::Read { path, source } => Diagnostic::new(
            Severity::Error,
            format!("failed to read '{}': {source}", path.display()),
        ),
    }
}

fn print_help() {
    println!(
        "\
GraduaLuau compiler

Usage:
  gluauc build <file.glu>   Compile an executable
  gluauc run <file.glu>     Compile and run a program
  gluauc check <file.glu>   Check a program without generating output
  gluauc version            Print compiler version
  gluauc help               Print this help message
"
    );
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn parses_check_command() {
        let cli = Cli::parse_from([
            OsString::from("gluauc"),
            OsString::from("check"),
            OsString::from("main.glu"),
        ]);

        assert_eq!(
            cli.command,
            Command::Check {
                file: PathBuf::from("main.glu")
            }
        );
    }

    #[test]
    fn rejects_non_glu_file() {
        let cli = Cli::parse_from([
            OsString::from("gluauc"),
            OsString::from("build"),
            OsString::from("main.lua"),
        ]);

        assert!(matches!(cli.command, Command::Invalid { .. }));
    }
}
