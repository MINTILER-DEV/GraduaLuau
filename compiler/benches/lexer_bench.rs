use criterion::{black_box, criterion_group, criterion_main, Criterion};
use compiler::lexer::Lexer;
use compiler::source::SourceManager;
use std::path::PathBuf;

fn lex_input(text: &str) {
    let mut srcs = SourceManager::new();
    let id = srcs.add_file(PathBuf::from("bench.glu"), text.to_string());
    let file = srcs.get(id).unwrap();
    let mut lex = Lexer::new(file);

    loop {
        let tok = lex.next_token();
        if matches!(tok.kind, compiler::lexer::TokenKind::EOF) { break; }
        black_box(tok);
    }
}

fn lexer_benchmark(c: &mut Criterion) {
    // build a moderately large input with various constructs
    let mut s = String::new();
    for i in 0..500 {
        s.push_str(&format!("local x{} = {}\n", i, i));
        s.push_str(&format!("print(\"Hello {}\")\n", i));
        s.push_str("-- comment\n");
        s.push_str("spawn { Position = Vector3.zero }\n");
        s.push_str("player:SendMessage \"Hi\"\n");
    }

    c.bench_function("lexer_large_input", |b| b.iter(|| lex_input(black_box(&s))));
}

criterion_group!(benches, lexer_benchmark);
criterion_main!(benches);
