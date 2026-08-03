use crate::source::SourceSpan;
use super::function::HirFunction;
use super::expression::HirExpression;
use super::types::HirType;

#[derive(Debug, Clone)]
pub struct HirModule {
    pub name: String,
    pub functions: Vec<HirFunction>,
    pub global_variables: Vec<HirGlobalVariable>,
    pub span: SourceSpan,
}

impl HirModule {
    pub fn new(name: String, span: SourceSpan) -> Self {
        Self {
            name,
            functions: Vec::new(),
            global_variables: Vec::new(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HirGlobalVariable {
    pub name: String,
    pub var_type: Option<HirType>,
    pub initializer: Option<HirExpression>,
    pub span: SourceSpan,
}