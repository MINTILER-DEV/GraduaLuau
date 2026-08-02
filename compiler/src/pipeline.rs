use std::path::Path;

use crate::context::CompilerContext;
use crate::diagnostics::Diagnostic;
use crate::lexer::{Lexer, TokenKind};
use crate::parser::Parser;
use crate::semantic;

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
    let file_id = context.sources.load_file(source_path).map_err(|error| {
        Diagnostic::error(format!("could not load '{}'", source_path.display()))
            .with_note(error.to_string())
    })?;

    let file = context
        .sources
        .get(file_id)
        .expect("loaded file must exist in source manager");

    let mut lexer = Lexer::new(file);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        tokens.push(token.clone());
        if matches!(token.kind, TokenKind::EOF) {
            break;
        }
    }

    let mut parser = Parser::new(&tokens);
    let ast = parser.parse_program();
    context.diagnostics.extend(parser.diagnostics().iter().cloned());

    let semantic_result = semantic::analyze(&ast);
    context.diagnostics.extend(semantic_result.diagnostics);

    if context.diagnostics.has_errors() {
        let diagnostic = context
            .diagnostics
            .iter()
            .find(|d| d.severity().is_error())
            .cloned()
            .unwrap_or_else(|| Diagnostic::error("unknown compilation error"));
        return Err(diagnostic);
    }

    Ok(PipelineOutput {
        kind: PipelineKind::Check,
    })
}

pub type PipelineResult = Result<PipelineOutput, Diagnostic>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineOutput {
    pub kind: PipelineKind,
}
