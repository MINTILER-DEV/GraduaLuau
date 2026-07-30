use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::Diagnostic;

pub(super) fn resolve_source_path(path: &Path) -> Result<PathBuf, Diagnostic> {
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

pub(super) fn default_output_path(source_path: &Path) -> PathBuf {
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
