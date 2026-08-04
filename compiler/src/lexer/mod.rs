use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    // Keywords
    If,
    Then,
    Else,
    ElseIf,
    End,
    While,
    Do,
    For,
    In,
    Repeat,
    Until,
    Break,
    Continue,
    Local,
    Function,
    Return,
    And,
    Or,
    Not,
    Type,
    Export,
    Any,
    Never,
    Typeof,
    NumberLiteral(String),
    StringLiteral(String),
    InterpolatedString(String),
    True,
    False,
    Nil,

    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,

    // Comparison / assignment
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Compound assignment
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    PercentEqual,
    AmpersandEqual,
    PipeEqual,

    // Delimiters
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    DotDot,
    DotDotDot,
    Colon,
    Semicolon,
    Pipe,
    Ampersand,
    Question,
    Arrow,
    InterpolatedStringStart,
    InterpolatedStringEnd,
    StringText(String),
    InterpolationStart,
    InterpolationEnd,

    EOF,
    Unknown(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

pub struct Lexer<'a> {
    file: &'a SourceFile,
    src: &'a str,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    mode: LexerMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerMode {
    Normal,
    InterpolatedString,
    Interpolation,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a SourceFile) -> Self {
        let src = file.text();

        Self {
            file,
            src,
            pos: 0,
            diagnostics: Vec::new(),
            mode: LexerMode::Normal,
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn current_byte(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.current_byte();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn make_span(&self, start: usize, end: usize) -> SourceSpan {
        SourceSpan::new(self.file.id(), start, end)
    }

    pub fn next_token(&mut self) -> Token {
        match self.mode {
            LexerMode::Normal => self.next_normal_token(),
            LexerMode::InterpolatedString => self.next_interpolated_string_token(),
            LexerMode::Interpolation => self.next_interpolation_token(),
        }
    }

    fn next_normal_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.pos;

        match self.current_byte() {
            None => Token {
                kind: TokenKind::EOF,
                span: self.make_span(start, start),
            },
            Some(b) => {
                // identifiers or keywords
                if is_alpha(b) || b == b'_' {
                    let s = self.read_identifier();
                    let kind = if let Some(kw) = keyword_token(&s) {
                        kw
                    } else {
                        TokenKind::Identifier(s.clone())
                    };
                    let end = self.pos;
                    Token {
                        kind,
                        span: self.make_span(start, end),
                    }
                } else if is_digit(b) {
                    let s = self.read_number();
                    let end = self.pos;
                    Token {
                        kind: TokenKind::NumberLiteral(s),
                        span: self.make_span(start, end),
                    }
                } else if b == b'"' || b == b'\'' {
                    // string literal (double or single quoted)
                    let quote = b;
                    self.advance(); // consume quote
                    let s = self.read_string(quote, start);
                    let end = self.pos;
                    Token {
                        kind: TokenKind::StringLiteral(s),
                        span: self.make_span(start, end),
                    }
                } else if b == b'`' {
                    self.advance();
                    let end = self.pos;
                    self.mode = LexerMode::InterpolatedString;
                    Token {
                        kind: TokenKind::InterpolatedStringStart,
                        span: self.make_span(start, end),
                    }
                } else {
                    // operators and punctuation
                    // three-char operators
                    if b == b'.'
                        && self.peek_byte() == Some(b'.')
                        && self.src.as_bytes().get(self.pos + 2).copied() == Some(b'.')
                    {
                        self.advance();
                        self.advance();
                        self.advance();
                        let end = self.pos;
                        Token {
                            kind: TokenKind::DotDotDot,
                            span: self.make_span(start, end),
                        }
                    } else if let Some(kind_op) = self.match_two_char_op(b) {
                        let end = self.pos;
                        Token {
                            kind: kind_op,
                            span: self.make_span(start, end),
                        }
                    } else {
                        // single char
                        let ch = self.advance().unwrap() as char;
                        let kind = match ch {
                            '+' => TokenKind::Plus,
                            '-' => TokenKind::Minus,
                            '*' => TokenKind::Star,
                            '/' => TokenKind::Slash,
                            '%' => TokenKind::Percent,
                            '^' => TokenKind::Caret,
                            '=' => TokenKind::Equal,
                            '<' => TokenKind::Less,
                            '>' => TokenKind::Greater,
                            '(' => TokenKind::LeftParen,
                            ')' => TokenKind::RightParen,
                            '{' => TokenKind::LeftBrace,
                            '}' => TokenKind::RightBrace,
                            '[' => TokenKind::LeftBracket,
                            ']' => TokenKind::RightBracket,
                            ',' => TokenKind::Comma,
                            ';' => TokenKind::Semicolon,
                            ':' => TokenKind::Colon,
                            '.' => TokenKind::Dot,
                            '|' => TokenKind::Pipe,
                            '&' => TokenKind::Ampersand,
                            '?' => TokenKind::Question,
                            _ => {
                                self.push_diagnostic(
                                    "Unexpected character",
                                    self.make_span(start, self.pos),
                                );
                                TokenKind::Unknown(ch)
                            }
                        };
                        if matches!(kind, TokenKind::Dot)
                            && self.current_byte().map_or(false, is_digit)
                        {
                            self.push_diagnostic(
                                "Invalid numeric literal",
                                self.make_span(start, self.pos + 1),
                            );
                        }
                        let end = self.pos;
                        Token {
                            kind,
                            span: self.make_span(start, end),
                        }
                    }
                }
            }
        }
    }

    fn next_interpolation_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start = self.pos;
        match self.current_byte() {
            Some(b'}') => {
                self.advance();
                self.mode = LexerMode::InterpolatedString;
                Token {
                    kind: TokenKind::InterpolationEnd,
                    span: self.make_span(start, self.pos),
                }
            }
            None => {
                self.push_diagnostic(
                    "Unterminated interpolated string",
                    self.make_span(start, start),
                );
                self.mode = LexerMode::Normal;
                Token {
                    kind: TokenKind::EOF,
                    span: self.make_span(start, start),
                }
            }
            _ => self.next_normal_token(),
        }
    }

    fn next_interpolated_string_token(&mut self) -> Token {
        let start = self.pos;

        match self.current_byte() {
            Some(b'`') => {
                self.advance();
                self.mode = LexerMode::Normal;
                Token {
                    kind: TokenKind::InterpolatedStringEnd,
                    span: self.make_span(start, self.pos),
                }
            }
            Some(b'{') => {
                self.advance();
                self.mode = LexerMode::Interpolation;
                Token {
                    kind: TokenKind::InterpolationStart,
                    span: self.make_span(start, self.pos),
                }
            }
            Some(_) => {
                let text = self.read_interpolated_string_text();
                Token {
                    kind: TokenKind::StringText(text),
                    span: self.make_span(start, self.pos),
                }
            }
            None => {
                self.push_diagnostic(
                    "Unterminated interpolated string",
                    self.make_span(start, start),
                );
                self.mode = LexerMode::Normal;
                Token {
                    kind: TokenKind::EOF,
                    span: self.make_span(start, start),
                }
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_byte() {
                Some(b) if is_whitespace(b) => {
                    self.advance();
                }
                Some(b) if b == b'-' && self.peek_byte() == Some(b'-') => {
                    if self.src.as_bytes().get(self.pos + 2).copied() == Some(b'[')
                        && self.src.as_bytes().get(self.pos + 3).copied() == Some(b'[')
                    {
                        self.skip_block_comment();
                    } else {
                        // line comment
                        self.advance(); // -
                        self.advance(); // -
                        while let Some(c) = self.current_byte() {
                            if c == b'\n' {
                                break;
                            }
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.current_byte() {
            if is_alpha(b) || is_digit(b) || b == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        self.src[start..self.pos].to_string()
    }

    fn read_number(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.current_byte() {
            if is_digit(b) {
                self.advance();
            } else {
                break;
            }
        }
        // fractional part
        if self.current_byte() == Some(b'.') && self.peek_byte().map_or(false, |c| is_digit(c)) {
            self.advance();
            while let Some(b) = self.current_byte() {
                if is_digit(b) {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.current_byte() == Some(b'.') && self.peek_byte().map_or(false, is_digit) {
                self.push_diagnostic(
                    "Malformed numeric literal",
                    self.make_span(start, self.pos + 2),
                );
            }
        } else if self.current_byte() == Some(b'.') && self.peek_byte() != Some(b'.') {
            self.push_diagnostic(
                "Invalid numeric literal",
                self.make_span(start, self.pos + 1),
            );
        }
        // exponent (simple)
        if let Some(b'e') | Some(b'E') = self.current_byte() {
            self.advance();
            if let Some(b'+') | Some(b'-') = self.current_byte() {
                self.advance();
            }
            while let Some(b) = self.current_byte() {
                if is_digit(b) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.src[start..self.pos].to_string()
    }

    fn read_string(&mut self, quote: u8, token_start: usize) -> String {
        let mut out = String::new();
        while let Some(b) = self.current_byte() {
            if b == quote {
                self.advance();
                return out;
            }
            if b == b'\n' || b == b'\r' {
                self.push_diagnostic(
                    "Unterminated string literal",
                    self.make_span(token_start, self.pos),
                );
                return out;
            }
            if b == b'\\' {
                let escape_start = self.pos;
                self.advance();
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        b'\'' => out.push('\''),
                        other => {
                            self.push_diagnostic(
                                "Unknown escape sequence",
                                self.make_span(escape_start, self.pos),
                            );
                            out.push(other as char);
                        }
                    }
                } else {
                    self.push_diagnostic(
                        "Unterminated string literal",
                        self.make_span(token_start, self.pos),
                    );
                    return out;
                }
            } else {
                out.push(b as char);
                self.advance();
            }
        }
        self.push_diagnostic(
            "Unterminated string literal",
            self.make_span(token_start, self.pos),
        );
        out
    }

    fn read_interpolated_string_text(&mut self) -> String {
        let mut content = String::new();

        while let Some(b) = self.current_byte() {
            if b == b'`' || b == b'{' {
                break;
            }
            content.push(b as char);
            self.advance();
        }

        content
    }

    fn skip_block_comment(&mut self) {
        let start = self.pos;
        self.advance();
        self.advance();
        self.advance();
        self.advance();

        while let Some(b) = self.current_byte() {
            if b == b']' && self.peek_byte() == Some(b']') {
                self.advance();
                self.advance();
                return;
            }
            self.advance();
        }

        self.push_diagnostic(
            "Unterminated block comment.",
            self.make_span(start, self.pos),
        );
    }

    fn match_two_char_op(&mut self, b: u8) -> Option<TokenKind> {
        match (b, self.peek_byte()) {
            (b'=', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::EqualEqual)
            }
            (b'~', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::NotEqual)
            }
            (b'<', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::LessEqual)
            }
            (b'>', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::GreaterEqual)
            }
            (b'+', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::PlusEqual)
            }
            (b'-', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::MinusEqual)
            }
            (b'*', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::StarEqual)
            }
            (b'/', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::SlashEqual)
            }
            (b'%', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::PercentEqual)
            }
            (b'&', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::AmpersandEqual)
            }
            (b'|', Some(b'=')) => {
                self.advance();
                self.advance();
                Some(TokenKind::PipeEqual)
            }
            (b'.', Some(b'.')) => {
                self.advance();
                self.advance();
                Some(TokenKind::DotDot)
            }
            (b'-', Some(b'>')) => {
                self.advance();
                self.advance();
                Some(TokenKind::Arrow)
            }
            _ => None,
        }
    }

    fn push_diagnostic(&mut self, message: impl Into<String>, span: SourceSpan) {
        self.diagnostics
            .push(Diagnostic::error(message).with_span(span));
    }
}

fn is_alpha(b: u8) -> bool {
    (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b)
}
fn is_digit(b: u8) -> bool {
    (b'0'..=b'9').contains(&b)
}
fn is_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r' || b == b'\n'
}

fn keyword_token(s: &str) -> Option<TokenKind> {
    Some(match s {
        "if" => TokenKind::If,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "elseif" => TokenKind::ElseIf,
        "end" => TokenKind::End,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "repeat" => TokenKind::Repeat,
        "until" => TokenKind::Until,
        "break" => TokenKind::Break,
        "local" => TokenKind::Local,
        "function" => TokenKind::Function,
        "return" => TokenKind::Return,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "type" => TokenKind::Type,
        "export" => TokenKind::Export,
        "any" => TokenKind::Any,
        "never" => TokenKind::Never,
        "typeof" => TokenKind::Typeof,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "nil" => TokenKind::Nil,
        "continue" => TokenKind::Continue,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceManager;
    use std::path::PathBuf;

    #[test]
    fn basic_tokenization() {
        let mut srcs = SourceManager::new();
        let id = srcs.add_file(
            PathBuf::from("test.glu"),
            String::from("local x = 42\nprint(x)\n-- comment\n"),
        );
        let file = srcs.get(id).unwrap();

        let mut lex = Lexer::new(file);
        let t1 = lex.next_token();
        assert!(matches!(t1.kind, TokenKind::Local));
        let t2 = lex.next_token();
        assert!(matches!(t2.kind, TokenKind::Identifier(ref s) if s == "x"));
        let t3 = lex.next_token();
        assert!(matches!(t3.kind, TokenKind::Equal));
        let t4 = lex.next_token();
        assert!(matches!(t4.kind, TokenKind::NumberLiteral(ref s) if s == "42"));
        // print
        let t5 = lex.next_token();
        assert!(matches!(t5.kind, TokenKind::Identifier(ref s) if s == "print"));
        let t6 = lex.next_token();
        assert!(matches!(t6.kind, TokenKind::LeftParen));
        let t7 = lex.next_token();
        assert!(matches!(t7.kind, TokenKind::Identifier(ref s) if s == "x"));
        let t8 = lex.next_token();
        assert!(matches!(t8.kind, TokenKind::RightParen));
        let t9 = lex.next_token();
        assert!(matches!(t9.kind, TokenKind::EOF));
    }
}
#[derive(Debug, Default)]
pub struct LexerStage;
