use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use crate::context::{BuildMode, CompilerContext, CompilerOptions};
use crate::diagnostics::{Diagnostic, DiagnosticBag};
use crate::pipeline;
use crate::utils::LogLevel;

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
        CommandParser::new(args).parse()
    }

    pub fn execute(self) -> ExitCode {
        self.command.execute().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Build(FileCommand),
    Run(FileCommand),
    Check(FileCommand),
    Format(FormatCommand),
    Version,
    Help,
    Invalid { diagnostic: Diagnostic },
}

impl Command {
    fn execute(self) -> CliExitCode {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCommand {
    source_path: PathBuf,
    options: CompilerOptions,
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
pub struct FormatCommand {
    path: PathBuf,
    log_level: Option<LogLevel>,
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
enum FileAction {
    Build,
    Run,
    Check,
}

struct CommandParser {
    executable: String,
    args: Vec<OsString>,
}

impl CommandParser {
    fn new<I>(args: I) -> Self
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let executable = args
            .next()
            .and_then(|value| value.into_string().ok())
            .unwrap_or_else(|| String::from("gluauc"));

        Self {
            executable,
            args: args.collect(),
        }
    }

    fn parse(self) -> Cli {
        let Some(command) = self.args.first().and_then(|value| value.to_str()) else {
            return Cli {
                command: Command::Help,
            };
        };

        let command = match command {
            "build" => self.parse_file_command(FileAction::Build),
            "run" => self.parse_file_command(FileAction::Run),
            "check" => self.parse_file_command(FileAction::Check),
            "fmt" | "format" => self.parse_format_command(),
            "version" | "--version" | "-V" => self.expect_no_args(Command::Version, "version"),
            "help" | "--help" | "-h" => self.expect_no_args(Command::Help, "help"),
            reserved if is_reserved_command(reserved) => Command::Invalid {
                diagnostic: Diagnostic::error(format!(
                    "'{reserved}' is reserved but not implemented yet"
                ))
                .with_note("available commands: build, run, check, fmt, version, help"),
            },
            unknown => Command::Invalid {
                diagnostic: Diagnostic::error("unknown command")
                    .with_note(format!("received: {unknown}"))
                    .with_note(command_hint(unknown))
                    .with_note(format!(
                        "run '{} help' to see available commands",
                        self.executable
                    )),
            },
        };

        Cli { command }
    }

    fn parse_file_command(self, action: FileAction) -> Command {
        let command_name = action.name();
        let mut options = CompilerOptions::default();
        let mut source_path = None;
        let mut positional_count = 0usize;
        let mut args = self.args.into_iter().skip(1).peekable();

        while let Some(argument) = args.next() {
            let Some(argument_text) = argument.to_str() else {
                return invalid(format!("'{command_name}' received a non-UTF-8 argument"));
            };

            match argument_text {
                "--release" => {
                    options.build_mode = BuildMode::Release;
                    options.debug_symbols = false;
                }
                "--debug" => {
                    options.build_mode = BuildMode::Debug;
                    options.debug_symbols = true;
                }
                "--verbose" => options.log_level = Some(LogLevel::Debug),
                "--trace" => options.log_level = Some(LogLevel::Trace),
                "--emit-llvm" => options.emit.llvm = true,
                "--emit-hir" => options.emit.hir = true,
                "--emit-mir" => options.emit.mir = true,
                "--emit-ast" => options.emit.ast = true,
                "--emit-tokens" => options.emit.tokens = true,
                "-o" => {
                    let Some(output_path) = args.next() else {
                        return invalid("'-o' expects an output path");
                    };

                    options.output_path = PathBuf::from(output_path);
                }
                unknown if unknown.starts_with('-') => {
                    return invalid(format!("unknown option '{unknown}'"));
                }
                path => {
                    positional_count += 1;
                    if positional_count > 1 {
                        return invalid(format!(
                            "'{command_name}' expects exactly one source path"
                        ));
                    }

                    source_path = Some(PathBuf::from(path));
                }
            }
        }

        let Some(source_path) = source_path else {
            return Command::Invalid {
                diagnostic: Diagnostic::error(format!("'{command_name}' expects a source path"))
                    .with_note(format!("usage: gluauc {command_name} [options] <file.glu>")),
            };
        };

        let source_path = match resolve_source_path(&source_path) {
            Ok(path) => path,
            Err(diagnostic) => return Command::Invalid { diagnostic },
        };

        if options.output_path == PathBuf::from("build") {
            options.output_path = default_output_path(&source_path);
        }

        match action {
            FileAction::Build => Command::Build(FileCommand {
                source_path,
                options,
            }),
            FileAction::Run => Command::Run(FileCommand {
                source_path,
                options,
            }),
            FileAction::Check => Command::Check(FileCommand {
                source_path,
                options,
            }),
        }
    }

    fn parse_format_command(self) -> Command {
        let mut path = None;
        let mut log_level = None;

        for argument in self.args.into_iter().skip(1) {
            let Some(argument_text) = argument.to_str() else {
                return invalid("'fmt' received a non-UTF-8 argument");
            };

            match argument_text {
                "--verbose" => log_level = Some(LogLevel::Debug),
                "--trace" => log_level = Some(LogLevel::Trace),
                unknown if unknown.starts_with('-') => {
                    return invalid(format!("unknown option '{unknown}'"));
                }
                path_text if path.is_none() => path = Some(PathBuf::from(path_text)),
                _ => return invalid("'fmt' expects exactly one path"),
            }
        }

        let Some(path) = path else {
            return Command::Invalid {
                diagnostic: Diagnostic::error("'fmt' expects a path")
                    .with_note("usage: gluauc fmt <path>"),
            };
        };

        Command::Format(FormatCommand { path, log_level })
    }

    fn expect_no_args(self, command: Command, name: &'static str) -> Command {
        if self.args.len() == 1 {
            return command;
        }

        invalid(format!("'{name}' does not accept arguments"))
    }
}

impl FileAction {
    fn name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Run => "run",
            Self::Check => "check",
        }
    }
}

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

impl From<Diagnostic> for Command {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Invalid { diagnostic }
    }
}

fn resolve_source_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("glu") {
        return Err(
            Diagnostic::error("GraduaLuau source files must use the .glu extension")
                .with_note(format!("received: {}", path.display())),
        );
    }

    let metadata = fs::metadata(path).map_err(|_| {
        Diagnostic::error("could not find source file").with_note(format!("{}", path.display()))
    })?;

    if !metadata.is_file() {
        return Err(Diagnostic::error("source path must point to a file")
            .with_note(format!("received: {}", path.display())));
    }

    fs::canonicalize(path).map_err(|source| {
        Diagnostic::error("could not canonicalize source path")
            .with_note(format!("{}", path.display()))
            .with_note(source.to_string())
    })
}

fn default_output_path(source_path: &Path) -> PathBuf {
    let mut path = PathBuf::from("build");
    let executable_name = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("main");

    path.push(executable_name);

    if cfg!(windows) {
        path.set_extension("exe");
    }

    path
}

fn print_success(action: FileAction, context: &CompilerContext, started_at: Instant) {
    match action {
        FileAction::Build => {
            println!("Compiling...");
            println!("Finished in {:.2?}", started_at.elapsed());
            println!("Output:");
            println!("{}", context.options.output_path.display());
        }
        FileAction::Run => {
            println!("Compiling...");
            println!("Finished in {:.2?}", started_at.elapsed());
            println!("Run step is not implemented yet.");
        }
        FileAction::Check => {
            println!("Checking...");
            println!("Finished in {:.2?}", started_at.elapsed());
        }
    }
}

fn print_version() {
    println!("GraduaLuau Compiler");
    println!();
    println!("Version: {VERSION}");
    println!();
    println!("LLVM Backend");
    println!();
    println!("Target: {}", CompilerOptions::default().target_triple);
}

fn print_help() {
    println!(
        "\
Usage:

gluauc build <file.glu>
gluauc run <file.glu>
gluauc check <file.glu>
gluauc fmt <path>
gluauc version
gluauc help

Options:
  -o <path>       Specify output file
  --release      Use release build options
  --debug        Use debug build options
  --emit-llvm    Reserve LLVM IR output
  --emit-hir     Reserve HIR output
  --emit-mir     Reserve MIR output
  --emit-ast     Reserve AST output
  --emit-tokens  Reserve token output
  --verbose      Enable debug logging
  --trace        Enable trace logging
"
    );
}

fn invalid(message: impl Into<String>) -> Command {
    Command::Invalid {
        diagnostic: Diagnostic::error(message),
    }
}

fn emit_single(diagnostic: Diagnostic, exit_code: CliExitCode) -> CliExitCode {
    let mut diagnostics = DiagnosticBag::new();
    diagnostics.push(diagnostic);
    diagnostics.emit_to_stderr();
    exit_code
}

fn is_reserved_command(command: &str) -> bool {
    matches!(
        command,
        "new" | "clean" | "doc" | "test" | "bench" | "publish" | "install"
    )
}

fn command_hint(command: &str) -> String {
    let suggestion = match command {
        "bulid" | "buid" => Some("build"),
        "chek" => Some("check"),
        "rn" => Some("run"),
        "formt" => Some("fmt"),
        "verison" => Some("version"),
        _ => None,
    };

    match suggestion {
        Some(suggestion) => format!("did you mean: {suggestion}"),
        None => String::from("available commands: build, run, check, fmt, version, help"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use crate::context::BuildMode;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_valid_commands() {
        let source = temp_source("valid_commands");

        assert!(matches!(
            parse(["gluauc", "build", source.to_str().unwrap()]).command,
            Command::Build(_)
        ));
        assert!(matches!(
            parse(["gluauc", "run", source.to_str().unwrap()]).command,
            Command::Run(_)
        ));
        assert!(matches!(
            parse(["gluauc", "check", source.to_str().unwrap()]).command,
            Command::Check(_)
        ));
        assert!(matches!(
            parse(["gluauc", "fmt", "."]).command,
            Command::Format(_)
        ));
        assert_eq!(parse(["gluauc", "help"]).command, Command::Help);
        assert_eq!(parse(["gluauc", "version"]).command, Command::Version);

        let _ = fs::remove_file(source);
    }

    #[test]
    fn creates_release_options() {
        let source = temp_source("release_options");
        let cli = parse(["gluauc", "build", "--release", source.to_str().unwrap()]);

        let Command::Build(command) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(command.options.build_mode, BuildMode::Release);
        assert!(!command.options.debug_symbols);

        let _ = fs::remove_file(source);
    }

    #[test]
    fn accepts_output_path() {
        let source = temp_source("output_path");
        let cli = parse([
            "gluauc",
            "build",
            source.to_str().unwrap(),
            "-o",
            "custom/out.exe",
        ]);

        let Command::Build(command) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(command.options.output_path, PathBuf::from("custom/out.exe"));

        let _ = fs::remove_file(source);
    }

    #[test]
    fn rejects_invalid_commands_and_usage() {
        assert!(matches!(
            parse(["gluauc", "bulid", "main.glu"]).command,
            Command::Invalid { .. }
        ));
        assert!(matches!(
            parse(["gluauc", "build"]).command,
            Command::Invalid { .. }
        ));
        assert!(matches!(
            parse(["gluauc", "build", "--wat", "main.glu"]).command,
            Command::Invalid { .. }
        ));
        assert!(matches!(
            parse(["gluauc", "build", "main.lua"]).command,
            Command::Invalid { .. }
        ));
    }

    #[test]
    fn rejects_missing_files() {
        assert!(matches!(
            parse(["gluauc", "check", "definitely_missing.glu"]).command,
            Command::Invalid { .. }
        ));
    }

    fn parse<const N: usize>(args: [&str; N]) -> Cli {
        Cli::parse_from(args.into_iter().map(OsString::from))
    }

    fn temp_source(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        path.push(format!("gradualuau_{name}_{nanos}.glu"));
        fs::write(&path, "print \"hello\"").expect("test source should be writable");
        canonical_path(&path)
    }

    fn canonical_path(path: &Path) -> PathBuf {
        fs::canonicalize(path).expect("test source should canonicalize")
    }
}
