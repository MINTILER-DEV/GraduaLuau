use std::path::Path;

use crate::context::CompilerContext;
use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    Build,
    Run,
    Check,
}

pub fn build(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    prepare_source(context, source_path)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Build,
    })
}

pub fn run(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    prepare_source(context, source_path)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Run,
    })
}

pub fn check(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    prepare_source(context, source_path)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Check,
    })
}

fn prepare_source(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    context
        .sources
        .load_file(source_path)
        .map(|_| PipelineOutput {
            kind: PipelineKind::Check,
        })
        .map_err(|error| {
            Diagnostic::error(format!("could not load '{}'", source_path.display()))
                .with_note(error.to_string())
        })
}

pub type PipelineResult = Result<PipelineOutput, Diagnostic>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineOutput {
    pub kind: PipelineKind,
}
