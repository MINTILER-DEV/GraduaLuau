use crate::ast::Program;
use crate::errors::CompilerResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenOutput {
    pub text: String,
}

pub fn generate(_program: &Program) -> CompilerResult<CodegenOutput> {
    Ok(CodegenOutput {
        text: String::from("-- code generation is not implemented yet"),
    })
}
