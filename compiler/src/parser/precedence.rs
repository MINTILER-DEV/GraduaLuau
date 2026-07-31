use crate::lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    LogicalOr,
    LogicalAnd,
    Comparison,
    Concatenation,
    Addition,
    Multiplication,
    Exponentiation,
    Unary,
    Primary,
}

impl Precedence {
    pub fn of_token(kind: &TokenKind) -> Option<Precedence> {
        Some(match kind {
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,
            TokenKind::EqualEqual
            | TokenKind::NotEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Precedence::Comparison,
            TokenKind::DotDot => Precedence::Concatenation,
            TokenKind::Plus | TokenKind::Minus => Precedence::Addition,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Multiplication,
            TokenKind::Caret => Precedence::Exponentiation,
            _ => return None,
        })
    }

    pub fn is_right_associative(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Caret | TokenKind::DotDot)
    }

    pub fn next(self) -> Precedence {
        match self {
            Precedence::Lowest => Precedence::LogicalOr,
            Precedence::LogicalOr => Precedence::LogicalAnd,
            Precedence::LogicalAnd => Precedence::Comparison,
            Precedence::Comparison => Precedence::Concatenation,
            Precedence::Concatenation => Precedence::Addition,
            Precedence::Addition => Precedence::Multiplication,
            Precedence::Multiplication => Precedence::Exponentiation,
            Precedence::Exponentiation => Precedence::Unary,
            Precedence::Unary => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}
