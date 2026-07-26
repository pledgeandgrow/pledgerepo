---
name: oxc-docs
version: "latest"
tags:
  - oxc
  - javascript
  - typescript
  - parser
  - linter
  - oxlint
  - rust
  - compiler
  - ast
  - ecmascript
description: |
  Comprehensive reference for the JavaScript Oxidation Compiler (Oxc) — a collection of
  high-performance JavaScript tools written in Rust. Covers parser design (lexer, AST,
  parser, errors, semantic analysis), architecture (parser, linter, test infrastructure,
  AST tools), ECMAScript specification and grammar, performance optimization techniques,
  terminology, and tool usage guide (parser, linter, formatter, transformer, minifier,
  resolver). Use whenever the user mentions oxc, oxlint, oxfmt, JavaScript parsing
  in Rust, AST design, or needs help with any oxc tool or contribution.
---

# Oxc Expert

**Official Documentation:** https://oxc.rs/docs/guide/introduction.html
**Learn (Parser Tutorial):** https://oxc.rs/docs/learn/parser_in_rust/intro.html
**GitHub:** https://github.com/oxc-project/oxc
**Playground:** https://playground.oxc.rs

## What is Oxc?

Oxc (The JavaScript Oxidation Compiler) is a collection of high-performance JavaScript tools written in Rust. It includes:

- **Parser** — Fastest JS/TS parser, 3x faster than swc parser
- **Linter (Oxlint)** — 840+ rules, 50-100x faster than ESLint, ESLint-compatible
- **Formatter (Oxfmt)** — Prettier-compatible formatter
- **Transformer** — TS type stripping, JSX, decorators, lowering (ES2026→ES2015)
- **Minifier** — Dead code elimination, mangling, syntax normalization
- **Resolver** — 28x faster than webpack/enhanced-resolve

## Quick Reference

| Topic | File |
|------|------|
| Lexer, AST, Parser, Errors, Semantic Analysis tutorial | `oxc-parser-tutorial.md` |
| Parser, Linter, Test Infrastructure, AST Tools architecture | `oxc-architecture.md` |
| ECMAScript Specification, Grammar quirks | `oxc-ecmascript.md` |
| Performance optimization techniques | `oxc-performance.md` |
| Tool usage guide (parser, linter, formatter, transformer, minifier, resolver) | `oxc-usage-guide.md` |

## Compiler Frontend Pipeline

```
Source Text --> Lexer --> Token --> Parser --> AST
```

## Philosophy

1. **Performance is a feature** — speed is a product requirement; regressions are bugs
2. **One toolchain, shared building blocks** — all tools built on shared components for consistency
3. **Correctness with clear boundaries** — differences from other tools are documented
4. **Practical developer experience** — sensible defaults, understandable config, stable output

## Core Design Principles

1. **Allocate less memory** — use memory arenas (bumpalo), string inlining (CompactString)
2. **Use fewer CPU cycles** — SIMD for whitespace, small enum sizes, cache-friendly layouts
3. **Zero-cost abstractions** — Rust's abstractions compile to efficient machine code
4. **Composition over inheritance** — no inheritance in Rust; use traits and composition
5. **Two-phase design** — parse to AST first, then semantic analysis separately

## Key Crates

| Crate | Purpose |
|-------|---------|
| `oxc_allocator` | Memory arena (bumpalo-based) for AST allocation |
| `oxc_ast` | AST node types and definitions |
| `oxc_parser` | Recursive descent parser |
| `oxc_semantic` | Semantic analysis (scopes, symbols, references) |
| `oxc_linter` | Linting engine (oxlint) |
| `oxc_diagnostics` | Error reporting with miette |
| `oxc_span` | Source span tracking (u32 offsets) |
| `oxc_formatter` | Code formatter (oxfmt) |
| `oxc_transformer` | TS/JSX/decorator/lowering transforms |
| `oxc_minifier` | Dead code elimination, mangling |
| `oxc_resolver` | Module resolver (enhanced-resolve compatible) |

## Terminology

| Term | Definition |
|------|-----------|
| **Binding** | A value assigned/bound within a scope |
| **Scope** | A block where bindings exist; hierarchical with parent/child |
| **Scope flags** | Metadata about scope (function, top-level, constructor, etc.) |
| **Symbol** | A binding wrapper with references to usage sites; assigned an ID |
| **Reference** | Usage of a symbol; flagged as read, write, or both |
| **Span** | Start/end offset (u32) of a node within source text |

## Usage Example

```rust
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;

let allocator = Allocator::default();
let source_text = "const x = 1;";
let source_type = oxc_span::SourceType::default();

// Phase 1: Parse to AST
let parser_result = Parser::new(&allocator, source_text, source_type).parse();

// Phase 2: Semantic analysis
let semantic_result = SemanticBuilder::new()
    .with_check_syntax_error(true)
    .build(&parser_result.program);
```

## Conformance

- **Test262**: 100% pass rate on ECMAScript conformance tests
- **Babel**: 99.62% compatibility with Babel parser tests
- **TypeScript**: 99.86% compatibility with TypeScript compiler tests

## npm Packages

| Package | Tool |
|---------|------|
| `oxc-parser` | Parser (Node.js binding) |
| `oxlint` | Linter |
| `oxc-transform` | Transformer (Node.js binding) |
| `oxc-minify` | Minifier (Node.js binding) |
| `oxc-resolver` | Resolver (Node.js binding) |
| `@oxlint/migrate` | ESLint config migration tool |
| `eslint-plugin-oxlint` | Disable overlapping ESLint rules |
| `unplugin-oxc` | Universal plugin for transform/minify |
| `unplugin-isolated-decl` | Isolated declarations plugin |
| `oxc-webpack-loader` | Webpack loader |

## References

- [ECMAScript Spec](https://tc39.es/ecma262/)
- [TypeScript 1.8 Spec](https://github.com/Boshen/TypeScript-Language-Specification/blob/main/TypeScript%20Language%20Specification.pdf)
- [JSX Spec](https://facebook.github.io/jsx/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/introduction.html)
- [Pratt Parsing Tutorial](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html)
- [V8: Understanding ECMAScript](https://v8.dev/blog/tags/understanding-ecmascript)
- [V8: Blazingly fast parsing](https://v8.dev/blog/scanner)
- [JS Syntactic Quirks](https://github.com/mozilla-spidermonkey/jsparagus/blob/master/js-quirks.md)
- [Crafting Interpreters](https://craftinginterpreters.com)
- [Faster than Rust and C++: PERFECT hash table](https://www.youtube.com/watch?v=DMQ_HcNSOAI)
- [Small strings in Rust](https://fasterthanli.me/articles/small-strings-in-rust)
- [Rust Atomics and Locks](https://marabos.nl/atomics/)
- [String Interners in Rust](https://dev.to/cad97/string-interners-in-rust-797)
