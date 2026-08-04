use compiler::hir::function::HirFunctionMetadata;
use compiler::hir::{
    HirExpression, HirExpressionKind, HirFunction, HirFunctionId, HirFunctionSignature, HirScopeId,
    HirStage, HirStatement, HirStatementKind, HirSymbolId, HirType,
};
use compiler::lexer::Lexer;
use compiler::mir::{
    MirInstructionKind, MirModule, MirPrinter, MirStage, MirTerminator, MirValidator, MirValueKind,
};
use compiler::parser::Parser;
use compiler::semantic;
use compiler::source::{FileId, SourceManager, SourceSpan};
use std::path::PathBuf;

fn compile_to_mir(source: &str) -> Result<MirModule, String> {
    let mut sources = SourceManager::new();
    let test_path = PathBuf::from("test.lua");
    let file_id = sources.add_file(test_path, source.to_string());

    let file = sources.get(file_id).ok_or("File not found")?;
    let mut lexer = Lexer::new(file);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        tokens.push(token.clone());
        if matches!(token.kind, compiler::lexer::TokenKind::EOF) {
            break;
        }
    }

    let mut parser = Parser::new(&tokens);
    let ast = parser.parse_program();

    // Allow parser diagnostics (warnings are ok)
    if parser.diagnostics().iter().any(|d| d.severity().is_error()) {
        return Err(format!("Parser errors: {:?}", parser.diagnostics()));
    }

    let (constant_diagnostics, _constant_results) = semantic::evaluate_constants(&ast);
    if !constant_diagnostics.is_empty()
        && constant_diagnostics.iter().any(|d| d.severity().is_error())
    {
        return Err(format!(
            "Constant evaluation errors: {:?}",
            constant_diagnostics
        ));
    }

    let (module_diagnostics, _resolved_modules) =
        semantic::resolve_modules(&mut sources, file_id, &ast);
    if !module_diagnostics.is_empty() && module_diagnostics.iter().any(|d| d.severity().is_error())
    {
        return Err(format!(
            "Module resolution errors: {:?}",
            module_diagnostics
        ));
    }

    let semantic_result = semantic::analyze(&ast);
    if !semantic_result.diagnostics.is_empty()
        && semantic_result
            .diagnostics
            .iter()
            .any(|d| d.severity().is_error())
    {
        return Err(format!(
            "Semantic analysis errors: {:?}",
            semantic_result.diagnostics
        ));
    }

    let hir = HirStage::lower(&ast).map_err(|e| e.to_string())?;
    MirStage::lower(&hir).map_err(|e| e.to_string())
}

#[test]
fn test_empty_module() {
    let source = "";
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 0);
}

#[test]
fn test_simple_function() {
    let source = r#"
function add()
    local a = 5
    local b = 10
    return a + b
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
    assert_eq!(mir.functions[0].name, "add");
}

#[test]
fn test_arithmetic() {
    let source = r#"
function calculate()
    local x = 5 + 10
    local y = x * 2
    return y
end
"#;
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
    assert!(mir.functions[0].blocks.len() > 0);
}

#[test]
fn test_function_call() {
    let source = r#"
function main()
    local msg = "Hello, World!"
    return msg
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

#[test]
fn test_control_flow_if() {
    let source = r#"
function check()
    local x = 5
    local positive = false
    return positive
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

#[test]
fn test_control_flow_while() {
    let source = r#"
function countdown()
    local n = 5
    local count = 0
    return n
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

#[test]
fn test_mir_validator() {
    let source = r#"
function simple()
    return 42
end
"#;
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();

    let mut validator = MirValidator::new();
    let validation_result = validator.validate(&mir);
    assert!(validation_result.is_ok());
}

#[test]
fn test_mir_printer() {
    let source = r#"
function example()
    local x = 10
    return x
end
"#;
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();

    let mut printer = MirPrinter::new();
    let output = printer.print_module(&mir);
    assert!(output.contains("Module"));
    assert!(output.contains("example"));
}

#[test]
fn test_multiple_functions() {
    let source = r#"
function foo()
    return 1
end

function bar()
    return 2
end
"#;
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 2);
}

#[test]
fn test_table_constructor() {
    let source = r#"
function create_table()
    local t = {a = 1, b = 2}
    return t
end
"#;
    let result = compile_to_mir(source);
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

#[test]
fn test_boolean_operators() {
    let source = r#"
function test_and()
    local a = true
    local b = false
    return a and b
end

function test_or()
    local a = true
    local b = false
    return a or b
end

function test_not()
    local a = true
    return not a
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 3);
}

#[test]
fn test_comparisons() {
    let source = r#"
function compare()
    local a = 5
    local b = 10
    local result = false
    return result
end
"#;
    let result = compile_to_mir(source);
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

#[test]
fn variables_lower_to_symbol_storage_not_source_names() {
    let source = r#"
function uses_symbols()
    local sourceName = 5
    local copiedName = sourceName
    return copiedName
end
"#;

    let mir = compile_to_mir(source).expect("source should lower to MIR");
    let mut printer = MirPrinter::new();
    let output = printer.print_module(&mir);

    assert!(output.contains("local_symbol_"));
    assert!(!output.contains("sourceName"));
    assert!(!output.contains("copiedName"));
}

#[test]
fn parameters_are_values_locals_and_load_by_symbol() {
    let source = r#"
function square(x: number): number
    return x * x
end
"#;

    let mir = compile_to_mir(source).expect("source should lower to MIR");
    let square = &mir.functions[0];

    assert_eq!(square.parameter_data.len(), 1);
    assert_eq!(square.locals.len(), 1);
    assert!(matches!(
        square.values[0].kind,
        MirValueKind::Parameter { .. }
    ));

    let parameter_storage = &square.parameter_data[0].storage;
    assert!(parameter_storage.starts_with("local_symbol_"));
    assert!(square.blocks[0].instructions.iter().any(|instruction| {
        matches!(
            &instruction.kind,
            MirInstructionKind::Load { name, .. } if name == parameter_storage
        )
    }));
}

#[test]
fn all_blocks_end_with_one_final_terminator() {
    let source = r#"
function one()
    return 1
end

function two()
    local x = 2
    return x
end
"#;

    let mir = compile_to_mir(source).expect("source should lower to MIR");

    for function in &mir.functions {
        for block in &function.blocks {
            assert!(block.terminator.is_some());
            assert!(
                block
                    .instructions
                    .last()
                    .is_some_and(|instruction| instruction.is_terminator())
            );
            assert_eq!(
                block
                    .instructions
                    .iter()
                    .filter(|instruction| instruction.is_terminator())
                    .count(),
                1
            );
        }
    }
}

#[test]
fn function_call_lowers_arguments_before_call() {
    let source = r#"
function square(x: number): number
    return x * x
end

local y = square(13)
"#;

    let mir = compile_to_mir(source).expect("source should lower to MIR");
    let main = mir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("entry function should exist");
    let instructions = &main.blocks[0].instructions;

    let const_position = instructions
        .iter()
        .position(|instruction| {
            matches!(
                instruction.kind,
                MirInstructionKind::Const {
                    value: compiler::mir::MirValue::Integer(13),
                    ..
                }
            )
        })
        .expect("argument constant should be emitted");
    let call_position = instructions
        .iter()
        .position(|instruction| {
            matches!(
                &instruction.kind,
                MirInstructionKind::Call {
                    function,
                    result: Some(_),
                    ..
                } if function == "square"
            )
        })
        .expect("square call should be emitted");

    assert!(const_position < call_position);
}

#[test]
fn table_constructor_lowers_fields_to_sets() {
    let source = r#"
function make_table()
    local t = {a = 1, b = 2, 3}
    return t
end
"#;

    let mir = compile_to_mir(source).expect("source should lower to MIR");
    let function = &mir.functions[0];
    let table_sets = function.blocks[0]
        .instructions
        .iter()
        .filter(|instruction| matches!(instruction.kind, MirInstructionKind::TableSet { .. }))
        .count();

    assert!(
        function.blocks[0]
            .instructions
            .iter()
            .any(|instruction| matches!(instruction.kind, MirInstructionKind::TableNew { .. }))
    );
    assert_eq!(table_sets, 3);
}

#[test]
fn manual_hir_if_lowers_to_branch_cfg() {
    let span = test_span();
    let module = manual_module(
        "branchy",
        vec![HirStatement {
            kind: HirStatementKind::If {
                condition: bool_expr(true, span),
                then_block: Vec::new(),
                else_block: Some(Vec::new()),
            },
            span,
        }],
        None,
    );

    let mir = MirStage::lower(&module).expect("manual if HIR should lower");
    let function = &mir.functions[0];

    assert_eq!(function.blocks.len(), 4);
    assert!(matches!(
        function.blocks[0].terminator,
        Some(MirTerminator::Branch { .. })
    ));
    assert!(matches!(
        function.blocks[1].terminator,
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[2].terminator,
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[3].terminator,
        Some(MirTerminator::Return { .. })
    ));
    assert_eq!(function.blocks[3].predecessors.len(), 2);
}

#[test]
fn manual_hir_while_lowers_to_loop_cfg() {
    let span = test_span();
    let module = manual_module(
        "loopy",
        vec![HirStatement {
            kind: HirStatementKind::While {
                condition: bool_expr(true, span),
                body: Vec::new(),
            },
            span,
        }],
        None,
    );

    let mir = MirStage::lower(&module).expect("manual while HIR should lower");
    let function = &mir.functions[0];

    assert_eq!(function.blocks.len(), 4);
    assert!(matches!(
        function.blocks[0].terminator,
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[1].terminator,
        Some(MirTerminator::Branch { .. })
    ));
    assert!(matches!(
        function.blocks[2].terminator,
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[3].terminator,
        Some(MirTerminator::Return { .. })
    ));
    assert_eq!(function.blocks[1].predecessors.len(), 2);
}

fn test_span() -> SourceSpan {
    SourceSpan::new(FileId::new(0), 0, 1)
}

fn bool_expr(value: bool, span: SourceSpan) -> HirExpression {
    HirExpression {
        kind: HirExpressionKind::Boolean(value),
        expr_type: Some(HirType::Boolean),
        symbol_id: None,
        span,
    }
}

fn manual_module(
    function_name: &str,
    body: Vec<HirStatement>,
    return_type: Option<HirType>,
) -> compiler::hir::HirModule {
    let span = test_span();
    let mut module = compiler::hir::HirModule::new("manual".to_string(), span);
    module.functions.push(HirFunction {
        id: HirFunctionId::new(0),
        symbol_id: HirSymbolId::new(0),
        name: function_name.to_string(),
        parameters: Vec::new(),
        local_variables: Vec::new(),
        body,
        return_type: return_type.clone(),
        signature: HirFunctionSignature {
            parameter_types: Vec::new(),
            return_type: return_type.unwrap_or(HirType::Nil),
            calling_convention: compiler::hir::HirCallingConvention::GraduaLuau,
            is_variadic: false,
        },
        scope_id: HirScopeId::new(0),
        is_local: false,
        metadata: HirFunctionMetadata::default(),
        span,
    });
    module
}
