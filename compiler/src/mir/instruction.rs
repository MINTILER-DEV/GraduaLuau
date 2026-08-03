use super::types::{MirValueId, MirType, MirValue, MirBlockId};

#[derive(Debug, Clone)]
pub struct MirInstruction {
    pub kind: MirInstructionKind,
    pub result_type: Option<MirType>,
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
    
    // Control flow
    Branch {
        condition: MirValueId,
        true_block: MirBlockId,
        false_block: MirBlockId,
    },
    Jump {
        target: MirBlockId,
    },
    
    // Function calls
    Call {
        result: Option<MirValueId>,
        function: String,
        arguments: Vec<MirValueId>,
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