use std::fs;
use std::path::{Path, PathBuf};

use crate::context::CompilerContext;
use crate::diagnostics::Diagnostic;
use crate::hir::{HirModule, HirPrinter, HirStage};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::llvm::{LlvmModule, LlvmStage};
use crate::mir::{MirModule, MirPrinter, MirStage};
use crate::optimization::OptimizationStage;
use crate::parser::ast_builder::AstNode;
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
    let prepared = prepare_source(context, source_path)?;
    let compilation = compile_for_native(&prepared.ast)?;

    emit_requested_artifacts(context, &prepared, &compilation)?;
    generate_executable(context, &compilation.optimized_module)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Build,
    })
}

pub fn run(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    let prepared = prepare_source(context, source_path)?;
    let compilation = compile_for_native(&prepared.ast)?;

    emit_requested_artifacts(context, &prepared, &compilation)?;
    generate_executable(context, &compilation.optimized_module)?;

    Ok(PipelineOutput {
        kind: PipelineKind::Run,
    })
}

pub fn check(context: &mut CompilerContext, source_path: &Path) -> PipelineResult {
    let prepared = prepare_source(context, source_path)?;

    if context.options.emit.any() {
        let compilation = compile_for_native(&prepared.ast)?;
        emit_requested_artifacts(context, &prepared, &compilation)?;
    }

    Ok(PipelineOutput {
        kind: PipelineKind::Check,
    })
}

fn compile_for_native(ast: &AstNode) -> Result<CompilationArtifacts, Diagnostic> {
    let hir = HirStage::lower(ast)
        .map_err(|error| Diagnostic::error("HIR lowering failed").with_note(error.to_string()))?;
    let mir = MirStage::lower(&hir)
        .map_err(|error| Diagnostic::error("MIR lowering failed").with_note(error.to_string()))?;
    let llvm_module = LlvmStage::generate(&mir)
        .map_err(|error| Diagnostic::error("LLVM generation failed").with_note(error.to_string()))?;

    let optimization_stage = OptimizationStage::default();
    let optimized_module = optimization_stage.optimize(&llvm_module).map_err(|error| {
        Diagnostic::error("Optimization failed").with_note(error.to_string())
    })?;

    Ok(CompilationArtifacts {
        hir,
        mir,
        llvm_module,
        optimized_module,
    })
}

fn generate_executable(context: &CompilerContext, optimized_module: &LlvmModule) -> Result<(), Diagnostic> {
    let mut runtime_stage = RuntimeStage::new();
    let diagnostics = runtime_stage
        .link(&context.options.output_path, optimized_module)
        .map_err(|error| Diagnostic::error("Runtime linking failed").with_note(error.to_string()))?;

    println!("{}", diagnostics.format());
    Ok(())
}

fn emit_requested_artifacts(
    context: &CompilerContext,
    prepared: &PreparedSource,
    compilation: &CompilationArtifacts,
) -> Result<(), Diagnostic> {
    if context.options.emit.tokens {
        let path = emission_path(&context.options.output_path, "tokens");
        write_emitted_file(&path, format!("{:#?}", prepared.tokens), "tokens")?;
    }

    if context.options.emit.ast {
        let path = emission_path(&context.options.output_path, "ast");
        write_emitted_file(&path, format!("{:#?}", prepared.ast), "AST")?;
    }

    if context.options.emit.hir {
        let mut printer = HirPrinter::new();
        let path = emission_path(&context.options.output_path, "hir");
        write_emitted_file(&path, printer.print_module(&compilation.hir), "HIR")?;
    }

    if context.options.emit.mir {
        let mut printer = MirPrinter::new();
        let path = emission_path(&context.options.output_path, "mir");
        write_emitted_file(&path, printer.print_module(&compilation.mir), "MIR")?;
    }

    if context.options.emit.llvm {
        let path = emission_path(&context.options.output_path, "ll");
        write_emitted_file(&path, compilation.llvm_module.ir.clone(), "LLVM IR")?;
    }

    Ok(())
}

fn write_emitted_file(path: &Path, contents: String, label: &str) -> Result<(), Diagnostic> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            Diagnostic::error(format!("failed to emit {label}"))
                .with_note(error.to_string())
                .with_note(format!("output: {}", path.display()))
        })?;
    }

    fs::write(path, contents).map_err(|error| {
        Diagnostic::error(format!("failed to emit {label}"))
            .with_note(error.to_string())
            .with_note(format!("output: {}", path.display()))
    })
}

fn emission_path(output_path: &Path, extension: &str) -> PathBuf {
    output_path.with_extension(extension)
}

fn prepare_source(
    context: &mut CompilerContext,
    source_path: &Path,
) -> Result<PreparedSource, Diagnostic> {
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

    Ok(PreparedSource { tokens, ast })
}

pub type PipelineResult = Result<PipelineOutput, Diagnostic>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipelineOutput {
    pub kind: PipelineKind,
}

#[derive(Debug)]
struct PreparedSource {
    tokens: Vec<Token>,
    ast: AstNode,
}

#[derive(Debug)]
struct CompilationArtifacts {
    hir: HirModule,
    mir: MirModule,
    llvm_module: LlvmModule,
    optimized_module: LlvmModule,
}
