# Oxc Architecture

Architecture documentation for the Oxc project, covering the parser, linter,
test infrastructure, and AST tools.

---

## Parser Architecture

### AST Design Philosophy

Oxc AST differs from estree by removing ambiguous nodes and introducing distinct types:

| estree | Oxc | Usage |
|--------|-----|-------|
| `Identifier` | `BindingIdentifier` | Variable declarations (`let x = 1`) |
| `Identifier` | `IdentifierReference` | Variable usage (`console.log(x)`) |
| `Identifier` | `IdentifierName` | Property names (`obj.property`) |

```rust
pub struct BindingIdentifier<'a> {
    pub span: Span,
    pub name: Atom<'a>,
}

pub struct IdentifierReference<'a> {
    pub span: Span,
    pub name: Atom<'a>,
    pub reference_id: Cell<Option<ReferenceId>>,
}

pub struct IdentifierName<'a> {
    pub span: Span,
    pub name: Atom<'a>,
}
```

### Performance Architecture

#### How is it so fast

- **Memory Arena**: AST allocated in bumpalo for fast allocation/deallocation
- **String Optimization**: Short strings inlined by CompactString (≤24 bytes on stack)
- **Minimal Heap Usage**: No other heap allocations except the above two
- **Separation of Concerns**: Scope binding, symbol resolution, and some syntax errors delegated to semantic analyzer

#### Arena Allocation

```rust
use oxc_allocator::Allocator;

let allocator = Allocator::default();
let ast_node = allocator.alloc(Expression::NumericLiteral(
    allocator.alloc(NumericLiteral { value: 42.0, span: SPAN })
));
```

Benefits: O(1) allocation (pointer bump), O(1) deallocation (drop arena), cache-friendly linear layout, no fragmentation.

#### String Interning with CompactString

```rust
// Strings ≤ 24 bytes are stored inline (no heap allocation)
let short_name = CompactString::from("variableName");     // Stack allocated
let long_name = CompactString::from("a_very_long_variable_name_that_exceeds_limit"); // Heap allocated
```

### Two-Phase Design

1. **Parsing Phase**: Build AST structure with minimal semantic analysis
2. **Semantic Phase**: Scope analysis, symbol resolution, advanced error checking

```rust
// Phase 1: Parse to AST
let parser_result = Parser::new(&allocator, &source_text, source_type).parse();

// Phase 2: Semantic analysis
let semantic_result = SemanticBuilder::new()
    .with_check_syntax_error(true)
    .build(&parser_result.program);
```

### Parser Components

#### Lexer
- Token generation from source text
- SIMD optimization for whitespace skipping
- Context-aware: regex vs division operator disambiguation

#### Recursive Descent Parser
- Hand-written for maximum performance
- Advanced error recovery with meaningful messages
- Follows ECMAScript specification precisely

#### AST Builder
- Type safety via Rust's type system
- Direct arena allocation
- Builder pattern for node construction

### Conformance Strategy

| Test Suite | Pass Rate |
|-----------|-----------|
| Test262 | 100% |
| Babel | 99.62% |
| TypeScript | 99.86% |

### Error Handling

```rust
pub struct OxcDiagnostic {
    pub message: String,
    pub span: Span,
    pub severity: Severity,
    pub help: Option<String>,
}
```

Features: precise error locations, recovery strategies (continue after errors), helpful suggestions.

### Advanced Features

- **TypeScript Support**: Type stripping, decorator parsing, namespace support, JSX/TSX integration
- **Research Areas**: SIMD text processing, cache optimization, branch prediction, zero-copy parsing

---

## Linter Architecture

### apps/oxlint

The `oxlint` binary parses arguments and runs `LintRunner`.

### crates/oxc_diagnostics

`LintService` passes `mpsc::channel` Sender to `oxc_diagnostics` to receive lint results. Formatting done by the `miette` crate.

### crates/oxc_linter

#### LintService
- Holds `self.runtime` as `Arc<Runtime>`
- Runtime holds paths for linting
- Iterates over Runtime paths in parallel using **rayon**
- Sends `None` to finish

#### Runtime: process_path()
- Infers extension and content from path
- Supports `.[m|c]?[j|t]s` or `.[j|t]sx` extensions
- Exceptions for `.vue`, `.astro`, `.svelte` (partial support for script blocks)
- Executes linting and sends results to DiagnosticService

#### Runtime: process_source()
- Processes source with parser into AST
- Creates `LintContext` from `SemanticBuilder` and runs through `Linter`

### crates/oxc_semantic: SemanticBuilder

Builds semantic information from source:
- `source_text`, `nodes`, `classes`, `scopes`, `trivias`, `jsdoc`
- Returns `SemanticBuilderReturn`, only `Semantic` is passed to `LintContext`

### crates/oxc_linter: LintContext

Represents the context with `Semantic` as the main body. Includes getters and `diagnostic()` method.

### crates/oxc_linter: Linter

The `run()` function is the core of the linting process:
- Holds rules in `self.rules`
- Each rule implements three types of processing per trait
- Sequentially executes these three patterns

### Linter Example

Minimal code for creating a linter: https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/examples/linter.rs

---

## Test Infrastructure

### Parser Testing

#### Conformance
- Test262 (stage 4 + regex tests), Babel, TypeScript test suites
- Results stored in snapshot files for tracking changes:
  - `test262.snap`, `babel.snap`, `typescript.snap`
- All syntax errors written to snapshots for diffing

#### Fuzzing
Three fuzzers ensure no panics on random data:
1. `cargo fuzz` — sends random bytes to parser
2. `shift-fuzzer-js` — produces random but valid ASTs
3. `Automated-Fuzzer` by qarmin — actively reports crashes

#### Memory Safety
- Arena allocator (bumpalo) for AST — no `Drop` implementations on AST nodes
- Compile-time enforcement: oxc's allocator causes compile error if `Drop` types are allocated in arena
- Statically ensures no memory leaks from heap-owning types in arena

#### Unsafe Code
- Used for performance optimizations
- Contained in self-contained data structures with safe external APIs
- Miri run on every PR for crates containing unsafe

### Linter Testing

#### Snapshot Diagnostics
All linter diagnostics written to snapshot files for regression testing:
```
⚠ typescript-eslint(adjacent-overload-signatures): All "foo" signatures should be adjacent.
╭─[adjacent_overload_signatures.tsx:3:18]
2 │ function foo(s: string);
3 │ function foo(n: number);
  ·          ───
4 │ type bar = number;
5 │ function foo(sn: string | number) {}
  ·          ───
6 │ }
╰────
```

#### Ecosystem CI
`oxc-ecosystem-ci` runs oxlint against large repositories:
- rolldown, napi-rs, affine, preact, vscode, bbc/simorgh, elastic/kibana, DefinitelyTyped

### Idempotency

```js
let sourceText = "foo";
let printed = tool(sourceText);
let printed2 = tool(printed);
assert(printed == printed2);
```

All tools (parser, transformer, minifier, etc.) are idempotently tested on Test262, Babel, and TypeScript files.

### Integration Tests
Integration tests preferred over unit tests. Codecov reports line coverage.

### End to End
`monitor-oxc` tests against top 3000 npm packages. Tools rewrite `node_modules` files, then test imports still work.

---

## AST Tools

### Background

Rust compile time is slow; procedural macros for AST code generation worsen it. Both cold and incremental build times can regress drastically.

### RFC: Codegen AST Related Codes

Agreed requirements:
- No `build.rs` published to the user
- All generated code checked into git
- No nightly
- Rust code is source of truth — parse types marked `#[ast]`
- Avoid compile-time procedural macros

### Workflow

1. User changes code in repo
2. Watch change picks it up
3. Parse all types marked `#[ast]`
4. Record details of all AST types in a schema
5. Generate code from schema and save to files
