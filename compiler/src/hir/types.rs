#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirType {
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function,
    Any,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOperator {
    Negate,      // -
    Not,         // not
    Length,      // #
    BitwiseNot,  // ~
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinaryOperator {
    // Arithmetic
    Add,          // +
    Subtract,     // -
    Multiply,     // *
    Divide,       // /
    FloorDivide,  // //
    Modulo,       // %
    Exponent,     // ^
    
    // Comparison
    Equal,        // ==
    NotEqual,     // ~=
    LessThan,     // <
    LessEqual,    // <=
    GreaterThan,  // >
    GreaterEqual, // >=
    
    // Logical
    And,          // and
    Or,           // or
    
    // String/Concatenation
    Concatenate,  // ..
    
    // Bitwise
    BitwiseAnd,   // &
    BitwiseOr,    // |
    BitwiseXor,   // ^
    BitwiseShiftLeft,   // <<
    BitwiseShiftRight,  // >>
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