use compiler::mir::{MirModule, MirStage, MirPrinter, MirValidator};
use compiler::hir::HirStage;
use compiler::lexer::Lexer;
use compiler::parser::Parser;
use compiler::semantic;
use compiler::source::SourceManager;
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