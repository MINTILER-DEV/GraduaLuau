use super::types::{MirValue, MirValueId};

#[derive(Debug, Clone, PartialEq)]
pub enum MirOperand {
    Value(MirValueId),
    Constant(MirValue),
    Local(String),
    Global(String),
    Function(String),
}
