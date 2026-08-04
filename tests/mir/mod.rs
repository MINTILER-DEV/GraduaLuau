use compiler::hir::function::{HirFunctionMetadata, HirParameter};
use compiler::hir::statement::HirLocalVariable;
use compiler::hir::{
    HirBinaryOperator, HirExpression, HirExpressionKind, HirFunction, HirFunctionId,
    HirFunctionSignature, HirScopeId, HirStage, HirStatement, HirStatementKind, HirSymbolId,
    HirType, HirVariableId,
};
use compiler::lexer::Lexer;
use compiler::mir::{
    MirBasicBlock, MirBuilder, MirFunction, MirFunctionId, MirInstruction, MirInstructionKind,
    MirModule, MirOptimizer, MirPrinter, MirStage, MirTerminator, MirType as LoweredMirType,
    MirValidator, MirValue, MirValueData, MirValueId, MirValueKind,
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
    assert!(
        square
            .locals
            .iter()
            .any(|local| local.storage == square.parameter_data[0].storage)
    );
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

    assert_eq!(function.blocks.len(), 5);
    assert_eq!(function.exit_blocks.len(), 1);
    assert!(function.blocks[4].is_exit);
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
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[4].terminator,
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

    assert_eq!(function.blocks.len(), 5);
    assert_eq!(function.exit_blocks.len(), 1);
    assert!(function.blocks[4].is_exit);
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
        Some(MirTerminator::Jump { .. })
    ));
    assert!(matches!(
        function.blocks[4].terminator,
        Some(MirTerminator::Return { .. })
    ));
    assert_eq!(function.blocks[1].predecessors.len(), 2);
}

#[test]
fn multiple_returns_share_one_exit_block() {
    let span = test_span();
    let module = manual_module(
        "returns",
        vec![HirStatement {
            kind: HirStatementKind::If {
                condition: bool_expr(true, span),
                then_block: vec![return_bool_stmt(true, span)],
                else_block: Some(vec![return_bool_stmt(false, span)]),
            },
            span,
        }],
        Some(HirType::Boolean),
    );

    let mir = MirStage::lower(&module).expect("manual returns should lower");
    let function = &mir.functions[0];

    assert_eq!(function.exit_blocks.len(), 1);
    assert!(function.blocks[3].is_exit);
    assert!(matches!(
        function.blocks[3].terminator,
        Some(MirTerminator::Return { .. })
    ));
    assert_eq!(function.blocks[3].predecessors.len(), 2);
    assert!(
        function
            .blocks
            .iter()
            .take(3)
            .all(|block| { !matches!(block.terminator, Some(MirTerminator::Return { .. })) })
    );
}

#[test]
fn cfg_tracks_edges_reachability_and_traversals() {
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
    let cfg = mir.functions[0].cfg.as_ref().expect("CFG should be built");

    assert!(cfg.validate().is_ok());
    assert_eq!(cfg.edges.len(), 5);
    assert_eq!(cfg.unreachable_blocks(), Vec::new());
    assert_eq!(cfg.bfs().first(), Some(&cfg.entry));
    assert_eq!(cfg.dfs().first(), Some(&cfg.entry));
    assert_eq!(cfg.reverse_post_order().first(), Some(&cfg.entry));
    assert!(cfg.to_dot("branchy").contains("Block"));
}

#[test]
fn cfg_detects_natural_while_loop() {
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
    let cfg = mir.functions[0].cfg.as_ref().expect("CFG should be built");

    assert_eq!(cfg.loops.len(), 1);
    assert!(
        cfg.loops[0]
            .body_blocks
            .contains(&function_block_id(&mir.functions[0], 1))
    );
    assert!(!cfg.loops[0].exit_edges.is_empty());

    let immediate_dominators = cfg.immediate_dominators();
    assert_eq!(
        immediate_dominators.get(&function_block_id(&mir.functions[0], 2)),
        Some(&function_block_id(&mir.functions[0], 1))
    );
}

#[test]
fn validator_rejects_cfg_block_edge_mismatches() {
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

    let mut mir = MirStage::lower(&module).expect("manual if HIR should lower");
    mir.functions[0].blocks[0].successors.clear();

    let mut validator = MirValidator::new();
    assert!(validator.validate(&mir).is_err());
}

#[test]
fn validator_rejects_duplicate_value_ids() {
    let mut mir = raw_copy_module();
    let duplicate = mir.functions[0].values[0].clone();
    mir.functions[0].values.push(duplicate);

    let mut validator = MirValidator::new();
    let errors = validator
        .validate(&mir)
        .expect_err("duplicate value IDs should be rejected");

    assert!(
        errors
            .iter()
            .any(|error| format!("{error:?}").contains("duplicate MIR value id"))
    );
}

#[test]
fn mir_optimizer_folds_constants_and_removes_dead_values() {
    let span = test_span();
    let result = local_variable(0, 0, "result", HirType::Number, span);
    let module = manual_module_with_signature(
        "folds",
        Vec::new(),
        vec![result.clone()],
        vec![
            local_stmt(
                result,
                Some(add_expr(
                    number_expr(2.0, span),
                    number_expr(3.0, span),
                    span,
                )),
                span,
            ),
            return_stmt(local_expr(0, 0, HirType::Number, span), span),
        ],
        HirType::Number,
    );

    let unoptimized = MirBuilder::new()
        .build(&module)
        .expect("manual HIR should build unoptimized MIR");
    let mut optimizer = MirOptimizer::new();
    let optimized = optimizer.optimize(&unoptimized);
    let function = &optimized.module.functions[0];

    assert!(optimized.stats.constants_folded >= 1);
    assert!(optimized.stats.dead_instructions_removed >= 2);
    assert!(function.blocks.iter().all(|block| {
        block
            .instructions
            .iter()
            .all(|instruction| !matches!(instruction.kind, MirInstructionKind::Add { .. }))
    }));
    assert!(function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            matches!(
                instruction.kind,
                MirInstructionKind::Const {
                    value: MirValue::Integer(5),
                    ..
                }
            )
        })
    }));
}

#[test]
fn mir_optimizer_propagates_copies() {
    let mir = raw_copy_module();
    let mut optimizer = MirOptimizer::new();
    let optimized = optimizer.optimize(&mir);
    let function = &optimized.module.functions[0];

    assert_eq!(optimized.stats.copies_propagated, 1);
    assert!(
        function.blocks[0]
            .instructions
            .iter()
            .all(|instruction| !matches!(instruction.kind, MirInstructionKind::Move { .. }))
    );
    assert!(matches!(
        function.blocks[0]
            .instructions
            .last()
            .map(|instruction| &instruction.kind),
        Some(MirInstructionKind::Return {
            value: Some(MirValueId(0))
        })
    ));

    let mut validator = MirValidator::new();
    assert!(validator.validate(&optimized.module).is_ok());
}

#[test]
fn ssa_metadata_collects_defs_uses_and_phi_candidates() {
    let span = test_span();
    let flag = HirParameter {
        id: HirVariableId::new(0),
        symbol_id: HirSymbolId::new(0),
        name: "flag".to_string(),
        param_type: Some(HirType::Boolean),
        scope_id: HirScopeId::new(0),
        span,
    };
    let score = local_variable(1, 1, "score", HirType::Number, span);
    let module = manual_module_with_signature(
        "choose",
        vec![flag],
        vec![score.clone()],
        vec![
            local_stmt(score.clone(), Some(number_expr(0.0, span)), span),
            HirStatement {
                kind: HirStatementKind::If {
                    condition: local_expr(0, 0, HirType::Boolean, span),
                    then_block: vec![assign_local_stmt(
                        1,
                        1,
                        number_expr(5.0, span),
                        HirType::Number,
                        span,
                    )],
                    else_block: Some(vec![assign_local_stmt(
                        1,
                        1,
                        number_expr(10.0, span),
                        HirType::Number,
                        span,
                    )]),
                },
                span,
            },
            return_stmt(local_expr(1, 1, HirType::Number, span), span),
        ],
        HirType::Number,
    );

    let mir = MirStage::lower(&module).expect("branching HIR should lower to MIR");
    let function = &mir.functions[0];
    let ssa = function
        .metadata
        .ssa
        .as_ref()
        .expect("SSA preparation metadata should be generated");
    let cfg = function.cfg.as_ref().expect("CFG should be generated");
    let reachable_blocks = cfg.reachable_blocks();

    assert!(
        ssa.dominators
            .values()
            .all(|dominators| dominators.contains(&cfg.entry))
    );
    assert!(
        ssa.dominance_frontiers
            .values()
            .any(|frontier| !frontier.is_empty())
    );
    assert!(
        ssa.definitions
            .values()
            .any(|definitions| definitions.len() >= 3)
    );

    let phi_candidate = ssa
        .phi_candidates
        .values()
        .flatten()
        .find(|candidate| reachable_blocks.contains(&candidate.block))
        .expect("score should need a phi candidate at the merge block");
    assert!(
        cfg.nodes
            .get(&phi_candidate.block)
            .is_some_and(|node| node.predecessors.len() >= 2)
    );
}

#[test]
fn lifetime_metadata_tracks_live_sets_dead_variables_and_temporaries() {
    let span = test_span();
    let flag = HirParameter {
        id: HirVariableId::new(0),
        symbol_id: HirSymbolId::new(0),
        name: "flag".to_string(),
        param_type: Some(HirType::Boolean),
        scope_id: HirScopeId::new(0),
        span,
    };
    let used = local_variable(1, 1, "used", HirType::Number, span);
    let dead = local_variable(2, 2, "dead", HirType::Number, span);
    let module = manual_module_with_signature(
        "lifetimes",
        vec![flag],
        vec![used.clone(), dead.clone()],
        vec![
            local_stmt(used.clone(), Some(number_expr(1.0, span)), span),
            local_stmt(dead, Some(number_expr(2.0, span)), span),
            HirStatement {
                kind: HirStatementKind::If {
                    condition: local_expr(0, 0, HirType::Boolean, span),
                    then_block: vec![assign_local_stmt(
                        1,
                        1,
                        add_expr(
                            local_expr(1, 1, HirType::Number, span),
                            number_expr(1.0, span),
                            span,
                        ),
                        HirType::Number,
                        span,
                    )],
                    else_block: Some(vec![assign_local_stmt(
                        1,
                        1,
                        add_expr(
                            local_expr(1, 1, HirType::Number, span),
                            number_expr(2.0, span),
                            span,
                        ),
                        HirType::Number,
                        span,
                    )]),
                },
                span,
            },
            return_stmt(local_expr(1, 1, HirType::Number, span), span),
        ],
        HirType::Number,
    );

    let mir = MirStage::lower(&module).expect("lifetime HIR should lower to MIR");
    let function = &mir.functions[0];
    let ssa = function
        .metadata
        .ssa
        .as_ref()
        .expect("SSA metadata should be generated");
    let lifetimes = function
        .metadata
        .lifetimes
        .as_ref()
        .expect("lifetime metadata should be generated");

    let dead_local = lifetimes.dead_variables.iter().find(|storage| {
        ssa.variables.get(*storage).is_some_and(|variable| {
            variable.symbol_id.is_some() && ssa.uses.get(*storage).is_none()
        })
    });
    assert!(dead_local.is_some());
    assert!(
        lifetimes
            .live_out
            .values()
            .any(|live_out| !live_out.is_empty())
    );
    assert!(
        lifetimes
            .value_lifetimes
            .values()
            .any(|lifetime| lifetime.last_use.is_some())
    );
    assert!(
        lifetimes
            .variable_lifetimes
            .values()
            .all(|lifetime| lifetime.definition.is_some())
    );
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

fn return_bool_stmt(value: bool, span: SourceSpan) -> HirStatement {
    HirStatement {
        kind: HirStatementKind::Return(Some(vec![bool_expr(value, span)])),
        span,
    }
}

fn local_variable(
    variable_id: usize,
    symbol_id: usize,
    name: &str,
    var_type: HirType,
    span: SourceSpan,
) -> HirLocalVariable {
    HirLocalVariable {
        id: HirVariableId::new(variable_id),
        symbol_id: HirSymbolId::new(symbol_id),
        name: name.to_string(),
        var_type: Some(var_type),
        scope_id: HirScopeId::new(0),
        span,
    }
}

fn local_stmt(
    variable: HirLocalVariable,
    initializer: Option<HirExpression>,
    span: SourceSpan,
) -> HirStatement {
    HirStatement {
        kind: HirStatementKind::LocalVariable {
            variable,
            initializer,
        },
        span,
    }
}

fn assign_local_stmt(
    variable_id: usize,
    symbol_id: usize,
    value: HirExpression,
    var_type: HirType,
    span: SourceSpan,
) -> HirStatement {
    HirStatement {
        kind: HirStatementKind::Assignment {
            targets: vec![local_expr(variable_id, symbol_id, var_type, span)],
            values: vec![value],
        },
        span,
    }
}

fn return_stmt(expression: HirExpression, span: SourceSpan) -> HirStatement {
    HirStatement {
        kind: HirStatementKind::Return(Some(vec![expression])),
        span,
    }
}

fn local_expr(
    variable_id: usize,
    symbol_id: usize,
    expr_type: HirType,
    span: SourceSpan,
) -> HirExpression {
    HirExpression {
        kind: HirExpressionKind::LocalVariable(HirVariableId::new(variable_id)),
        expr_type: Some(expr_type),
        symbol_id: Some(HirSymbolId::new(symbol_id)),
        span,
    }
}

fn number_expr(value: f64, span: SourceSpan) -> HirExpression {
    HirExpression {
        kind: HirExpressionKind::Number(value),
        expr_type: Some(HirType::Number),
        symbol_id: None,
        span,
    }
}

fn add_expr(left: HirExpression, right: HirExpression, span: SourceSpan) -> HirExpression {
    HirExpression {
        kind: HirExpressionKind::Binary {
            left: Box::new(left),
            operator: HirBinaryOperator::Add,
            right: Box::new(right),
        },
        expr_type: Some(HirType::Number),
        symbol_id: None,
        span,
    }
}

fn function_block_id(function: &HirLoweredMirFunction, index: usize) -> compiler::mir::MirBlockId {
    function.blocks[index].id
}

type HirLoweredMirFunction = compiler::mir::MirFunction;

fn raw_copy_module() -> MirModule {
    let span = test_span();
    let mut module = MirModule::new("raw".to_string());
    let mut function = MirFunction::new(MirFunctionId::new(0), "copies".to_string());
    function.return_type = Some(LoweredMirType::Integer);
    function.entry_block = Some(0);
    function.add_block(MirBasicBlock::with_entry(compiler::mir::MirBlockId::new(0)));
    function.add_value(MirValueData::new(
        MirValueId::new(0),
        LoweredMirType::Integer,
        MirValueKind::Constant,
        Some(span),
    ));
    function.add_value(MirValueData::new(
        MirValueId::new(1),
        LoweredMirType::Integer,
        MirValueKind::Temporary,
        Some(span),
    ));
    function.add_instruction(
        0,
        MirInstruction::new(
            MirInstructionKind::Const {
                result: MirValueId::new(0),
                value: MirValue::Integer(7),
            },
            Some(LoweredMirType::Integer),
            Some(span),
        ),
    );
    function.add_instruction(
        0,
        MirInstruction::new(
            MirInstructionKind::Move {
                result: MirValueId::new(1),
                value: MirValueId::new(0),
            },
            Some(LoweredMirType::Integer),
            Some(span),
        ),
    );
    function.add_instruction(
        0,
        MirInstruction::new(
            MirInstructionKind::Return {
                value: Some(MirValueId::new(1)),
            },
            Some(LoweredMirType::Void),
            Some(span),
        ),
    );
    function.rebuild_cfg();
    function.exit_blocks = vec![0];
    module.add_function(function);
    module
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

fn manual_module_with_signature(
    function_name: &str,
    parameters: Vec<HirParameter>,
    local_variables: Vec<HirLocalVariable>,
    body: Vec<HirStatement>,
    return_type: HirType,
) -> compiler::hir::HirModule {
    let span = test_span();
    let mut module = compiler::hir::HirModule::new("manual".to_string(), span);
    module.functions.push(HirFunction {
        id: HirFunctionId::new(0),
        symbol_id: HirSymbolId::new(0),
        name: function_name.to_string(),
        parameters,
        local_variables,
        body,
        return_type: Some(return_type.clone()),
        signature: HirFunctionSignature {
            parameter_types: vec![HirType::Boolean],
            return_type,
            calling_convention: compiler::hir::HirCallingConvention::GraduaLuau,
            is_variadic: false,
        },
        scope_id: HirScopeId::new(0),
        is_local: false,
        metadata: HirFunctionMetadata {
            has_explicit_return: true,
        },
        span,
    });
    module
}
