use super::expression::HirExpression;
use super::function::HirFunction;
use super::ids::{HirScopeId, HirSymbolId};
use super::symbol::{HirScope, HirSymbol};
use super::types::HirType;
use crate::source::SourceSpan;

#[derive(Debug, Clone)]
pub struct HirModule {
    pub name: String,
    pub functions: Vec<HirFunction>,
    pub global_variables: Vec<HirGlobalVariable>,
    pub type_aliases: Vec<HirTypeAlias>,
    pub scopes: Vec<HirScope>,
    pub symbols: Vec<HirSymbol>,
    pub metadata: HirModuleMetadata,
    pub span: SourceSpan,
}

impl HirModule {
    pub fn new(name: String, span: SourceSpan) -> Self {
        Self {
            name,
            functions: Vec::new(),
            global_variables: Vec::new(),
            type_aliases: Vec::new(),
            scopes: Vec::new(),
            symbols: Vec::new(),
            metadata: HirModuleMetadata::default(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HirGlobalVariable {
    pub symbol_id: HirSymbolId,
    pub name: String,
    pub var_type: Option<HirType>,
    pub initializer: Option<HirExpression>,
    pub scope_id: HirScopeId,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct HirTypeAlias {
    pub symbol_id: HirSymbolId,
    pub name: String,
    pub alias: HirType,
    pub scope_id: HirScopeId,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default)]
pub struct HirModuleMetadata {
    pub root_scope: Option<HirScopeId>,
}
