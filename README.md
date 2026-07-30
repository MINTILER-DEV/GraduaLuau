# GraduaLuau

GraduaLuau is an experimental statically compiled language inspired by Luau.

The compiler executable is `gluauc`. This repository is currently in the
project-foundation phase: command-line structure, diagnostics, source
management, compiler context, and module boundaries exist, while actual lexing,
parsing, semantic analysis, and code generation are intentionally not
implemented yet.

## Commands

```bash
cargo run -p compiler --bin gluauc -- help
cargo run -p compiler --bin gluauc -- version
cargo run -p compiler --bin gluauc -- check examples/hello/main.glu
cargo run -p compiler --bin gluauc -- build examples/hello/main.glu
cargo run -p compiler --bin gluauc -- run examples/hello/main.glu
```

## Project Layout

```text
compiler/   Rust compiler crate and `gluauc` executable
runtime/    Reserved for native runtime support
stdlib/     Reserved for GraduaLuau standard library modules
examples/   Example GraduaLuau programs
tests/      Future integration and component tests
docs/       Project documentation
spec/       Language specification
```
