#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirFunctionId(pub usize);

impl HirFunctionId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirVariableId(pub usize);

impl HirVariableId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirSymbolId(pub usize);

impl HirSymbolId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirScopeId(pub usize);

impl HirScopeId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}
