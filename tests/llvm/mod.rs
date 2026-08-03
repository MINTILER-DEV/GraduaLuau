use compiler::llvm::{LlvmModule, LlvmStage, LlvmVerifier};
use compiler::mir::MirStage;
use compiler::hir::HirStage;
use compiler::lexer::Lexer;
use compiler::parser::Parser;
use compiler::semantic;
use compiler::source::SourceManager;
use std::path::PathBuf;

fn compile_to_llvm(source: &str) -> Result<LlvmModule, String> {
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
    
    if parser.diagnostics().iter().any(|d| d.severity().is_error()) {
        return Err(format!("Parser errors: {:?}", parser.diagnostics()));
    }
    
    let (constant_diagnostics, _constant_results) = semantic::evaluate_constants(&ast);
    if !constant_diagnostics.is_empty() && constant_diagnostics.iter().any(|d| d.severity().is_error()) {
        return Err(format!("Constant evaluation errors: {:?}", constant_diagnostics));
    }
    
    let (module_diagnostics, _resolved_modules) = semantic::resolve_modules(&mut sources, file_id, &ast);
    if !module_diagnostics.is_empty() && module_diagnostics.iter().any(|d| d.severity().is_error()) {
        return Err(format!("Module resolution errors: {:?}", module_diagnostics));
    }
    
    let semantic_result = semantic::analyze(&ast);
    if !semantic_result.diagnostics.is_empty() && semantic_result.diagnostics.iter().any(|d| d.severity().is_error()) {
        return Err(format!("Semantic analysis errors: {:?}", semantic_result.diagnostics));
    }
    
    let hir = HirStage::lower(&ast).map_err(|e| e.to_string())?;
    let mir = MirStage::lower(&hir).map_err(|e| e.to_string())?;
    LlvmStage::generate(&mir).map_err(|e| e.to_string())
}

#[test]
fn test_empty_module() {
    let source = "";
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("target triple"));
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
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("define"));
    assert!(llvm.ir.contains("add"));
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
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("add"));
    assert!(llvm.ir.contains("mul"));
}

#[test]
fn test_llvm_verifier() {
    let source = r#"
function simple()
    return 42
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    
    let verification_result = LlvmVerifier::verify(&llvm.ir);
    assert!(verification_result.is_ok());
}

#[test]
fn test_constants() {
    let source = r#"
function constants()
    local int_val = 42
    local bool_val = true
    return int_val
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("42"));
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
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("define"));
}

#[test]
fn test_return_statements() {
    let source = r#"
function test_return()
    local x = 5
    return x
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("ret"));
}

#[test]
fn test_runtime_declarations() {
    let source = r#"
function main()
    return 0
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("declare"));
    assert!(llvm.ir.contains("glua_print"));
}

#[test]
fn test_top_level_print_generates_main() {
    let source = r#"print "Hello, GraduaLuau!""#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("define"));
    assert!(llvm.ir.contains("@main"));
    assert!(llvm.ir.contains("glua_print"));
    assert!(llvm.ir.contains("Hello, GraduaLuau"));
}

#[test]
fn test_table_operations() {
    let source = r#"
function create_table()
    local t = {}
    return t
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("glua_table_new"));
}

#[test]
fn test_module_structure() {
    let source = r#"
function example()
    return 0
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("Module ID"));
    assert!(llvm.ir.contains("target triple"));
    assert!(llvm.ir.contains("target datalayout"));
}

#[test]
fn test_comparison_operations() {
    let source = r#"
function compare()
    local a = 5
    local b = 10
    local result = false
    return result
end
"#;
    let result = compile_to_llvm(source);
    assert!(result.is_ok());
    let llvm = result.unwrap();
    assert!(llvm.ir.contains("define"));
}
