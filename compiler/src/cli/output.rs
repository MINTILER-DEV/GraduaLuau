use std::time::Instant;

use crate::context::{CompilerContext, CompilerOptions};
use crate::diagnostics::{Diagnostic, DiagnosticBag};

use super::command::FileAction;
use super::exit_code::CliExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(super) fn emit_single(diagnostic: Diagnostic, exit_code: CliExitCode) -> CliExitCode {
    let mut diagnostics = DiagnosticBag::new();
    diagnostics.push(diagnostic);
    diagnostics.emit_to_stderr();
    exit_code
}

pub(super) fn print_success(action: FileAction, context: &CompilerContext, started_at: Instant) {
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

pub(super) fn print_version() {
    println!("GraduaLuau Compiler");
    println!();
    println!("Version: {VERSION}");
    println!();
    println!("LLVM Backend");
    println!();
    println!("Target: {}", CompilerOptions::default().target_triple);
}

pub(super) fn print_help() {
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
  --emit-llvm    Write LLVM IR output
  --emit-hir     Write HIR output
  --emit-mir     Write MIR output
  --emit-ast     Write AST output
  --emit-tokens  Write token output
  --verbose      Enable debug logging
  --trace        Enable trace logging
"
    );
}
