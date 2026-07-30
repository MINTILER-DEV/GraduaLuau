use std::process::ExitCode;

fn main() -> ExitCode {
    compiler::cli::Cli::parse().execute()
}
