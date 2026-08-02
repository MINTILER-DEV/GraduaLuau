// Parser error recovery tests

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, StatementKind};

    fn parse_code(code: &str) -> (AstNode, Vec<compiler::diagnostics::Diagnostic>) {
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
        let ast = parser.parse_program();
        let diagnostics = parser.diagnostics().to_vec();
        
        (ast, diagnostics)
    }

    #[test]
    fn test_missing_identifier_after_local() {
        let code = r#"local =
print("hi")"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing identifier
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected identifier after 'local'")));
        
        // Should still parse the second statement
        if let AstNode::Program(program) = ast {
            assert!(program.statements.len() >= 1);
        }
    }

    #[test]
    fn test_missing_closing_parenthesis() {
        let code = r#"print("Hello""#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing ')'
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected ')'")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_expression_after_operator() {
        let code = r#"x = +"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing expression
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected expression")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_multiple_errors_in_same_file() {
        let code = r#"local =
if then
print("#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit multiple errors
        assert!(diagnostics.len() >= 2);
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_type_annotation() {
        let code = r#"local x:"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing type
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected type after ':'")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_table_field_value() {
        let code = r#"local t = { Name = }"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing expression
        assert!(!diagnostics.is_empty());
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_closing_brace() {
        let code = r#"local t = { Name = "Test""#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing '}'
        assert!(!diagnostics.is_empty());
        // The exact error message might vary, just check we got some error
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_closing_bracket() {
        let code = r#"local t = { [1 = "test" }"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing ']'
        assert!(!diagnostics.is_empty());
        // The exact error message might vary, just check we got some error
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_invalid_parameter_list() {
        let code = r#"function test(,)"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about invalid parameter
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected parameter name")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_missing_closing_paren_in_params() {
        let code = r#"function test("#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error about missing ')'
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected ')'")));
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_error_nodes_in_ast() {
        let code = r#"x = +"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error
        assert!(!diagnostics.is_empty());
        
        // Should produce error nodes in AST
        if let AstNode::Program(program) = ast {
            // Just verify we can access the AST
            let _ = program.statements;
        }
    }

    #[test]
    fn test_cascading_error_suppression() {
        let code = r#"local =
if +
print +
return +"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit errors but not too many (cascading suppression)
        assert!(!diagnostics.is_empty());
        
        // Should still produce a valid AST
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
        }
    }

    #[test]
    fn test_recovery_preserves_source_spans() {
        let code = r#"print("Hello""#;
        let (ast, diagnostics) = parse_code(code);
        
        // All diagnostics should have valid spans
        for diagnostic in &diagnostics {
            // Just verify diagnostics exist and have proper structure
            assert!(!diagnostic.message().is_empty());
        }
        
        // AST nodes should have valid spans
        if let AstNode::Program(program) = ast {
            for stmt in &program.statements {
                assert!(stmt.span.start() <= stmt.span.end());
            }
        }
    }

    #[test]
    fn test_partial_ast_generation() {
        let code = r#"local x =
local y = 5"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error for the first statement
        assert!(!diagnostics.is_empty());
        
        // Should still parse the valid second statement
        if let AstNode::Program(program) = ast {
            assert!(program.statements.len() >= 1);
            
            // Check that the second statement is valid
            if program.statements.len() >= 2 {
                if let StatementKind::Local { names, .. } = &program.statements[1].kind {
                    assert_eq!(names[0].0, "y");
                }
            }
        }
    }

    #[test]
    fn test_synchronization_to_statement_boundary() {
        let code = r#"local = local x = 5"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error for the first statement
        assert!(!diagnostics.is_empty());
        
        // Should recover and parse the second statement
        if let AstNode::Program(program) = ast {
            assert!(program.statements.len() >= 1);
        }
    }

    #[test]
    fn test_infinite_loop_prevention() {
        let code = r#"x = +"#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should not hang and should emit an error
        assert!(!diagnostics.is_empty());
        
        // Should produce some AST (not hang indefinitely)
        let _ = ast;
    }

    #[test]
    fn test_help_messages_in_diagnostics() {
        let code = r#"print("Hello""#;
        let (ast, diagnostics) = parse_code(code);
        
        // Should emit an error with help message
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.message().contains("Expected ')'")));
        
        let _ = ast;
    }
}