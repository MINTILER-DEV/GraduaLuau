use crate::source::SourceSpan;

use super::types::{MirFunctionId, MirType, MirValue, MirValueId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirValueKind {
    Constant,
    Parameter {
        index: usize,
        storage: String,
        symbol_id: Option<usize>,
    },
    Local {
        storage: String,
        symbol_id: Option<usize>,
    },
    Temporary,
    Global {
        name: String,
        symbol_id: Option<usize>,
    },
    FunctionReference {
        function: MirFunctionId,
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct MirValueData {
    pub id: MirValueId,
    pub value_type: MirType,
    pub kind: MirValueKind,
    pub span: Option<SourceSpan>,
}

impl MirValueData {
    pub fn new(
        id: MirValueId,
        value_type: MirType,
        kind: MirValueKind,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            id,
            value_type,
            kind,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MirParameter {
    pub name: String,
    pub storage: String,
    pub value_id: MirValueId,
    pub value_type: MirType,
    pub symbol_id: Option<usize>,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub struct MirLocal {
    pub storage: String,
    pub value_id: MirValueId,
    pub value_type: MirType,
    pub symbol_id: Option<usize>,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub struct MirGlobal {
    pub name: String,
    pub storage: String,
    pub value_id: MirValueId,
    pub value_type: MirType,
    pub symbol_id: Option<usize>,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub struct MirConstant {
    pub value_id: MirValueId,
    pub value: MirValue,
    pub value_type: MirType,
    pub span: Option<SourceSpan>,
}
