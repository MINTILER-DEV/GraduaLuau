use std::path::Path;

use crate::context::CompilerContext;
use crate::diagnostics::Diagnostic;
use crate::hir::HirStage;
use crate::llvm::LlvmStage;
use crate::lexer::{Lexer, TokenKind};
use crate::mir::MirStage;
use crate::parser::Parser;
use crate::runtime::RuntimeStage;
use crate::semantic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineKind {
    Build,
    Run,
    Check,
}

pub fn build(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    let (_file_id, ast) = prepare_source(context, source_path)?;
    generate_executable(context, &ast)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Build,
    })
}

pub fn run(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    let (_file_id, ast) = prepare_source(context, source_path)?;
    generate_executable(context, &ast)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Run,
    })
}

pub fn check(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    let _ = prepare_source(context, source_path)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Check,
    })
}

fn generate_executable(context: &CompilerContext, ast: &crate::parser::ast_builder::AstNode) -> Result<(), Diagnostic> {
    let hir = HirStage::lower(ast).map_err(|error| {
        Diagnostic::error("HIR lowering failed")
            .with_note(error.to_string())
    })?;
    let mir = MirStage::lower(&hir).map_err(|error| {
        Diagnostic::error("MIR lowering failed")
            .with_note(error.to_string())
    })?;
    let llvm_module = LlvmStage::generate(&mir);

    RuntimeStage::link(&context.options.output_path, &llvm_module).map_err(|error| {
        Diagnostic::error("failed to generate executable").with_note(error.to_string())
    })?;

    Ok(())
}

fn prepare_source(context: &mut CompilerContext, source_path: &Path) -> Result<(crate::source::FileId, crate::parser::ast_builder::AstNode), Diagnostic> {
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

    let (constant_diagnostics, _constant_results) = semantic::evaluate_constants(&ast);
    context.diagnostics.extend(constant_diagnostics);

    let (module_diagnostics, resolved_modules) = semantic::resolve_modules(&mut context.sources, file_id, &ast);
    context.diagnostics.extend(module_diagnostics);
    context.resolved_modules = resolved_modules;

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

    Ok((file_id, ast))
}

pub type PipelineResult = Result<PipelineOutput, Diagnostic>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineOutput {
    pub kind: PipelineKind,
}
