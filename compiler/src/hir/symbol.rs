use crate::source::SourceSpan;

use super::ids::{HirScopeId, HirSymbolId};
use super::types::{HirFunctionSignature, HirType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirScopeKind {
    Global,
    Module,
    Function,
    Block,
    If,
    Else,
    While,
    Repeat,
    For,
    GenericBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirSymbolKind {
    BuiltinFunction,
    Constant,
    Function,
    Global,
    Local,
    Module,
    NativeFunction,
    Parameter,
    Type,
}

#[derive(Debug, Clone)]
pub struct HirSymbol {
    pub id: HirSymbolId,
    pub name: String,
    pub kind: HirSymbolKind,
    pub scope_id: HirScopeId,
    pub value_type: Option<HirType>,
    pub function_signature: Option<HirFunctionSignature>,
    pub span: SourceSpan,
}

impl HirSymbol {
    pub fn new(
        id: HirSymbolId,
        name: String,
        kind: HirSymbolKind,
        scope_id: HirScopeId,
        value_type: Option<HirType>,
        function_signature: Option<HirFunctionSignature>,
        span: SourceSpan,
    ) -> Self {
        Self {
            id,
            name,
            kind,
            scope_id,
            value_type,
            function_signature,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HirScope {
    pub id: HirScopeId,
    pub parent: Option<HirScopeId>,
    pub children: Vec<HirScopeId>,
    pub kind: HirScopeKind,
    pub symbols: Vec<HirSymbolId>,
    pub span: SourceSpan,
}

impl HirScope {
    pub fn new(
        id: HirScopeId,
        parent: Option<HirScopeId>,
        kind: HirScopeKind,
        span: SourceSpan,
    ) -> Self {
        Self {
            id,
            parent,
            children: Vec::new(),
            kind,
            symbols: Vec::new(),
            span,
        }
    }
}
