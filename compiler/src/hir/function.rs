use super::ids::{HirFunctionId, HirScopeId, HirSymbolId, HirVariableId};
use super::statement::HirLocalVariable;
use super::statement::HirStatement;
use super::types::{HirFunctionSignature, HirType};
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct HirFunction {
    pub id: HirFunctionId,
    pub symbol_id: HirSymbolId,
    pub name: String,
    pub parameters: Vec<HirParameter>,
    pub local_variables: Vec<HirLocalVariable>,
    pub body: Vec<HirStatement>,
    pub return_type: Option<HirType>,
    pub signature: HirFunctionSignature,
    pub scope_id: HirScopeId,
    pub is_local: bool,
    pub metadata: HirFunctionMetadata,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct HirParameter {
    pub id: HirVariableId,
    pub symbol_id: HirSymbolId,
    pub name: String,
    pub param_type: Option<HirType>,
    pub scope_id: HirScopeId,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default)]
pub struct HirFunctionMetadata {
    pub has_explicit_return: bool,
}
