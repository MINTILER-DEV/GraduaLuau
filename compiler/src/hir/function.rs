use crate::source::SourceSpan;
use super::ids::HirFunctionId;
use super::statement::HirStatement;
use super::types::HirType;

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub id: HirFunctionId,
    pub name: String,
    pub parameters: Vec<HirParameter>,
    pub body: Vec<HirStatement>,
    pub return_type: Option<HirType>,
    pub is_local: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct HirParameter {
    pub name: String,
    pub param_type: Option<HirType>,
    pub span: SourceSpan,
}