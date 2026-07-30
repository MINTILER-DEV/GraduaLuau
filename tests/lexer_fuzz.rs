use compiler::lexer::Lexer;
use compiler::source::SourceManager;
use std::path::PathBuf;

// Simple deterministic pseudo-fuzz: linear congruential generator
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self { Self(seed) }
    fn next_u8(&mut self) -> u8 {
        // constants from Numerical Recipes
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.0 >> 24) as u8
    }
}

fn make_input(seed: u64, len: usize) -> String {
    let mut rng = Lcg::new(seed);
    let mut s = String::with_capacity(len);
    for _ in 0..len {
        let b = rng.next_u8();
        // map to common ascii range plus some control chars
        let ch = match b % 64 {
            0..=31 => (b % 26 + b'a') as char,
            v => (32 + v) as char,
        };
        s.push(ch);
    }
    s
}

#[test]
fn lexer_does_not_panic_on_random_input() {
    for seed in 0u64..128u64 {
        let input = make_input(seed, 256);
        let mut srcs = SourceManager::new();
        let id = srcs.add_file(PathBuf::from(format!("fuzz_{}.glu", seed)), input);
        let file = srcs.get(id).unwrap();
        let mut lex = Lexer::new(file);

        // iterate tokens to ensure we make progress and do not loop infinitely
        let mut steps = 0usize;
        loop {
            let tok = lex.next_token();
            steps += 1;
            assert!(steps < 10000, "lexer looped too long");
            if matches!(tok.kind, compiler::lexer::TokenKind::EOF) { break; }
        }
    }
}
