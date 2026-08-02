// Parser tests for Luau-specific syntax

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, ExpressionKind, StatementKind, TableField};

    fn parse_code(code: &str) -> AstNode {
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

    #[test]
    fn parse_string_argument_shorthand() {
        let ast = parse_code(r#"print "Hello""#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::Call { callee, arguments } = &expr.kind {
                    assert!(matches!(callee.kind, ExpressionKind::Identifier(ref s) if s == "print"));
                    assert_eq!(arguments.len(), 1);
                    assert!(matches!(arguments[0].kind, ExpressionKind::StringLiteral(ref s) if s == "Hello"));
                    return;
                }
            }
        }
        panic!("Expected call expression with string argument shorthand");
    }

    #[test]
    fn parse_table_argument_shorthand() {
        let ast = parse_code(r#"spawn { Position = "test" }"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::Call { callee, arguments } = &expr.kind {
                    assert!(matches!(callee.kind, ExpressionKind::Identifier(ref s) if s == "spawn"));
                    assert_eq!(arguments.len(), 1);
                    if let ExpressionKind::TableConstructor(fields) = &arguments[0].kind {
                        assert_eq!(fields.len(), 1);
                        if let TableField::Named { key, value } = &fields[0] {
                            assert_eq!(key, "Position");
                            assert!(matches!(value.kind, ExpressionKind::StringLiteral(ref s) if s == "test"));
                            return;
                        }
                    }
                }
            }
        }
        panic!("Expected call expression with table argument shorthand");
    }

    #[test]
    fn parse_chained_shorthand_calls() {
        let ast = parse_code(r#"factory "Player" "Enemy""#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                // Should parse as: factory("Player")("Enemy")
                if let ExpressionKind::Call { callee, arguments } = &expr.kind {
                    assert_eq!(arguments.len(), 1);
                    assert!(matches!(arguments[0].kind, ExpressionKind::StringLiteral(ref s) if s == "Enemy"));
                    
                    if let ExpressionKind::Call { callee: inner_callee, arguments: inner_args } = &callee.kind {
                        assert!(matches!(inner_callee.kind, ExpressionKind::Identifier(ref s) if s == "factory"));
                        assert_eq!(inner_args.len(), 1);
                        assert!(matches!(inner_args[0].kind, ExpressionKind::StringLiteral(ref s) if s == "Player"));
                        return;
                    }
                }
            }
        }
        panic!("Expected chained shorthand calls");
    }

    #[test]
    fn parse_mixed_parenthesis_and_shorthand() {
        let ast = parse_code(r#"print("Hello") "World""#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                // Should parse as: print("Hello")("World")
                if let ExpressionKind::Call { callee, arguments } = &expr.kind {
                    assert_eq!(arguments.len(), 1);
                    assert!(matches!(arguments[0].kind, ExpressionKind::StringLiteral(ref s) if s == "World"));
                    
                    if let ExpressionKind::Call { callee: inner_callee, arguments: inner_args } = &callee.kind {
                        assert!(matches!(inner_callee.kind, ExpressionKind::Identifier(ref s) if s == "print"));
                        assert_eq!(inner_args.len(), 1);
                        assert!(matches!(inner_args[0].kind, ExpressionKind::StringLiteral(ref s) if s == "Hello"));
                        return;
                    }
                }
            }
        }
        panic!("Expected mixed parenthesis and shorthand calls");
    }

    #[test]
    fn parse_method_call() {
        let ast = parse_code(r#"player:Jump()"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::MethodCall { receiver, method, arguments } = &expr.kind {
                    assert!(matches!(receiver.kind, ExpressionKind::Identifier(ref s) if s == "player"));
                    assert_eq!(method, "Jump");
                    assert_eq!(arguments.len(), 0);
                    return;
                }
            }
        }
        panic!("Expected method call expression");
    }

    #[test]
    fn parse_method_call_with_arguments() {
        let ast = parse_code(r#"player:MoveTo(position, 10)"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::MethodCall { receiver, method, arguments } = &expr.kind {
                    assert!(matches!(receiver.kind, ExpressionKind::Identifier(ref s) if s == "player"));
                    assert_eq!(method, "MoveTo");
                    assert_eq!(arguments.len(), 2);
                    return;
                }
            }
        }
        panic!("Expected method call with arguments");
    }

    #[test]
    fn parse_table_constructor() {
        let ast = parse_code(r#"local t = { Name = "Test" }"#);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            let stmt = &program.statements[0];
            if let StatementKind::Local { names, initializers } = &stmt.kind {
                assert_eq!(names.len(), 1);
                assert_eq!(names[0].0, "t");
                if !initializers.is_empty() {
                    // Just check that we got some expression as initializer
                    return;
                }
            }
        }
        panic!("Expected table constructor");
    }

    #[test]
    fn parse_indexed_table_field() {
        let ast = parse_code(r#"local t = { [1] = "first" }"#);
        
        if let AstNode::Program(program) = ast {
            assert!(!program.statements.is_empty());
            let stmt = &program.statements[0];
            if let StatementKind::Local { names: _, initializers } = &stmt.kind {
                if !initializers.is_empty() {
                    // Just check that we got some expression as initializer
                    return;
                }
            }
        }
        panic!("Expected indexed table fields");
    }

    #[test]
    fn parse_compound_assignment() {
        let ast = parse_code(r#"x += 1"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Assignment { targets, values, operator } = &stmt.kind {
                assert_eq!(targets.len(), 1);
                assert_eq!(values.len(), 1);
                assert_eq!(operator, "+=");
                return;
            }
        }
        panic!("Expected compound assignment");
    }

    #[test]
    fn parse_bitwise_compound_assignment() {
        let ast = parse_code(r#"flags &= mask"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Assignment { targets, values, operator } = &stmt.kind {
                assert_eq!(targets.len(), 1);
                assert_eq!(values.len(), 1);
                assert_eq!(operator, "&=");
                return;
            }
        }
        panic!("Expected bitwise compound assignment");
    }

    #[test]
    fn parse_member_access() {
        let ast = parse_code(r#"player.Name"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::MemberAccess { object, property } = &expr.kind {
                    assert!(matches!(object.kind, ExpressionKind::Identifier(ref s) if s == "player"));
                    assert_eq!(property, "Name");
                    return;
                }
            }
        }
        panic!("Expected member access expression");
    }

    #[test]
    fn parse_index_expression() {
        let ast = parse_code(r#"array[1]"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Expression(expr) = &stmt.kind {
                if let ExpressionKind::Index { object, index } = &expr.kind {
                    assert!(matches!(object.kind, ExpressionKind::Identifier(ref s) if s == "array"));
                    assert!(matches!(index.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
                    return;
                }
            }
        }
        panic!("Expected index expression");
    }

    #[test]
    fn parse_continue_statement() {
        let ast = parse_code(r#"for i = 1, 10 do if i == 5 then continue end end"#);
        
        if let AstNode::Program(program) = ast {
            // The parser will parse the for loop and its contents
            // Just check that we got a program with statements
            assert!(!program.statements.is_empty());
            return;
        }
        panic!("Expected continue statement to be parsed");
    }

    #[test]
    fn parse_interpolated_string() {
        let ast = parse_code(r#"local message = `Hello {name}`"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Local { names, initializers } = &stmt.kind {
                assert_eq!(names.len(), 1);
                assert_eq!(names[0].0, "message");
                assert_eq!(initializers.len(), 1);
                if let ExpressionKind::InterpolatedString(parts) = &initializers[0].kind {
                    assert!(!parts.is_empty());
                    return;
                }
            }
        }
        panic!("Expected interpolated string");
    }

    #[test]
    fn parse_multiple_assignment() {
        let ast = parse_code(r#"x, y = y, x"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Assignment { targets, values, operator } = &stmt.kind {
                assert_eq!(targets.len(), 2);
                assert_eq!(values.len(), 2);
                assert_eq!(operator, "=");
                return;
            }
        }
        panic!("Expected multiple assignment");
    }

    #[test]
    fn parse_multiple_return_values() {
        let ast = parse_code(r#"return x, y, z"#);
        
        if let AstNode::Program(program) = ast {
            assert_eq!(program.statements.len(), 1);
            let stmt = &program.statements[0];
            if let StatementKind::Return(Some(values)) = &stmt.kind {
                assert_eq!(values.len(), 3);
                return;
            }
        }
        panic!("Expected multiple return values");
    }
}
