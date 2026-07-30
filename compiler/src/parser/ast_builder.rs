use crate::source::SourceSpan;

// Minimal AST node types for scaffolding
#[derive(Debug, Clone)]
pub enum AstNode {
    Program,
    Error,
}

pub fn make_program(span: SourceSpan) -> AstNode {
    let _ = span;
    AstNode::Program
}
