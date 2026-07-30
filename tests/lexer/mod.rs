use compiler::lexer::{Lexer, TokenKind};
use compiler::source::SourceManager;
use std::path::PathBuf;

#[test]
fn basic_tokenization() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("test.glu"), String::from("local x = 42\nprint(x)\n-- comment\n"));
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

#[test]
fn operator_tokenization() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(
        PathBuf::from("ops.glu"),
        String::from("a += 1\nb -= 2\nc *= 3\nd /= 4\ne %= 5\nf == g\nh ~= i\nj <= k\nl >= m\nn .. p\n"),
    );
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    let expected = [
        TokenKind::Identifier("a".into()),
        TokenKind::PlusEqual,
        TokenKind::NumberLiteral("1".into()),
        TokenKind::Identifier("b".into()),
        TokenKind::MinusEqual,
        TokenKind::NumberLiteral("2".into()),
        TokenKind::Identifier("c".into()),
        TokenKind::StarEqual,
        TokenKind::NumberLiteral("3".into()),
        TokenKind::Identifier("d".into()),
        TokenKind::SlashEqual,
        TokenKind::NumberLiteral("4".into()),
        TokenKind::Identifier("e".into()),
        TokenKind::PercentEqual,
        TokenKind::NumberLiteral("5".into()),
        TokenKind::Identifier("f".into()),
        TokenKind::EqualEqual,
        TokenKind::Identifier("g".into()),
        TokenKind::Identifier("h".into()),
        TokenKind::NotEqual,
        TokenKind::Identifier("i".into()),
        TokenKind::Identifier("j".into()),
        TokenKind::LessEqual,
        TokenKind::Identifier("k".into()),
        TokenKind::Identifier("l".into()),
        TokenKind::GreaterEqual,
        TokenKind::Identifier("m".into()),
        TokenKind::Identifier("n".into()),
        TokenKind::DotDot,
        TokenKind::Identifier("p".into()),
        TokenKind::EOF,
    ];

    for expected_kind in expected {
        let token = lex.next_token();
        assert_eq!(token.kind, expected_kind);
    }
}

#[test]
fn string_literal_quotes() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(
        PathBuf::from("strings.glu"),
        String::from("local a = \"hello\"\nlocal b = 'world'\n"),
    );
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "a"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "hello"));
    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "b"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "world"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn invalid_float_formats_are_split() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("nums.glu"), String::from("5.\n.25\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "5"));
    assert!(matches!(lex.next_token().kind, TokenKind::Dot));
    assert!(matches!(lex.next_token().kind, TokenKind::Dot));
    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "25"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
    assert_eq!(lex.diagnostics().len(), 2);
    assert_eq!(lex.diagnostics()[0].message(), "Invalid numeric literal");
    assert_eq!(lex.diagnostics()[1].message(), "Invalid numeric literal");
}

#[test]
fn invalid_leading_dot_number_reports_diagnostic() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("nums.glu"), String::from(".25\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Dot));
    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "25"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
    assert_eq!(lex.diagnostics().len(), 1);
    assert_eq!(lex.diagnostics()[0].message(), "Invalid numeric literal");
}

#[test]
fn single_line_comment_is_ignored() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("comment.glu"), String::from("local x = 1 -- comment\nprint(x)\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "x"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "1"));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftParen));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "x"));
    assert!(matches!(lex.next_token().kind, TokenKind::RightParen));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn block_comments_are_ignored() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("comment.glu"), String::from("--[[\nblock\ncomment\n]]\nprint(1)\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftParen));
    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "1"));
    assert!(matches!(lex.next_token().kind, TokenKind::RightParen));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn unterminated_block_comment_reports_diagnostic() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("comment.glu"), String::from("--[[\nunterminated\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    let token = lex.next_token();
    assert!(matches!(token.kind, TokenKind::EOF));
    assert_eq!(lex.diagnostics().len(), 1);
    assert_eq!(lex.diagnostics()[0].message(), "Unterminated block comment.");
}

#[test]
fn interpolated_string_tokens() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(
        PathBuf::from("interp.glu"),
        String::from("print(`Hello {name}!`)\n"),
    );
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftParen));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringStart));
    assert!(matches!(lex.next_token().kind, TokenKind::StringText(ref s) if s == "Hello "));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationStart));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "name"));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::StringText(ref s) if s == "!"));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::RightParen));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn interpolated_expression_lexing() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(
        PathBuf::from("interp.glu"),
        String::from("print(`{player.Health + 25}`)\n"),
    );
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftParen));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringStart));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationStart));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "player"));
    assert!(matches!(lex.next_token().kind, TokenKind::Dot));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "Health"));
    assert!(matches!(lex.next_token().kind, TokenKind::Plus));
    assert!(matches!(lex.next_token().kind, TokenKind::NumberLiteral(ref s) if s == "25"));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::RightParen));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn string_argument_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("shorthand.glu"), String::from("print \"Hello\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "Hello"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn table_argument_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(
        PathBuf::from("shorthand.glu"),
        String::from("spawn { Position = Vector3.zero }\n"),
    );
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "spawn"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftBrace));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "Position"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "Vector3"));
    assert!(matches!(lex.next_token().kind, TokenKind::Dot));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "zero"));
    assert!(matches!(lex.next_token().kind, TokenKind::RightBrace));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn chained_shorthand_calls() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("chain.glu"), String::from("factory \"Player\" \"Enemy\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "factory"));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "Player"));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "Enemy"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn mixed_parenthesis_and_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("mix.glu"), String::from("print(\"Hello\") \"World\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::LeftParen));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "Hello"));
    assert!(matches!(lex.next_token().kind, TokenKind::RightParen));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "World"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn method_call_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("method.glu"), String::from("player:SendMessage \"Hello\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "player"));
    assert!(matches!(lex.next_token().kind, TokenKind::Colon));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "SendMessage"));
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(ref s) if s == "Hello"));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn interpolated_string_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("interp_shorthand.glu"), String::from("print `Hello {name}`\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringStart));
    assert!(matches!(lex.next_token().kind, TokenKind::StringText(ref s) if s == "Hello "));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationStart));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "name"));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolationEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringEnd));
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn newline_separates_shorthand() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("newline.glu"), String::from("print\n\"Hello\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    let t1 = lex.next_token();
    assert!(matches!(t1.kind, TokenKind::Identifier(ref s) if s == "print"));
    let t2 = lex.next_token();
    assert!(matches!(t2.kind, TokenKind::StringLiteral(ref s) if s == "Hello"));

    // ensure spans preserve line information (tokens must be on different lines)
    let loc1 = file.location(t1.span.start()).unwrap();
    let loc2 = file.location(t2.span.start()).unwrap();
    assert_ne!(loc1.line, loc2.line);
    assert!(matches!(lex.next_token().kind, TokenKind::EOF));
}

#[test]
fn unexpected_character_reports_diagnostic_and_recovers() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("bad.glu"), String::from("local x = @\nprint(1)\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    // local x =
    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "x"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));

    // unexpected '@' should produce Unknown/diagnostic but lexer continues
    let t = lex.next_token();
    assert!(matches!(t.kind, TokenKind::Unknown('@')));
    assert!(lex.diagnostics().iter().any(|d| d.message().contains("Unexpected character")));

    // next tokens still parsed
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
}

#[test]
fn unterminated_string_reports_and_recovers() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("bad.glu"), String::from("local a = \"Hello\nlocal b = 1\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "a"));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));

    // string is unterminated; lexer should report and recover to next line
    assert!(matches!(lex.next_token().kind, TokenKind::StringLiteral(_)));
    assert!(lex.diagnostics().iter().any(|d| d.message().contains("Unterminated string literal")));

    // following statement still lexed
    assert!(matches!(lex.next_token().kind, TokenKind::Local));
}

#[test]
fn unknown_escape_sequence_reports_diagnostic() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("bad.glu"), String::from("local s = \"\\q\"\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    // consume to string
    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(_)));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    let t = lex.next_token();
    assert!(matches!(t.kind, TokenKind::StringLiteral(_)));
    assert!(lex.diagnostics().iter().any(|d| d.message().contains("Unknown escape sequence")));
}

#[test]
fn unterminated_interpolated_string_reports_and_recovers() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("bad.glu"), String::from("print `Hello {name\nprint(1)\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(ref s) if s == "print"));
    // interpolated start
    assert!(matches!(lex.next_token().kind, TokenKind::InterpolatedStringStart));
    // since unterminated, diagnostics should include message
    // consume tokens until EOF
    while !matches!(lex.next_token().kind, TokenKind::EOF) {}
    assert!(lex.diagnostics().iter().any(|d| d.message().contains("Unterminated interpolated string")));
}

#[test]
fn malformed_numeric_literal_reports_and_recovers() {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("nums.glu"), String::from("local x = 12.34.56\nprint(x)\n"));
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    // consume to number
    assert!(matches!(lex.next_token().kind, TokenKind::Local));
    assert!(matches!(lex.next_token().kind, TokenKind::Identifier(_)));
    assert!(matches!(lex.next_token().kind, TokenKind::Equal));
    let t = lex.next_token();
    assert!(matches!(t.kind, TokenKind::NumberLiteral(_)));
    assert!(lex.diagnostics().iter().any(|d| d.message().contains("Malformed numeric literal") || d.message().contains("Invalid numeric literal")));
}
