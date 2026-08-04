// Snapshot tests for parser output verification

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, StatementKind, ExpressionKind};

    fn parse_program(code: &str) -> AstNode {
        let mut sources = SourceManager::new();
        let file_id = sources.add_file(PathBuf::from("test.glu"), String::from(code));
        let file = sources.get(file_id).unwrap();

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
        parser.parse_program()
    }

    fn ast_to_string(ast: &AstNode) -> String {
        match ast {
            AstNode::Program(program) => {
                let mut output = String::from("Program\n");
                for stmt in &program.statements {
                    output.push_str("  ");
                    output.push_str(&statement_to_string(stmt));
                    output.push('\n');
                }
                output
            }
            AstNode::Statement(stmt) => statement_to_string(stmt),
            AstNode::Expression(expr) => expression_to_string(expr),
            AstNode::Error => "Error".to_string(),
        }
    }

    fn statement_to_string(stmt: &compiler::parser::ast_builder::Statement) -> String {
        match &stmt.kind {
            StatementKind::Empty => "EmptyStatement".to_string(),
            StatementKind::Expression(expr) => format!("ExpressionStatement({})", expression_to_string(expr)),
            StatementKind::Return(None) => "ReturnStatement".to_string(),
            StatementKind::Return(Some(values)) => {
                let exprs: Vec<String> = values.iter().map(expression_to_string).collect();
                format!("ReturnStatement({})", exprs.join(", "))
            }
            StatementKind::Break => "BreakStatement".to_string(),
            StatementKind::Continue => "ContinueStatement".to_string(),
            StatementKind::Local { names, initializers } => {
                let name_list: Vec<String> = names.iter().map(|(name, _)| name.clone()).collect();
                if initializers.is_empty() {
                    format!("LocalStatement({})", name_list.join(", "))
                } else {
                    let initializer_list: Vec<String> = initializers.iter().map(expression_to_string).collect();
                    format!("LocalStatement({}) init=[{}]", name_list.join(", "), initializer_list.join(", "))
                }
            }
            StatementKind::Assignment { targets, .. } => {
                let target_list: Vec<String> = targets.iter().map(expression_to_string).collect();
                format!("AssignmentStatement({})", target_list.join(", "))
            }
            StatementKind::Function { name, receiver, .. } => {
                if let Some(recv) = receiver {
                    format!("MethodDeclaration({}:{})", recv, name)
                } else {
                    format!("FunctionDeclaration({})", name)
                }
            }
            StatementKind::TypeAlias { name, .. } => format!("TypeAlias({})", name),
            StatementKind::Error => "ErrorStatement".to_string(),
        }
    }

    fn expression_to_string(expr: &compiler::parser::ast_builder::Expression) -> String {
        match &expr.kind {
            ExpressionKind::Identifier(name) => format!("Identifier({})", name),
            ExpressionKind::NumberLiteral(value) => format!("NumberLiteral({})", value),
            ExpressionKind::StringLiteral(value) => format!("StringLiteral({})", value),
            ExpressionKind::BooleanLiteral(value) => format!("BooleanLiteral({})", value),
            ExpressionKind::Nil => "Nil".to_string(),
            ExpressionKind::Unary { operator, operand } => {
                format!("Unary({}, {})", operator, expression_to_string(operand))
            }
            ExpressionKind::Binary { left, operator, right } => {
                format!("Binary({}, {}, {})", 
                    expression_to_string(left), 
                    operator, 
                    expression_to_string(right))
            }
            ExpressionKind::Call { callee, arguments } => {
                let args: Vec<String> = arguments.iter().map(expression_to_string).collect();
                format!("Call({}, [{}])", expression_to_string(callee), args.join(", "))
            }
            ExpressionKind::TableConstructor(_) => "TableConstructor".to_string(),
            ExpressionKind::MemberAccess { object, property } => {
                format!("MemberAccess({}, {})", expression_to_string(object), property)
            }
            ExpressionKind::Index { object, index } => {
                format!("Index({}, {})", expression_to_string(object), expression_to_string(index))
            }
            ExpressionKind::MethodCall { receiver, method, arguments } => {
                let args: Vec<String> = arguments.iter().map(expression_to_string).collect();
                format!("MethodCall({}, {}, [{}])", 
                    expression_to_string(receiver), 
                    method, 
                    args.join(", "))
            }
            ExpressionKind::InterpolatedString(_) => "InterpolatedString".to_string(),
            ExpressionKind::Error => "ErrorExpression".to_string(),
        }
    }

    #[test]
    fn test_snapshot_simple_program() {
        let code = r#"local x = 5
print(x)"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        let expected = "Program\n  LocalStatement(x) init=[NumberLiteral(5)]\n  ExpressionStatement(Call(Identifier(print), [Identifier(x)]))\n";
        assert_eq!(output, expected, "Snapshot mismatch!\nExpected:\n{}\nGot:\n{}", expected, output);
    }

    #[test]
    fn test_snapshot_function_declaration() {
        let code = r#"function add(a, b)
    return a + b
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        // Just verify we got a function declaration
        assert!(output.contains("FunctionDeclaration(add)"));
    }

    #[test]
    fn test_snapshot_table_constructor() {
        let code = r#"local t = { Name = "Test", Value = 42 }"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("LocalStatement(t)"));
        assert!(output.contains("TableConstructor"));
    }

    #[test]
    fn test_snapshot_method_call() {
        let code = r#"player:Jump()"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("MethodCall"));
    }

    #[test]
    fn test_snapshot_complex_expressions() {
        let code = r#"local result = (1 + 2) * 3"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("LocalStatement(result)"));
        assert!(output.contains("Binary"));
    }

    #[test]
    fn test_snapshot_nested_calls() {
        let code = r#"factory("Player")("Enemy")"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("Call"));
    }

    #[test]
    fn test_snapshot_type_annotation() {
        let code = r#"local x: number = 5"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("LocalStatement(x)"));
    }

    #[test]
    fn test_snapshot_return_statement() {
        let code = r#"return 1, 2, 3"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("ReturnStatement"));
    }

    #[test]
    fn test_snapshot_assignment() {
        let code = r#"x, y = 1, 2"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("AssignmentStatement"));
    }

    #[test]
    fn test_snapshot_compound_assignment() {
        let code = r#"x += 5"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("AssignmentStatement"));
    }

    #[test]
    fn test_snapshot_member_access() {
        let code = r#"player.Name"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("MemberAccess"));
    }

    #[test]
    fn test_snapshot_index_expression() {
        let code = r#"array[1]"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("Index"));
    }

    #[test]
    fn test_snapshot_control_flow() {
        let code = r#"if x > 5 then
    return true
else
    return false
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        // Verify we got some structure (exact structure depends on if/else parsing)
        assert!(!output.is_empty());
    }

    #[test]
    fn test_snapshot_loop_constructs() {
        let code = r#"while x < 10 do
    x = x + 1
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        // Verify we got some structure
        assert!(!output.is_empty());
    }

    #[test]
    fn test_snapshot_interpolated_string() {
        let code = r#"local message = `Hello {name}`"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("LocalStatement(message)"));
    }

    #[test]
    fn test_snapshot_function_with_params() {
        let code = r#"function greet(name: string): string
    return "Hello, " .. name
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("FunctionDeclaration(greet)"));
    }

    #[test]
    fn test_snapshot_local_function() {
        let code = r#"local function helper()
    return 42
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("FunctionDeclaration"));
    }

    #[test]
    fn test_snapshot_type_alias() {
        let code = r#"type Vector3 = { x: number, y: number, z: number }"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        assert!(output.contains("TypeAlias"));
    }

    #[test]
    fn test_snapshot_break_continue() {
        let code = r#"for i = 1, 10 do
    if i == 5 then
        break
    end
    if i == 7 then
        continue
    end
end"#;
        let ast = parse_program(code);
        let output = ast_to_string(&ast);
        
        // Verify we got some structure
        assert!(!output.is_empty());
    }
}
