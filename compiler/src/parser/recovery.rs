use crate::lexer::TokenKind;

pub struct RecoveryState {
    error_count: usize,
    in_error_recovery: bool,
}

impl RecoveryState {
    pub fn new() -> Self { 
        Self { 
            error_count: 0,
            in_error_recovery: false,
        } 
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn increment_error_count(&mut self) {
        self.error_count += 1;
    }

    pub fn is_in_error_recovery(&self) -> bool {
        self.in_error_recovery
    }

    pub fn set_in_error_recovery(&mut self, value: bool) {
        self.in_error_recovery = value;
    }

    pub fn should_suppress_cascading(&self) -> bool {
        // Suppress cascading errors if we're already recovering from an error
        // and have encountered a significant number of errors recently
        self.in_error_recovery && self.error_count > 3
    }
}

/// Synchronization tokens that indicate safe points for error recovery
pub fn is_synchronization_token(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::End |
        TokenKind::ElseIf |
        TokenKind::Else |
        TokenKind::Until |
        TokenKind::Function |
        TokenKind::Local |
        TokenKind::If |
        TokenKind::While |
        TokenKind::Repeat |
        TokenKind::For |
        TokenKind::Return |
        TokenKind::Break |
        TokenKind::Continue |
        TokenKind::Type |
        TokenKind::EOF
    )
}

/// Tokens that can terminate an expression
pub fn is_expression_boundary(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Comma |
        TokenKind::RightParen |
        TokenKind::RightBracket |
        TokenKind::RightBrace |
        TokenKind::End |
        TokenKind::EOF |
        TokenKind::Semicolon
    )
}

/// Tokens that can terminate a statement
pub fn is_statement_boundary(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Semicolon |
        TokenKind::EOF |
        TokenKind::End |
        TokenKind::ElseIf |
        TokenKind::Else |
        TokenKind::Until
    )
}

/// Tokens that can terminate a block
pub fn is_block_terminator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::End |
        TokenKind::Else |
        TokenKind::ElseIf |
        TokenKind::Until |
        TokenKind::EOF
    )
}

/// Tokens that follow in a parameter list
pub fn is_parameter_separator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Comma |
        TokenKind::RightParen
    )
}

/// Tokens that can start a type annotation
pub fn can_start_type(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Identifier(_) |
        TokenKind::LeftParen |
        TokenKind::LeftBrace |
        TokenKind::DotDotDot |
        TokenKind::Any |
        TokenKind::Never |
        TokenKind::Nil
    )
}
