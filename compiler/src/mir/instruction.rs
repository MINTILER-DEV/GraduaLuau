use crate::source::SourceSpan;

use super::types::{MirBlockId, MirCompareOperator, MirType, MirValue, MirValueId};

#[derive(Debug, Clone)]
pub struct MirInstruction {
    pub kind: MirInstructionKind,
    pub result_type: Option<MirType>,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone)]
pub enum MirInstructionKind {
    // Constants
    Const {
        result: MirValueId,
        value: MirValue,
    },

    // Arithmetic
    Add {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Subtract {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Multiply {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Divide {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Modulo {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },

    // Comparison
    Equal {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    NotEqual {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    LessThan {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    LessEqual {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    GreaterThan {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    GreaterEqual {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },

    // Boolean operations
    And {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Or {
        result: MirValueId,
        left: MirValueId,
        right: MirValueId,
    },
    Not {
        result: MirValueId,
        operand: MirValueId,
    },

    // Memory operations
    Load {
        result: MirValueId,
        name: String,
    },
    Store {
        name: String,
        value: MirValueId,
    },
    Move {
        result: MirValueId,
        value: MirValueId,
    },
    AllocateLocal {
        local: MirValueId,
        name: String,
    },

    // Control flow
    Branch {
        condition: MirValueId,
        true_block: MirBlockId,
        false_block: MirBlockId,
    },
    Jump {
        target: MirBlockId,
    },
    Unreachable,

    // Function calls
    Call {
        result: Option<MirValueId>,
        function: String,
        arguments: Vec<MirValueId>,
    },
    Compare {
        result: MirValueId,
        operator: MirCompareOperator,
        left: MirValueId,
        right: MirValueId,
    },

    // Return
    Return {
        value: Option<MirValueId>,
    },

    // Table operations
    TableNew {
        result: MirValueId,
    },
    TableSet {
        table: MirValueId,
        key: MirValueId,
        value: MirValueId,
    },
    TableGet {
        result: MirValueId,
        table: MirValueId,
        key: MirValueId,
    },

    // Error recovery
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirTerminator {
    Return {
        value: Option<MirValueId>,
    },
    Jump {
        target: MirBlockId,
    },
    Branch {
        condition: MirValueId,
        true_block: MirBlockId,
        false_block: MirBlockId,
    },
    Unreachable,
}

impl MirInstruction {
    pub fn new(
        kind: MirInstructionKind,
        result_type: Option<MirType>,
        span: Option<SourceSpan>,
    ) -> Self {
        Self {
            kind,
            result_type,
            span,
        }
    }

    pub fn terminator(&self) -> Option<MirTerminator> {
        MirTerminator::from_instruction_kind(&self.kind)
    }

    pub fn is_terminator(&self) -> bool {
        self.terminator().is_some()
    }
}

impl MirTerminator {
    pub fn from_instruction_kind(kind: &MirInstructionKind) -> Option<Self> {
        match kind {
            MirInstructionKind::Return { value } => Some(Self::Return { value: *value }),
            MirInstructionKind::Jump { target } => Some(Self::Jump { target: *target }),
            MirInstructionKind::Branch {
                condition,
                true_block,
                false_block,
            } => Some(Self::Branch {
                condition: *condition,
                true_block: *true_block,
                false_block: *false_block,
            }),
            MirInstructionKind::Unreachable => Some(Self::Unreachable),
            _ => None,
        }
    }

    pub fn successors(&self) -> Vec<MirBlockId> {
        match self {
            Self::Return { .. } | Self::Unreachable => Vec::new(),
            Self::Jump { target } => vec![*target],
            Self::Branch {
                true_block,
                false_block,
                ..
            } => vec![*true_block, *false_block],
        }
    }
}
