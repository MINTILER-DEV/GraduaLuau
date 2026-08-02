// AST validation tests to ensure AST invariants

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use compiler::lexer::Lexer;
    use compiler::lexer::TokenKind;
    use compiler::parser::Parser;
    use compiler::source::SourceManager;
    use compiler::parser::ast_builder::{AstNode, ExpressionKind, StatementKind, TableField, InterpolatedStringPart, TypeExpressionKind};

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

    fn validate_ast_node(node: &AstNode) -> Result<(), String> {
        match node {
            AstNode::Program(program) => validate_program(program),
            AstNode::Statement(stmt) => validate_statement(stmt),
            AstNode::Expression(expr) => validate_expression(expr),
            AstNode::Error => Err("Error node found".to_string()),
        }
    }

    fn validate_program(program: &compiler::parser::ast_builder::Program) -> Result<(), String> {
        // Check that program has valid span
        if program.span.start() > program.span.end() {
            return Err("Program span is invalid".to_string());
        }

        // Validate all statements
        for stmt in &program.statements {
            validate_statement(stmt)?;
        }

        Ok(())
    }

    fn validate_statement(stmt: &compiler::parser::ast_builder::Statement) -> Result<(), String> {
        // Check that statement has valid span
        if stmt.span.start() > stmt.span.end() {
            return Err("Statement span is invalid".to_string());
        }

        // Validate statement kind
        match &stmt.kind {
            StatementKind::Empty => Ok(()),
            StatementKind::Expression(expr) => validate_expression(expr),
            StatementKind::Return(None) => Ok(()),
            StatementKind::Return(Some(values)) => {
                for expr in values {
                    validate_expression(expr)?;
                }
                Ok(())
            }
            StatementKind::Break => Ok(()),
            StatementKind::Continue => Ok(()),
            StatementKind::Local { names, initializers } => {
                // Validate names
                for (name, type_expr) in names {
                    if name.is_empty() {
                        return Err("Local variable name is empty".to_string());
                    }
                    if let Some(type_expr) = type_expr {
                        validate_type_expression(type_expr)?;
                    }
                }
                // Validate initializers
                for expr in initializers {
                    validate_expression(expr)?;
                }
                Ok(())
            }
            StatementKind::Assignment { targets, values, .. } => {
                for expr in targets {
                    validate_expression(expr)?;
                }
                for expr in values {
                    validate_expression(expr)?;
                }
                Ok(())
            }
            StatementKind::Function { body, .. } => {
                for stmt in body {
                    validate_statement(stmt)?;
                }
                Ok(())
            }
            StatementKind::TypeAlias { alias, .. } => {
                validate_type_expression(alias)
            }
            StatementKind::Error => Ok(()), // Error nodes are allowed during recovery
        }
    }

    fn validate_expression(expr: &compiler::parser::ast_builder::Expression) -> Result<(), String> {
        // Check that expression has valid span
        if expr.span.start() > expr.span.end() {
            return Err("Expression span is invalid".to_string());
        }

        // Validate expression kind
        match &expr.kind {
            ExpressionKind::Identifier(name) => {
                if name.is_empty() {
                    return Err("Identifier name is empty".to_string());
                }
                Ok(())
            }
            ExpressionKind::NumberLiteral(value) => {
                if value.is_empty() {
                    return Err("Number literal is empty".to_string());
                }
                Ok(())
            }
            ExpressionKind::StringLiteral(value) => {
                if value.is_empty() {
                    return Err("String literal is empty".to_string());
                }
                Ok(())
            }
            ExpressionKind::BooleanLiteral(_) => Ok(()),
            ExpressionKind::Nil => Ok(()),
            ExpressionKind::Unary { operand, .. } => {
                validate_expression(operand)
            }
            ExpressionKind::Binary { left, right, .. } => {
                validate_expression(left)?;
                validate_expression(right)
            }
            ExpressionKind::Call { callee, arguments } => {
                validate_expression(callee)?;
                for arg in arguments {
                    validate_expression(arg)?;
                }
                Ok(())
            }
            ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    validate_table_field(field)?;
                }
                Ok(())
            }
            ExpressionKind::MemberAccess { object, property } => {
                validate_expression(object)?;
                if property.is_empty() {
                    return Err("Property name is empty".to_string());
                }
                Ok(())
            }
            ExpressionKind::Index { object, index } => {
                validate_expression(object)?;
                validate_expression(index)
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                validate_expression(receiver)?;
                for arg in arguments {
                    validate_expression(arg)?;
                }
                Ok(())
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    validate_interpolated_string_part(part)?;
                }
                Ok(())
            }
            ExpressionKind::Error => Ok(()), // Error nodes are allowed during recovery
        }
    }

    fn validate_table_field(field: &TableField) -> Result<(), String> {
        match field {
            TableField::Named { key, value } => {
                if key.is_empty() {
                    return Err("Named field key is empty".to_string());
                }
                validate_expression(value)
            }
            TableField::Indexed { key, value } => {
                validate_expression(key)?;
                validate_expression(value)
            }
            TableField::Expression(expr) => {
                validate_expression(expr)
            }
        }
    }

    fn validate_interpolated_string_part(part: &InterpolatedStringPart) -> Result<(), String> {
        match part {
            InterpolatedStringPart::Text(_) => Ok(()),
            InterpolatedStringPart::Expression(expr) => {
                validate_expression(expr)
            }
        }
    }

    fn validate_type_expression(type_expr: &compiler::parser::ast_builder::TypeExpression) -> Result<(), String> {
        // Check that type expression has valid span
        if type_expr.span.start() > type_expr.span.end() {
            return Err("Type expression span is invalid".to_string());
        }

        match &type_expr.kind {
            TypeExpressionKind::Named(name) => {
                if name.is_empty() {
                    return Err("Type name is empty".to_string());
                }
                Ok(())
            }
            TypeExpressionKind::Optional(inner) => {
                validate_type_expression(inner)
            }
            TypeExpressionKind::Union(types) => {
                for typ in types {
                    validate_type_expression(typ)?;
                }
                Ok(())
            }
            TypeExpressionKind::Intersection(types) => {
                for typ in types {
                    validate_type_expression(typ)?;
                }
                Ok(())
            }
            TypeExpressionKind::Table(fields) => {
                for (_, typ, _) in fields {
                    validate_type_expression(typ)?;
                }
                Ok(())
            }
            TypeExpressionKind::Array(inner) => {
                validate_type_expression(inner)
            }
            TypeExpressionKind::Function { params, return_type } => {
                for param in params {
                    validate_type_expression(param)?;
                }
                validate_type_expression(return_type)
            }
            TypeExpressionKind::Tuple(types) => {
                for typ in types {
                    validate_type_expression(typ)?;
                }
                Ok(())
            }
            TypeExpressionKind::Variadic(inner) => {
                validate_type_expression(inner)
            }
            TypeExpressionKind::Parenthesized(inner) => {
                validate_type_expression(inner)
            }
        }
    }

    #[test]
    fn test_ast_validation_simple_program() {
        let code = r#"local x = 5
print(x)"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_complex_expressions() {
        let code = r#"local x = 1 + 2 * 3
local y = (4 + 5) / 2"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_function_calls() {
        let code = r#"print("Hello", 42)
player:Jump()
math.sqrt(16)"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_tables() {
        let code = r#"local t = { Name = "Test", [1] = "first" }
local empty = {}"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_type_annotations() {
        let code = r#"local x: number = 5
local y: string = "test"
local z: boolean = true"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_complex_types() {
        let code = r#"type Pair = { first: number, second: number }
local p: Pair = { first = 1, second = 2 }"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_union_types() {
        let code = r#"local x: number | string = 5"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_function_types() {
        let code = r#"type Callback = (number, string) -> boolean
local cb: Callback = function(a, b) return true end"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_nested_structures() {
        let code = r#"local data = {
    player = {
        name = "Test",
        stats = { health = 100, mana = 50 }
    }
}"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_method_calls() {
        let code = r#"player.Inventory:GetWeapon(1):Equip()"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_interpolated_strings() {
        let code = r#"local message = `Hello {name}, your score is {score}`"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_multiple_assignment() {
        let code = r#"x, y, z = 1, 2, 3"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_function_declarations() {
        let code = r#"function add(a: number, b: number): number
    return a + b
end

local function multiply(x: number, y: number): number
    return x * y
end"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_error_nodes() {
        let code = r#"local = 
x = 5"#;
        let ast = parse_program(code);
        
        // Should still validate even with error nodes
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_span_integrity() {
        let code = r#"local x = 5
print(x)
return x"#;
        let ast = parse_program(code);
        
        if let AstNode::Program(program) = &ast {
            // Check that all spans are valid
            for stmt in &program.statements {
                assert!(stmt.span.start() <= stmt.span.end(), 
                    "Statement span is invalid: start={}, end={}", stmt.span.start(), stmt.span.end());
            }
        }
    }

    #[test]
    fn test_ast_validation_no_empty_identifiers() {
        let code = r#"local x = 5
print("test")"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }

    #[test]
    fn test_ast_validation_no_empty_literals() {
        let code = r#"local x = 42
local y = "hello"
local z = true"#;
        let ast = parse_program(code);
        
        let result = validate_ast_node(&ast);
        assert!(result.is_ok(), "AST validation failed: {:?}", result);
    }
}