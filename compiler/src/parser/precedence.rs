#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    LogicalOr,
    LogicalAnd,
    Comparison,
    Addition,
    Multiplication,
    Exponentiation,
    Unary,
    Primary,
}

impl Precedence {
    pub fn of_operator(_op: &str) -> Precedence {
        // placeholder mapping
        Precedence::Lowest
    }
}
