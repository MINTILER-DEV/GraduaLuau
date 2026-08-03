#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirValueId(pub usize);

impl MirValueId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirBlockId(pub usize);

impl MirBlockId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MirFunctionId(pub usize);

impl MirFunctionId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirType {
    Void,
    Integer,
    Float,
    Boolean,
    String,
    Table,
    Function,
    Any,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MirValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Nil,
    Unit,
}