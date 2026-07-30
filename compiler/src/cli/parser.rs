use std::ffi::OsString;
use std::path::PathBuf;

use crate::context::{BuildMode, CompilerOptions};
use crate::diagnostics::Diagnostic;
use crate::utils::LogLevel;

use super::command::{Command, FileAction, FileCommand, FormatCommand};
use super::paths::{default_output_path, resolve_source_path};

pub(super) struct CommandParser {
    executable: String,
    args: Vec<OsString>,
}

impl CommandParser {
    pub(super) fn new<I>(args: I) -> Self
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

    pub(super) fn parse(self) -> Command {
        let Some(command) = self.args.first().and_then(|value| value.to_str()) else {
            return Command::Help;
        };

        match command {
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
        }
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

        let command = FileCommand {
            source_path,
            options,
        };

        match action {
            FileAction::Build => Command::Build(command),
            FileAction::Run => Command::Run(command),
            FileAction::Check => Command::Check(command),
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

fn invalid(message: impl Into<String>) -> Command {
    Command::Invalid {
        diagnostic: Diagnostic::error(message),
    }
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
    use super::super::Cli;
    use super::Command;
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
