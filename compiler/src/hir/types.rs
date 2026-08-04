use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirType {
    Nil,
    Boolean,
    Integer,
    Number,
    String,
    Table,
    Function,
    Thread,
    Userdata,
    Any,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirCallingConvention {
    GraduaLuau,
    Builtin,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirFunctionSignature {
    pub parameter_types: Vec<HirType>,
    pub return_type: HirType,
    pub calling_convention: HirCallingConvention,
    pub is_variadic: bool,
}

impl fmt::Display for HirType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            HirType::Nil => "nil",
            HirType::Boolean => "boolean",
            HirType::Integer => "integer",
            HirType::Number => "number",
            HirType::String => "string",
            HirType::Table => "table",
            HirType::Function => "function",
            HirType::Thread => "thread",
            HirType::Userdata => "userdata",
            HirType::Any => "any",
            HirType::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOperator {
    Negate,     // -
    Not,        // not
    Length,     // #
    BitwiseNot, // ~
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinaryOperator {
    // Arithmetic
    Add,         // +
    Subtract,    // -
    Multiply,    // *
    Divide,      // /
    FloorDivide, // //
    Modulo,      // %
    Exponent,    // ^

    // Comparison
    Equal,        // ==
    NotEqual,     // ~=
    LessThan,     // <
    LessEqual,    // <=
    GreaterThan,  // >
    GreaterEqual, // >=

    // Logical
    And, // and
    Or,  // or

    // String/Concatenation
    Concatenate, // ..

    // Bitwise
    BitwiseAnd,        // &
    BitwiseOr,         // |
    BitwiseXor,        // ^
    BitwiseShiftLeft,  // <<
    BitwiseShiftRight, // >>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirBuiltinFunction {
    Print,
    Type,
    ToNumber,
    ToString,
    Error,
    Pairs,
    Ipairs,
    Require,
}
