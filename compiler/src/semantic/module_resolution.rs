use std::collections::HashMap;
use std::path::PathBuf;

use crate::diagnostics::Diagnostic;
use crate::lexer::Lexer;
use crate::parser::ast_builder::{AstNode, Expression, ExpressionKind, InterpolatedStringPart, Statement, StatementKind, TableField};
use crate::parser::Parser;
use crate::source::{FileId, SourceManager, SourceSpan};

#[derive(Debug, Clone)]
pub struct ModuleMetadata {
    pub file_id: FileId,
    pub path: PathBuf,
    pub module_name: String,
    pub dependencies: Vec<FileId>,
    pub exports: Vec<String>,
    pub ast: AstNode,
}

#[derive(Debug)]
pub struct ModuleResolver<'a> {
    sources: &'a mut SourceManager,
    diagnostics: Vec<Diagnostic>,
    resolved_modules: HashMap<FileId, ModuleMetadata>,
    resolve_stack: Vec<FileId>,
}

impl<'a> ModuleResolver<'a> {
    pub fn new(sources: &'a mut SourceManager) -> Self {
        Self {
            sources,
            diagnostics: Vec::new(),
            resolved_modules: HashMap::new(),
            resolve_stack: Vec::new(),
        }
    }

    pub fn resolve(mut self, root_file_id: FileId, root_ast: &AstNode) -> (Vec<Diagnostic>, Vec<ModuleMetadata>) {
        self.visit_module(root_file_id, root_ast);
        let modules = self.resolved_modules.values().cloned().collect();
        (self.diagnostics, modules)
    }

    fn visit_module(&mut self, file_id: FileId, ast: &AstNode) {
        if self.resolved_modules.contains_key(&file_id) {
            return;
        }

        if self.resolve_stack.contains(&file_id) {
            self.diagnostics.push(
                Diagnostic::warning("Circular module dependency detected.").with_span(self.module_span(file_id)),
            );
            return;
        }

        self.resolve_stack.push(file_id);

        let mut dependencies = Vec::new();
        let require_calls = self.collect_require_calls(ast);

        for (span, module_request) in require_calls.iter().cloned() {
            match self.sources.resolve_module(file_id, &module_request) {
                Ok(dependency_id) => {
                    dependencies.push(dependency_id);
                    let dependency_ast = self.parse_file(dependency_id);
                    self.visit_module(dependency_id, &dependency_ast);
                }
                Err(error) => {
                    self.diagnostics.push(
                        Diagnostic::error(format!("Module '{}' could not be loaded: {}", module_request, error)).with_span(span),
                    );
                }
            }
        }

        let metadata = ModuleMetadata {
            file_id,
            path: self.sources.get(file_id).map(|f| f.path().to_path_buf()).unwrap_or_default(),
            module_name: self.module_name(file_id),
            dependencies,
            exports: self.extract_exports(ast),
            ast: ast.clone(),
        };

        self.resolved_modules.insert(file_id, metadata);
        self.resolve_stack.pop();
    }

    fn collect_require_calls(&mut self, ast: &AstNode) -> Vec<(SourceSpan, String)> {
        let mut calls = Vec::new();
        if let AstNode::Program(program) = ast {
            for statement in &program.statements {
                self.collect_require_calls_in_statement(statement, &mut calls);
            }
        }
        calls
    }

    fn collect_require_calls_in_statement(&mut self, statement: &Statement, calls: &mut Vec<(SourceSpan, String)>) {
        match &statement.kind {
            StatementKind::Local { initializers, .. }
            | StatementKind::Assignment { values: initializers, .. } => {
                for initializer in initializers {
                    self.collect_require_calls_in_expression(initializer, calls);
                }
            }
            StatementKind::Function { body, .. } => {
                for body_statement in body {
                    self.collect_require_calls_in_statement(body_statement, calls);
                }
            }
            StatementKind::Return(values) => {
                if let Some(values) = values {
                    for value in values {
                        self.collect_require_calls_in_expression(value, calls);
                    }
                }
            }
            StatementKind::Expression(expression) => {
                self.collect_require_calls_in_expression(expression, calls);
            }
            StatementKind::TypeAlias { .. } | StatementKind::Break | StatementKind::Continue | StatementKind::Empty | StatementKind::Error => {}
        }
    }

    fn collect_require_calls_in_expression(&mut self, expression: &Expression, calls: &mut Vec<(SourceSpan, String)>) {
        if let Some(module_request) = self.extract_require_call(expression) {
            calls.push((expression.span, module_request));
            return;
        }

        match &expression.kind {
            ExpressionKind::Unary { operand, .. } => self.collect_require_calls_in_expression(operand, calls),
            ExpressionKind::Binary { left, right, .. } => {
                self.collect_require_calls_in_expression(left, calls);
                self.collect_require_calls_in_expression(right, calls);
            }
            ExpressionKind::Call { callee, arguments } => {
                self.collect_require_calls_in_expression(callee, calls);
                for argument in arguments {
                    self.collect_require_calls_in_expression(argument, calls);
                }
            }
            ExpressionKind::TableConstructor(fields) => {
                for field in fields {
                    match field {
                        TableField::Named { value, .. } => self.collect_require_calls_in_expression(value, calls),
                        TableField::Indexed { key, value } => {
                            self.collect_require_calls_in_expression(key, calls);
                            self.collect_require_calls_in_expression(value, calls);
                        }
                        TableField::Expression(expr) => self.collect_require_calls_in_expression(expr, calls),
                    }
                }
            }
            ExpressionKind::MemberAccess { object, .. } => self.collect_require_calls_in_expression(object, calls),
            ExpressionKind::Index { object, index } => {
                self.collect_require_calls_in_expression(object, calls);
                self.collect_require_calls_in_expression(index, calls);
            }
            ExpressionKind::MethodCall { receiver, arguments, .. } => {
                self.collect_require_calls_in_expression(receiver, calls);
                for argument in arguments {
                    self.collect_require_calls_in_expression(argument, calls);
                }
            }
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let InterpolatedStringPart::Expression(expr) = part {
                        self.collect_require_calls_in_expression(expr, calls);
                    }
                }
            }
            ExpressionKind::Identifier(_) | ExpressionKind::NumberLiteral(_) | ExpressionKind::StringLiteral(_) | ExpressionKind::BooleanLiteral(_) | ExpressionKind::Nil | ExpressionKind::Error => {}
        }
    }

    fn extract_require_call(&self, expression: &Expression) -> Option<String> {
        if let ExpressionKind::Call { callee, arguments } = &expression.kind {
            if let ExpressionKind::Identifier(name) = &callee.kind {
                if name == "require" && arguments.len() == 1 {
                    if let ExpressionKind::StringLiteral(path) = &arguments[0].kind {
                        return Some(path.clone());
                    }
                }
            }
        }
        None
    }

    fn parse_file(&mut self, file_id: FileId) -> AstNode {
        let file = self.sources.get(file_id);
        let file = match file {
            Some(file) => file,
            None => return AstNode::Error,
        };

        let mut lexer = Lexer::new(file);
        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token();
            tokens.push(token.clone());
            if matches!(token.kind, crate::lexer::TokenKind::EOF) {
                break;
            }
        }

        let mut parser = Parser::new(&tokens);
        let ast = parser.parse_program();
        self.diagnostics.extend(parser.diagnostics().iter().cloned());
        ast
    }

    fn extract_exports(&self, ast: &AstNode) -> Vec<String> {
        let mut exports = Vec::new();
        if let AstNode::Program(program) = ast {
            for statement in &program.statements {
                if let StatementKind::Return(Some(values)) = &statement.kind {
                    if let Some(value) = values.first() {
                        if let ExpressionKind::TableConstructor(fields) = &value.kind {
                            for field in fields {
                                if let TableField::Named { key, .. } = field {
                                    exports.push(key.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        exports
    }

    fn module_name(&self, file_id: FileId) -> String {
        self.sources
            .get(file_id)
            .and_then(|file| file.path().file_stem().map(|stem| stem.to_string_lossy().to_string()))
            .unwrap_or_default()
    }

    fn module_span(&self, file_id: FileId) -> SourceSpan {
        self.sources
            .get(file_id)
            .map(|file| SourceSpan::new(file.id(), 0, 0))
            .unwrap_or_else(|| SourceSpan::new(FileId::new(0), 0, 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::TokenKind;
    use crate::source::{SourceManager, FileId};
    use std::path::PathBuf;

    fn parse_code(manager: &mut SourceManager, file_id: FileId) -> AstNode {
        let file = manager.get(file_id).expect("source file should exist");
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
    fn resolves_single_relative_module() {
        let mut manager = SourceManager::new();
        let main_id = manager.add_file(PathBuf::from("src/main.glu"), String::from("local math = require('./math')"));
        let _math_id = manager.add_file(PathBuf::from("src/math.glu"), String::from("local a = 1"));
        let ast = parse_code(&mut manager, main_id);
        println!("AST={:?}", ast);

        let resolver = ModuleResolver::new(&mut manager);
        let (diagnostics, modules) = resolver.resolve(main_id, &ast);

        assert!(diagnostics.is_empty());
        assert_eq!(modules.len(), 2);
        let main = modules.iter().find(|module| module.file_id == main_id).expect("main module found");
        assert_eq!(main.dependencies.len(), 1);
    }

    #[test]
    fn reuses_duplicate_module_imports() {
        let mut manager = SourceManager::new();
        let main_id = manager.add_file(PathBuf::from("src/main.glu"), String::from("local a = require('./math')\nlocal b = require('./math')"));
        let math_id = manager.add_file(PathBuf::from("src/math.glu"), String::from("local a = 1"));
        let ast = parse_code(&mut manager, main_id);

        let resolver = ModuleResolver::new(&mut manager);
        let (diagnostics, modules) = resolver.resolve(main_id, &ast);

        assert!(diagnostics.is_empty());
        assert_eq!(modules.len(), 2);
        let main = modules.iter().find(|module| module.file_id == main_id).expect("main module found");
        assert_eq!(main.dependencies, vec![math_id, math_id]);
    }

    #[test]
    fn detects_missing_module() {
        let mut manager = SourceManager::new();
        let main_id = manager.add_file(PathBuf::from("src/main.glu"), String::from("local math = require('./missing')"));
        let ast = parse_code(&mut manager, main_id);

        let resolver = ModuleResolver::new(&mut manager);
        let (diagnostics, modules) = resolver.resolve(main_id, &ast);

        assert!(!diagnostics.is_empty());
        assert_eq!(modules.len(), 1);
    }

    #[test]
    fn detects_circular_module_dependencies() {
        let mut manager = SourceManager::new();
        let main_id = manager.add_file(PathBuf::from("src/main.glu"), String::from("local a = require('./a')"));
        let _a_id = manager.add_file(PathBuf::from("src/a.glu"), String::from("local b = require('./b')"));
        let _b_id = manager.add_file(PathBuf::from("src/b.glu"), String::from("local a = require('./a')"));
        let ast = parse_code(&mut manager, main_id);

        let resolver = ModuleResolver::new(&mut manager);
        let (diagnostics, modules) = resolver.resolve(main_id, &ast);

        assert!(!diagnostics.is_empty());
        assert_eq!(modules.len(), 3);
    }

    #[test]
    fn extracts_module_exports_from_return_table() {
        let mut manager = SourceManager::new();
        let main_id = manager.add_file(PathBuf::from("src/main.glu"), String::from("local math = require('./math')"));
        let math_id = manager.add_file(PathBuf::from("src/math.glu"), String::from("return { Add = Add, Subtract = Subtract }"));
        let ast = parse_code(&mut manager, main_id);

        let resolver = ModuleResolver::new(&mut manager);
        let (_diagnostics, modules) = resolver.resolve(main_id, &ast);

        let math_module = modules.iter().find(|module| module.file_id == math_id).expect("math module found");
        assert_eq!(math_module.exports, vec!["Add".to_string(), "Subtract".to_string()]);
    }
}
