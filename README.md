# GraduaLuau

GraduaLuau is an experimental statically compiled language inspired by Luau.

The compiler executable is `gluauc`. The compiler has progressed from project-foundation
to a working compiler pipeline with multiple intermediate representations.

## Compiler Pipeline

The GraduaLuau compiler uses a multi-stage compilation pipeline:

1. **Lexing** - Tokenizes the source code
2. **Parsing** - Builds an Abstract Syntax Tree (AST)
3. **Semantic Analysis** - Performs type checking and semantic validation
4. **HIR (High-Level IR)** - Language-independent intermediate representation preserving high-level concepts
5. **MIR (Mid-Level IR)** - Low-level intermediate representation with explicit control flow
6. **LLVM IR Generation** - Translates MIR to LLVM Intermediate Representation
7. **Code Generation** - Produces native executables

## Commands

```bash
cargo run -p compiler --bin gluauc -- help
cargo run -p compiler --bin gluauc -- version
cargo run -p compiler --bin gluauc -- check examples/hello/main.glu
cargo run -p compiler --bin gluauc -- build examples/hello/main.glu
cargo run -p compiler --bin gluauc -- run examples/hello/main.glu
```

## Implementation Status

### Completed Stages
- ✅ Lexing (Lua/Luau-compatible tokenizer)
- ✅ Parsing (AST generation with error recovery)
- ✅ Semantic Analysis (type checking, constant evaluation, module resolution)
- ✅ HIR (High-Level Intermediate Representation)
- ✅ MIR (Mid-Level Intermediate Representation)
- ✅ LLVM IR Generation (text-based LLVM IR output)

### In Progress
- 🔄 Runtime implementation
- 🔄 Advanced optimizations
- 🔄 Full executable generation

### Future Work
- ⏳ LLVM optimization passes
- ⏳ Native code generation
- ⏳ Standard library
- ⏳ Advanced language features (coroutines, closures)

## Project Layout

```text
compiler/   Rust compiler crate and `gluauc` executable
runtime/    Reserved for native runtime support
stdlib/     Reserved for GraduaLuau standard library modules
examples/   Example GraduaLuau programs
tests/      Integration and component tests (lexer, parser, hir, mir, llvm)
docs/       Project documentation
spec/       Language specification
local/      Roadmap and planning documents
```

## Testing

The project includes comprehensive test suites for each compilation stage:

```bash
cargo test --test lexer           # Lexing tests
cargo test --test parser          # Parsing tests
cargo test --test hir             # HIR generation tests
cargo test --test mir             # MIR generation tests
cargo test --test llvm            # LLVM IR generation tests
```

## Development

The compiler is implemented in Rust and follows a modular architecture with separate
modules for each compilation stage. See the `local/` directory for detailed roadmaps
and implementation plans.
