// Unit tests for individual parser functions

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, ExpressionKind, StatementKind, TableField};

    fn parse_expression(code: &str) -> compiler::parser::ast_builder::Expression {
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
        parser.parse_expression()
    }

    fn parse_statement(code: &str) -> compiler::parser::ast_builder::Statement {
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
        parser.parse_statement()
    }

    // Literal Tests
    #[test]
    fn test_parse_number_literal() {
        let expr = parse_expression("42");
        assert!(matches!(expr.kind, ExpressionKind::NumberLiteral(ref s) if s == "42"));
    }

    #[test]
    fn test_parse_string_literal() {
        let expr = parse_expression(r#""Hello""#);
        assert!(matches!(expr.kind, ExpressionKind::StringLiteral(ref s) if s == "Hello"));
    }

    #[test]
    fn test_parse_boolean_literal_true() {
        let expr = parse_expression("true");
        assert!(matches!(expr.kind, ExpressionKind::BooleanLiteral(true)));
    }

    #[test]
    fn test_parse_boolean_literal_false() {
        let expr = parse_expression("false");
        assert!(matches!(expr.kind, ExpressionKind::BooleanLiteral(false)));
    }

    #[test]
    fn test_parse_nil_literal() {
        let expr = parse_expression("nil");
        assert!(matches!(expr.kind, ExpressionKind::Nil));
    }

    // Operator Tests
    #[test]
    fn test_parse_unary_minus() {
        let expr = parse_expression("-5");
        if let ExpressionKind::Unary { operator, operand } = &expr.kind {
            assert_eq!(operator, "-");
            assert!(matches!(operand.kind, ExpressionKind::NumberLiteral(ref s) if s == "5"));
        } else {
            panic!("Expected unary expression");
        }
    }

    #[test]
    fn test_parse_unary_not() {
        let expr = parse_expression("not true");
        if let ExpressionKind::Unary { operator, operand } = &expr.kind {
            assert_eq!(operator, "not");
            assert!(matches!(operand.kind, ExpressionKind::BooleanLiteral(true)));
        } else {
            panic!("Expected unary expression");
        }
    }

    #[test]
    fn test_parse_binary_addition() {
        let expr = parse_expression("1 + 2");
        if let ExpressionKind::Binary { left, operator, right } = &expr.kind {
            assert_eq!(operator, "+");
            assert!(matches!(left.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
            assert!(matches!(right.kind, ExpressionKind::NumberLiteral(ref s) if s == "2"));
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn test_parse_binary_subtraction() {
        let expr = parse_expression("5 - 3");
        if let ExpressionKind::Binary { operator, .. } = &expr.kind {
            assert_eq!(operator, "-");
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn test_parse_binary_multiplication() {
        let expr = parse_expression("2 * 3");
        if let ExpressionKind::Binary { operator, .. } = &expr.kind {
            assert_eq!(operator, "*");
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn test_parse_binary_division() {
        let expr = parse_expression("10 / 2");
        if let ExpressionKind::Binary { operator, .. } = &expr.kind {
            assert_eq!(operator, "/");
        } else {
            panic!("Expected binary expression");
        }
    }

    // Operator Precedence Tests
    #[test]
    fn test_operator_precedence_multiplication_before_addition() {
        let expr = parse_expression("1 + 2 * 3");
        if let ExpressionKind::Binary { left, operator, right } = &expr.kind {
            assert_eq!(operator, "+");
            assert!(matches!(left.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
            if let ExpressionKind::Binary { left: inner_left, operator: inner_op, right: inner_right } = &right.kind {
                assert_eq!(inner_op, "*");
                assert!(matches!(inner_left.kind, ExpressionKind::NumberLiteral(ref s) if s == "2"));
                assert!(matches!(inner_right.kind, ExpressionKind::NumberLiteral(ref s) if s == "3"));
            } else {
                panic!("Expected multiplication on right side");
            }
        } else {
            panic!("Expected binary expression");
        }
    }

    #[test]
    fn test_operator_precedence_parentheses_override() {
        let expr = parse_expression("(1 + 2) * 3");
        if let ExpressionKind::Binary { left, operator, right } = &expr.kind {
            assert_eq!(operator, "*");
            if let ExpressionKind::Binary { left: inner_left, operator: inner_op, right: inner_right } = &left.kind {
                assert_eq!(inner_op, "+");
                assert!(matches!(inner_left.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
                assert!(matches!(inner_right.kind, ExpressionKind::NumberLiteral(ref s) if s == "2"));
            } else {
                panic!("Expected addition in parentheses");
            }
            assert!(matches!(right.kind, ExpressionKind::NumberLiteral(ref s) if s == "3"));
        } else {
            panic!("Expected binary expression");
        }
    }

    // Function Call Tests
    #[test]
    fn test_parse_function_call_no_args() {
        let expr = parse_expression("print()");
        if let ExpressionKind::Call { callee, arguments } = &expr.kind {
            assert!(matches!(callee.kind, ExpressionKind::Identifier(ref s) if s == "print"));
            assert!(arguments.is_empty());
        } else {
            panic!("Expected call expression");
        }
    }

    #[test]
    fn test_parse_function_call_with_args() {
        let expr = parse_expression("print(1, 2)");
        if let ExpressionKind::Call { callee, arguments } = &expr.kind {
            assert!(matches!(callee.kind, ExpressionKind::Identifier(ref s) if s == "print"));
            assert_eq!(arguments.len(), 2);
        } else {
            panic!("Expected call expression");
        }
    }

    // Table Constructor Tests
    #[test]
    fn test_parse_empty_table() {
        let expr = parse_expression("{}");
        if let ExpressionKind::TableConstructor(fields) = &expr.kind {
            assert!(fields.is_empty());
        } else {
            panic!("Expected table constructor");
        }
    }

    #[test]
    fn test_parse_table_with_named_field() {
        let expr = parse_expression("{ Name = \"Test\" }");
        if let ExpressionKind::TableConstructor(fields) = &expr.kind {
            assert_eq!(fields.len(), 1);
            if let TableField::Named { key, .. } = &fields[0] {
                assert_eq!(key, "Name");
            } else {
                panic!("Expected named field");
            }
        } else {
            panic!("Expected table constructor");
        }
    }

    #[test]
    fn test_parse_table_with_indexed_field() {
        let expr = parse_expression("{ [1] = \"first\" }");
        if let ExpressionKind::TableConstructor(fields) = &expr.kind {
            assert_eq!(fields.len(), 1);
            assert!(matches!(fields[0], TableField::Indexed { .. }));
        } else {
            panic!("Expected table constructor");
        }
    }

    // Member Access Tests
    #[test]
    fn test_parse_member_access() {
        let expr = parse_expression("player.Name");
        if let ExpressionKind::MemberAccess { object, property } = &expr.kind {
            assert!(matches!(object.kind, ExpressionKind::Identifier(ref s) if s == "player"));
            assert_eq!(property, "Name");
        } else {
            panic!("Expected member access");
        }
    }

    // Index Expression Tests
    #[test]
    fn test_parse_index_expression() {
        let expr = parse_expression("array[1]");
        if let ExpressionKind::Index { object, index } = &expr.kind {
            assert!(matches!(object.kind, ExpressionKind::Identifier(ref s) if s == "array"));
            assert!(matches!(index.kind, ExpressionKind::NumberLiteral(ref s) if s == "1"));
        } else {
            panic!("Expected index expression");
        }
    }

    // Method Call Tests
    #[test]
    fn test_parse_method_call() {
        let expr = parse_expression("player:Jump()");
        if let ExpressionKind::MethodCall { receiver, method, arguments } = &expr.kind {
            assert!(matches!(receiver.kind, ExpressionKind::Identifier(ref s) if s == "player"));
            assert_eq!(method, "Jump");
            assert!(arguments.is_empty());
        } else {
            panic!("Expected method call");
        }
    }

    // Statement Tests
    #[test]
    fn test_parse_local_statement() {
        let stmt = parse_statement("local x = 5");
        if let StatementKind::Local { names, initializers } = &stmt.kind {
            assert_eq!(names.len(), 1);
            assert_eq!(names[0].0, "x");
            assert_eq!(initializers.len(), 1);
        } else {
            panic!("Expected local statement");
        }
    }

    #[test]
    fn test_parse_local_with_type_annotation() {
        let stmt = parse_statement("local x: number = 5");
        if let StatementKind::Local { names, initializers } = &stmt.kind {
            assert_eq!(names.len(), 1);
            assert_eq!(names[0].0, "x");
            assert!(names[0].1.is_some()); // Has type annotation
            assert_eq!(initializers.len(), 1);
        } else {
            panic!("Expected local statement");
        }
    }

    #[test]
    fn test_parse_assignment_statement() {
        let stmt = parse_statement("x = 5");
        if let StatementKind::Assignment { targets, values, operator } = &stmt.kind {
            assert_eq!(targets.len(), 1);
            assert_eq!(values.len(), 1);
            assert_eq!(operator, "=");
        } else {
            panic!("Expected assignment statement");
        }
    }

    #[test]
    fn test_parse_compound_assignment() {
        let stmt = parse_statement("x += 5");
        if let StatementKind::Assignment { operator, .. } = &stmt.kind {
            assert_eq!(operator, "+=");
        } else {
            panic!("Expected assignment statement");
        }
    }

    #[test]
    fn test_parse_return_statement() {
        let stmt = parse_statement("return 5");
        if let StatementKind::Return(Some(values)) = &stmt.kind {
            assert_eq!(values.len(), 1);
        } else {
            panic!("Expected return statement");
        }
    }

    #[test]
    fn test_parse_return_multiple_values() {
        let stmt = parse_statement("return 1, 2, 3");
        if let StatementKind::Return(Some(values)) = &stmt.kind {
            assert_eq!(values.len(), 3);
        } else {
            panic!("Expected return statement");
        }
    }

    #[test]
    fn test_parse_break_statement() {
        let stmt = parse_statement("break");
        assert!(matches!(stmt.kind, StatementKind::Break));
    }

    #[test]
    fn test_parse_continue_statement() {
        let stmt = parse_statement("continue");
        assert!(matches!(stmt.kind, StatementKind::Continue));
    }

    // Type Expression Tests
    #[test]
    fn test_parse_named_type() {
        let stmt = parse_statement("local x: number = 5");
        if let StatementKind::Local { names, .. } = &stmt.kind {
            if let Some(type_expr) = &names[0].1 {
                // Just verify we got a type expression
                let _ = type_expr;
            } else {
                panic!("Expected type annotation");
            }
        } else {
            panic!("Expected local statement");
        }
    }

    // Complex Expression Tests
    #[test]
    fn test_parse_nested_function_calls() {
        let expr = parse_expression("foo()(bar)");
        if let ExpressionKind::Call { callee, .. } = &expr.kind {
            // Should be a call to the result of another call
            let _ = callee;
        } else {
            panic!("Expected nested call");
        }
    }

    #[test]
    fn test_parse_chained_member_access() {
        let expr = parse_expression("player.Inventory.Name");
        if let ExpressionKind::MemberAccess { object, property } = &expr.kind {
            assert_eq!(property, "Name");
            if let ExpressionKind::MemberAccess { object: inner_object, property: inner_property } = &object.kind {
                assert_eq!(inner_property, "Inventory");
                assert!(matches!(inner_object.kind, ExpressionKind::Identifier(ref s) if s == "player"));
            } else {
                panic!("Expected nested member access");
            }
        } else {
            panic!("Expected member access");
        }
    }

    #[test]
    fn test_parse_complex_expressions() {
        let expr = parse_expression("1 + 2 * 3 - 4 / 2");
        // Should respect operator precedence
        if let ExpressionKind::Binary { operator, .. } = &expr.kind {
            // Top level should be subtraction or addition
            assert!(operator == "+" || operator == "-");
        } else {
            panic!("Expected binary expression");
        }
    }
}