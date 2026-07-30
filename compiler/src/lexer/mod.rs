use crate::source::{FileId, SourceFile, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Keyword(String),
    Number(String),
    StringLiteral(String),
    Boolean(bool),
    Nil,
    Operator(String),
    Punct(char),
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
    len: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a SourceFile) -> Self {
        let src = file.text();
        let len = src.len();

        Self { file, src, pos: 0, len }
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
        self.skip_whitespace_and_comments();

        let start = self.pos;

        match self.current_byte() {
            None => Token { kind: TokenKind::EOF, span: self.make_span(start, start) },
            Some(b) => {
                // identifiers or keywords
                if is_alpha(b) || b == b'_' {
                    let s = self.read_identifier();
                    let kind = if is_keyword(&s) {
                        TokenKind::Keyword(s.clone())
                    } else {
                        TokenKind::Identifier(s.clone())
                    };
                    let end = self.pos;
                    Token { kind, span: self.make_span(start, end) }
                } else if is_digit(b) {
                    let s = self.read_number();
                    let end = self.pos;
                    Token { kind: TokenKind::Number(s), span: self.make_span(start, end) }
                } else if b == b'"' {
                    // string literal
                    self.advance(); // consume '"'
                    let s = self.read_string();
                    let end = self.pos;
                    Token { kind: TokenKind::StringLiteral(s), span: self.make_span(start, end) }
                } else {
                    // operators and punctuation
                    // two-char operators
                    if let Some(p) = self.match_two_char_op(b) {
                        let end = self.pos;
                        Token { kind: TokenKind::Operator(p), span: self.make_span(start, end) }
                    } else {
                        // single char
                        let ch = self.advance().unwrap() as char;
                        let kind = match ch {
                            '+' | '-' | '*' | '/' | '%' | '^' | '=' | '<' | '>' => TokenKind::Operator(ch.to_string()),
                            '(' | ')' | '{' | '}' | '[' | ']' | ',' | ';' | ':' | '.' => TokenKind::Punct(ch),
                            _ => TokenKind::Unknown(ch),
                        };
                        let end = self.pos;
                        Token { kind, span: self.make_span(start, end) }
                    }
                }
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_byte() {
                Some(b) if is_whitespace(b) => { self.advance(); }
                Some(b) if b == b'-' && self.peek_byte() == Some(b'-') => {
                    // line comment
                    self.advance(); // -
                    self.advance(); // -
                    while let Some(c) = self.current_byte() {
                        if c == b'\n' { break; }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.current_byte() {
            if is_alpha(b) || is_digit(b) || b == b'_' { self.advance(); } else { break; }
        }
        self.src[start..self.pos].to_string()
    }

    fn read_number(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.current_byte() {
            if is_digit(b) { self.advance(); } else { break; }
        }
        // fractional part
        if self.current_byte() == Some(b'.') && self.peek_byte().map_or(false, |c| is_digit(c)) {
            self.advance();
            while let Some(b) = self.current_byte() {
                if is_digit(b) { self.advance(); } else { break; }
            }
        }
        // exponent (simple)
        if let Some(b'e') | Some(b'E') = self.current_byte() {
            self.advance();
            if let Some(b'+') | Some(b'-') = self.current_byte() { self.advance(); }
            while let Some(b) = self.current_byte() {
                if is_digit(b) { self.advance(); } else { break; }
            }
        }

        self.src[start..self.pos].to_string()
    }

    fn read_string(&mut self) -> String {
        let mut out = String::new();
        while let Some(b) = self.current_byte() {
            if b == b'"' { self.advance(); break; }
            if b == b'\\' {
                self.advance();
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'"' => out.push('"'),
                        other => out.push(other as char),
                    }
                }
            } else {
                out.push(b as char);
                self.advance();
            }
        }
        out
    }

    fn match_two_char_op(&mut self, b: u8) -> Option<String> {
        match (b, self.peek_byte()) {
            (b'=', Some(b'=')) => { self.advance(); self.advance(); Some(String::from("==")) }
            (b'~', Some(b'=')) => { self.advance(); self.advance(); Some(String::from("~=")) }
            (b'<', Some(b'=')) => { self.advance(); self.advance(); Some(String::from("<=")) }
            (b'>', Some(b'=')) => { self.advance(); self.advance(); Some(String::from(">=")) }
            _ => None,
        }
    }
}

fn is_alpha(b: u8) -> bool { (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b) }
fn is_digit(b: u8) -> bool { (b'0'..=b'9').contains(&b) }
fn is_whitespace(b: u8) -> bool { b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' }

fn is_keyword(s: &str) -> bool {
    matches!(s, "local" | "if" | "else" | "while" | "for" | "function" | "return" | "require" | "break" | "continue" | "true" | "false" | "nil")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceManager;
    use std::path::PathBuf;

    #[test]
    fn basic_tokenization() {
        let mut srcs = SourceManager::new();
        let id = srcs.add_file(PathBuf::from("test.glu"), String::from("local x = 42\nprint(x)\n-- comment\n"));
        let file = srcs.get(id).unwrap();

        let mut lex = Lexer::new(file);
        let t1 = lex.next_token();
        assert!(matches!(t1.kind, TokenKind::Keyword(ref k) if k == "local"));
        let t2 = lex.next_token();
        assert!(matches!(t2.kind, TokenKind::Identifier(ref s) if s == "x"));
        let t3 = lex.next_token();
        assert!(matches!(t3.kind, TokenKind::Operator(ref s) if s == "="));
        let t4 = lex.next_token();
        assert!(matches!(t4.kind, TokenKind::Number(ref s) if s == "42"));
        // print
        let t5 = lex.next_token();
        assert!(matches!(t5.kind, TokenKind::Identifier(ref s) if s == "print"));
        let t6 = lex.next_token();
        assert!(matches!(t6.kind, TokenKind::Punct('(')));
        let t7 = lex.next_token();
        assert!(matches!(t7.kind, TokenKind::Identifier(ref s) if s == "x"));
        let t8 = lex.next_token();
        assert!(matches!(t8.kind, TokenKind::Punct(')')));
        let t9 = lex.next_token();
        assert!(matches!(t9.kind, TokenKind::EOF));
    }
}
#[derive(Debug, Default)]
pub struct LexerStage;
