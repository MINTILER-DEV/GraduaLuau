use compiler::hir::{
    HirBinaryOperator, HirBuilder, HirExpressionKind, HirModule, HirOptimizer, HirPrinter,
    HirScopeKind, HirStage, HirStatement, HirStatementKind, HirSymbolKind, HirType, HirValidator,
};
use compiler::lexer::{Lexer, TokenKind};
use compiler::parser::ast_builder::AstNode;
use compiler::parser::Parser;
use compiler::source::{SourceManager, SourceSpan};
use std::path::PathBuf;

fn parse_source(source: &str) -> Result<AstNode, String> {
    let mut sources = SourceManager::new();
    let file_id = sources.add_file(PathBuf::from("hir_test.glu"), source.to_string());
    let file = sources.get(file_id).ok_or("file not found")?;
    let mut lexer = Lexer::new(file);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let done = matches!(token.kind, TokenKind::EOF);
        tokens.push(token);
        if done {
            break;
        }
    }

    let mut parser = Parser::new(&tokens);
    let ast = parser.parse_program();
    if parser
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity().is_error())
    {
        return Err(format!("parser errors: {:?}", parser.diagnostics()));
    }

    Ok(ast)
}

fn lower_unoptimized(source: &str) -> Result<HirModule, String> {
    let ast = parse_source(source)?;
    HirBuilder::new()
        .build(&ast)
        .map_err(|error| error.to_string())
}

fn compile_to_hir(source: &str) -> Result<HirModule, String> {
    let ast = parse_source(source)?;
    HirStage::lower(&ast).map_err(|error| error.to_string())
}

fn validate(module: &HirModule) {
    HirValidator::new()
        .validate(module)
        .expect("HIR should validate");
}

fn main_body(module: &HirModule) -> &[HirStatement] {
    &module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("expected synthetic main")
        .body
}

#[test]
fn lowers_empty_module_with_root_scope_and_builtins() {
    let module = compile_to_hir("").unwrap();
    validate(&module);

    let root_scope = module
        .scopes
        .iter()
        .find(|scope| Some(scope.id) == module.metadata.root_scope)
        .unwrap();
    assert_eq!(module.name, "main");
    assert_eq!(root_scope.kind, HirScopeKind::Global);
    assert!(module.functions.is_empty());
    assert!(module
        .symbols
        .iter()
        .any(|symbol| symbol.name == "print" && symbol.kind == HirSymbolKind::BuiltinFunction));
}

#[test]
fn lowers_entry_locals_into_synthetic_main() {
    let module = compile_to_hir("local x = 5\nprint(x)").unwrap();
    validate(&module);

    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .unwrap();
    assert_eq!(module.global_variables.len(), 0);
    assert_eq!(main.local_variables.len(), 1);
    assert_eq!(main.local_variables[0].var_type, Some(HirType::Integer));
}

#[test]
fn resolves_symbols_and_scope_tree_for_nested_functions() {
    let module =
        compile_to_hir("function outer(x: integer)\nfunction inner()\nreturn x\nend\nend").unwrap();
    validate(&module);

    let root_scope = module
        .scopes
        .iter()
        .find(|scope| Some(scope.id) == module.metadata.root_scope)
        .unwrap();
    let outer = module
        .functions
        .iter()
        .find(|function| function.name == "outer")
        .unwrap();
    let HirStatementKind::Function { function: inner } = &outer.body[0].kind else {
        panic!("expected nested function");
    };

    assert!(root_scope.children.contains(&outer.scope_id));
    assert!(module
        .scopes
        .iter()
        .find(|scope| scope.id == outer.scope_id)
        .unwrap()
        .children
        .contains(&inner.scope_id));
    assert_eq!(inner.return_type, Some(HirType::Integer));
}

#[test]
fn integrates_types_for_functions_and_returns() {
    let module = compile_to_hir("function square(x: integer): number\nreturn x * x\nend").unwrap();
    validate(&module);

    let function = module
        .functions
        .iter()
        .find(|function| function.name == "square")
        .unwrap();
    assert_eq!(function.signature.parameter_types, vec![HirType::Integer]);
    assert_eq!(function.signature.return_type, HirType::Number);
    assert_eq!(function.return_type, Some(HirType::Number));
}

#[test]
fn optimizes_constant_arithmetic_before_mir() {
    let module = compile_to_hir("local x = 2 + 3").unwrap();
    validate(&module);

    let HirStatementKind::LocalVariable {
        initializer: Some(initializer),
        ..
    } = &main_body(&module)[0].kind
    else {
        panic!("expected optimized local");
    };
    assert!(matches!(initializer.kind, HirExpressionKind::Number(5.0)));
}

#[test]
fn keeps_side_effecting_builtin_calls_while_removing_dead_expressions() {
    let module = compile_to_hir("1 + 2\nprint(\"alive\")").unwrap();
    validate(&module);

    let body = main_body(&module);
    assert_eq!(body.len(), 1);
    assert!(matches!(body[0].kind, HirStatementKind::Expression(_)));
}

#[test]
fn handles_assignments_without_propagating_mutated_locals() {
    let module = compile_to_hir("local x = 1\nx = 2\nlocal y = x").unwrap();
    validate(&module);

    let HirStatementKind::LocalVariable {
        initializer: Some(initializer),
        ..
    } = &main_body(&module)[2].kind
    else {
        panic!("expected y local");
    };
    assert!(matches!(
        initializer.kind,
        HirExpressionKind::LocalVariable(_)
    ));
}

#[test]
fn lowers_strings_booleans_tables_and_interpolation() {
    let module = compile_to_hir(
        "local x = true and false\nlocal t = {name = \"GraduaLuau\"}\nprint(`hello {x}`)",
    )
    .unwrap();
    validate(&module);

    assert_eq!(main_body(&module).len(), 3);
    assert!(module
        .symbols
        .iter()
        .any(|symbol| symbol.name == "t" && symbol.kind == HirSymbolKind::Local));
}

#[test]
fn supports_luau_shorthand_call_syntax() {
    let module = compile_to_hir("print \"Hello\"").unwrap();
    validate(&module);

    let HirStatementKind::Expression(expression) = &main_body(&module)[0].kind else {
        panic!("expected print expression");
    };
    assert!(matches!(
        expression.kind,
        HirExpressionKind::BuiltinCall { .. }
    ));
}

#[test]
fn reports_undefined_identifier_errors() {
    let error = compile_to_hir("print(missing)").unwrap_err();
    assert!(error.contains("Undefined identifier 'missing'"));
}

#[test]
fn rejects_type_mismatch_errors() {
    let error = compile_to_hir("local x: number = \"hello\"").unwrap_err();
    assert!(error.contains("Cannot assign value of type 'string'"));
}

#[test]
fn validation_rejects_corrupted_symbol_references() {
    let mut module = compile_to_hir("print(\"hello\")").unwrap();
    let main = module
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .unwrap();
    let HirStatementKind::Expression(expression) = &mut main.body[0].kind else {
        panic!("expected expression");
    };
    expression.symbol_id = Some(compiler::hir::HirSymbolId::new(9999));

    assert!(HirValidator::new().validate(&module).is_err());
}

#[test]
fn validation_rejects_invalid_control_flow() {
    let mut module = compile_to_hir("print(\"hello\")").unwrap();
    let span = SourceSpan::new(
        module.span.file_id(),
        module.span.start(),
        module.span.end(),
    );
    let main = module
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .unwrap();
    main.body.push(HirStatement {
        kind: HirStatementKind::Break,
        span,
    });

    assert!(HirValidator::new().validate(&module).is_err());
}

#[test]
fn printer_golden_output_contains_symbols_scopes_and_types() {
    let module = compile_to_hir("local x = 5\nprint(x)").unwrap();
    let output = HirPrinter::new().print_module(&module);

    for expected in [
        "Module 'main'",
        "Scopes:",
        "Scope #0 Global",
        "Symbols:",
        "Function 'main'",
        "Local Variable: x",
        "type=Some(Integer)",
        "<builtin Print>",
    ] {
        assert!(
            output.contains(expected),
            "expected HIR output to contain {expected:?}\n{output}"
        );
    }
}

#[test]
fn optimizer_statistics_report_is_available() {
    let ast = parse_source("1 + 2").unwrap();
    let result = HirStage::lower_with_optimization(&ast).unwrap();
    validate(&result.module);

    let report = result.stats.report();
    assert!(report.contains("HIR Optimization"));
    assert!(report.contains("Constant Folds: 1"));
    assert!(report.contains("Dead Expressions Removed: 1"));
}

#[test]
fn regression_forward_references_and_shadowing_remain_stable() {
    let module = compile_to_hir(
        "function foo(x: integer)\nfunction inner()\nlocal x = 2\nreturn x\nend\nreturn x\nend",
    )
    .unwrap();
    validate(&module);

    let foo_symbols = module
        .symbols
        .iter()
        .filter(|symbol| symbol.name == "x")
        .collect::<Vec<_>>();
    assert_eq!(foo_symbols.len(), 2);
}

#[test]
fn regression_nested_return_constant_folding_survives_validation() {
    let module = compile_to_hir("function value(): integer\nreturn (1 + 2) * 3\nend").unwrap();
    validate(&module);

    let function = module
        .functions
        .iter()
        .find(|function| function.name == "value")
        .unwrap();
    let HirStatementKind::Return(Some(values)) = &function.body[0].kind else {
        panic!("expected return");
    };
    assert!(matches!(values[0].kind, HirExpressionKind::Number(9.0)));
}

#[test]
fn large_hir_smoke_test_remains_fast_and_valid() {
    let mut source = String::new();
    for index in 0..50 {
        source.push_str(&format!("local value{index} = {index} + 1\n"));
    }
    source.push_str("print(value49)\n");

    let module = compile_to_hir(&source).unwrap();
    validate(&module);
    assert!(main_body(&module).len() >= 50);
}

#[test]
fn unoptimized_hir_can_be_compared_with_optimized_hir() {
    let unoptimized = lower_unoptimized("local x = 2 + 3").unwrap();
    let optimized = HirOptimizer::new().optimize(&unoptimized).module;
    let unoptimized_text = HirPrinter::new().print_module(&unoptimized);
    let optimized_text = HirPrinter::new().print_module(&optimized);

    assert_ne!(unoptimized_text, optimized_text);
    assert!(optimized_text.contains(" = 5"));
}

#[test]
fn binary_operator_metadata_survives_unoptimized_lowering() {
    let module = lower_unoptimized("local x = 1 + 2").unwrap();
    let HirStatementKind::LocalVariable {
        initializer: Some(initializer),
        ..
    } = &main_body(&module)[0].kind
    else {
        panic!("expected local");
    };
    let HirExpressionKind::Binary { operator, .. } = initializer.kind else {
        panic!("expected unoptimized binary");
    };
    assert_eq!(operator, HirBinaryOperator::Add);
}
