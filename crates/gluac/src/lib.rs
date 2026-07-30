pub mod ast;
pub mod codegen;
pub mod errors;
pub mod lexer;
pub mod parser;
pub mod semantic;

use ast::Program;
use codegen::CodegenOutput;
use errors::CompilerResult;
use lexer::Lexer;
use parser::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    pub program: Program,
    pub codegen: CodegenOutput,
}

pub fn compile_source(source: &str) -> CompilerResult<CompileOutput> {
    let tokens = Lexer::new(source).tokenize()?;
    let program = Parser::new(tokens).parse_program()?;

    semantic::analyze(&program)?;
    let codegen = codegen::generate(&program)?;

    Ok(CompileOutput { program, codegen })
}

#[cfg(test)]
mod tests {
    use super::compile_source;

    #[test]
    fn compiles_empty_source() {
        let output = compile_source("").expect("empty source should compile during phase 0");

        assert!(output.program.statements.is_empty());
        assert!(!output.codegen.text.is_empty());
    }
}
