use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), CliError> {
    let mut args = env::args_os();
    let executable = args.next().unwrap_or_default();

    let Some(path) = args.next().map(PathBuf::from) else {
        return Err(CliError::usage(executable.to_string_lossy()));
    };

    if args.next().is_some() {
        return Err(CliError::usage(executable.to_string_lossy()));
    }

    let source = fs::read_to_string(&path).map_err(|source| CliError::ReadSource {
        path: path.clone(),
        source,
    })?;

    let output = gluac::compile_source(&source)?;

    println!("Compiled {} successfully.", path.display());
    println!("{}", output.codegen.text);

    Ok(())
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    ReadSource {
        path: PathBuf,
        source: std::io::Error,
    },
    Compile(gluac::errors::CompilerError),
}

impl CliError {
    fn usage(executable: impl AsRef<str>) -> Self {
        Self::Usage(format!("Usage: {} <file.glu>", executable.as_ref()))
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => formatter.write_str(message),
            Self::ReadSource { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::Compile(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for CliError {}

impl From<gluac::errors::CompilerError> for CliError {
    fn from(source: gluac::errors::CompilerError) -> Self {
        Self::Compile(source)
    }
}
