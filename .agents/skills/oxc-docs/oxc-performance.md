# Oxc Performance Optimization

Performance engineering techniques used in the Oxc JavaScript compiler.
Key principle: **allocate less memory and use fewer CPU cycles**.

---

## AST Optimizations

### Memory Allocation (Memory Arena)

Problem: AST nodes allocated via `Box`/`Vec` are dropped individually — `drop` dominates execution time.

Solution: Memory arena (bumpalo) — allocate upfront in chunks, deallocate all at once.

```rust
pub enum Statement<'ast> {
    Expression(ExpressionNode<'ast>),
}
```

Side benefit: linear memory layout → cache-friendly traversal (nearby memory read into CPU cache in pages).

Result: **~20% performance improvement** from switching to memory arena.

### Enum Sizes

Rust enum size = union of all variants. Unboxed `Expression` (45 variants) = 200+ bytes. Box the variants:

```rust
pub enum Expression {
    AwaitExpression(Box<AwaitExpression>),
    YieldExpression(Box<YieldExpression>),
}
```

Keeps enum at 16 bytes (pointer + tag on 64-bit). Verify:

```rust
#[test]
fn no_bloat_enum_sizes() {
    use std::mem::size_of;
    assert_eq!(size_of::<Statement>(), 16);
    assert_eq!(size_of::<Expression>(), 16);
}
```

Result: **~10% speed-up** from boxing enum variants.

Find large types: `RUSTFLAGS=-Zprint-type-sizes cargo +nightly build -p crate --release`

### Span (u32 instead of usize)

```rust
// Before: 16 bytes on 64-bit
pub struct Node { pub start: usize, pub end: usize }

// After: 8 bytes
pub struct Node { pub start: u32, pub end: u32 }
```

u32 is safe because >4GB JavaScript files don't exist. Result: **up to 5% improvement on large files**.

### Strings and Identifiers

Can't use `&str` references to source text because JavaScript has escape sequences (`\251`, `\xA9`, `©` are the same). Must compute escaped values and allocate.

#### String Interning

`string-cache` (servo) — global `Mutex<Set>` locks on every string insert. In parallel parsing (rayon), CPU utilization drops to ~50%.

Removing `string-cache` improved parallel parsing by **~30%**.

The root cause: `Mutex<HashMap>` — locking the entire hashmap instead of individual buckets. PR to fix: https://github.com/servo/string-cache/pull/268

Other interners: `string-interner`, `lasso`, `lalrpop-intern`, `intaglio`, `strena` (single-threaded); `ustr` (multi-threaded). All still slower than string inlining due to hashing + indirection overhead.

#### String Inlining

Store short strings on the stack instead of heap:

```rust
enum Str {
    Static(&'static str),
    Inline(InlineRepresentation),  // same size as String
    Heap(String),
}
```

Crates compared:

| Crate | Inline bytes (64-bit) | O(1) clone |
|-------|----------------------|------------|
| `smol_str` | 23 | Yes |
| `smartstring` | 23 | No |
| `compact_str` | 24 | No |
| `flexstr` | 22 | Yes |

Oxc uses `compact_str::CompactStr` — reduced memory allocations significantly.

---

## Lexer Optimizations

### Token Size

Keep `Kind` enum as 1 byte, separate values into `TokenValue`:

```rust
pub struct Token<'a> {
    pub kind: Kind,           // 1 byte
    pub value: TokenValue     // separate
}
pub enum TokenValue<'a> {
    None,
    String(&'a str),  // 16 bytes (was 24 with String)
    Num(f64),
}
```

Using `&str` instead of `String` drops `TokenValue` from 32 to 24 bytes. Trade-off: lifetime annotations propagate through codebase.

Alternative: `Box<str>` for owned immutable data instead of `String`.

### Cow (Clone-on-Write)

`Cow` = "RefOrOwned" — return borrowed `&str` if no escapes, allocate only when needed:

```rust
fn finish(&mut self, lexer: &Lexer<'alloc>) -> &'alloc str {
    match self.value.take() {
        Some(arena_string) => arena_string.into_bump_str(),
        None => &self.start[..self.start.len() - lexer.chars.as_str().len()],
    }
}
```

99.9% of the time no allocation needed (escaped strings are rare).

### SIMD

Use portable SIMD to skip whitespace and multi-line comments in bulk. Inspired by RapidJSON's SIMD whitespace removal and Daniel Lemire's research.

Result: **a few percent improvement** for whitespace + comment skipping.

### Keyword Matching

84 keywords (with TypeScript). Simple `match` on string is ~1-2% of execution time.

LLVM already optimizes this: branches on string length first (switch on `i64`), then compares. The compiler is smarter than hand-rolled perfect hash functions.

Conclusion: simple keyword match is sufficient — not worth the effort of building a perfect hashmap in Rust.

---

## Linter Optimizations

### Parent Pointing Tree (indextree)

Problem: Can't go up the AST in Rust (no parent pointers in nodes).

Solution: Use `indextree::Arena` with an untyped AST wrapper:

```rust
struct Node<'a> { kind: AstKind<'a> }

enum AstKind<'a> {
    BlockStatement(&'a BlockStatement<'a>),
    ArrayExpression(&'a ArrayExpression<'a>),
    Class(&'a Class<'a>),
    // ...
}
```

Build tree via visitor pattern callbacks:

```rust
pub trait Visit<'a> {
    fn enter_node(&mut self, _kind: AstKind<'a>) {}
    fn leave_node(&mut self, _kind: AstKind<'a>) {}
}

impl<'a> Visit<'a> for TreeBuilder<'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) { self.push_ast_node(kind); }
    fn leave_node(&mut self, kind: AstKind<'a>) { self.pop_ast_node(); }
}
```

Linter becomes a simple loop over indextree nodes:

```rust
for node in nodes {
    match node.get().kind {
        AstKind::DebuggerStatement(stmt) => { /* report error */ }
        _ => {}
    }
}
```

Result: **84x faster than ESLint**. Linear memory access through arena + indextree is efficient.

### Processing Files in Parallel

Use `ignore` crate for directory traversal (supports `.gitignore`, `.eslintignore`). No `par_iter` available, so use rayon primitives with channels:

```rust
let walk = Walk::new(&self.options);
rayon::spawn(move || {
    walk.iter().for_each(|path| { tx_path.send(path).unwrap(); });
});
let linter = Arc::clone(&self.linter);
rayon::spawn(move || {
    while let Ok(path) = rx_path.recv() {
        let tx_error = tx_error.clone();
        let linter = Arc::clone(&linter);
        rayon::spawn(move || {
            if let Some(diagnostics) = Self::lint_path(&linter, &path) {
                tx_error.send(diagnostics).unwrap();
            }
            drop(tx_error);
        });
    }
});
```

Single-threaded printing of all diagnostics.

### Printing is Slow

`println!` locks stdout on every newline (line buffering). Use block buffering:

```rust
use std::io::{self, Write};
let stdout = io::stdout();
let mut handle = io::BufWriter::new(stdout);
writeln!(handle, "foo: {}", 42);
```

Or acquire the lock once:

```rust
let stdout = io::stdout();
let mut handle = stdout.lock();
writeln!(handle, "foo: {}", 42);
```

---

## Performance Summary

| Optimization | Improvement |
|-------------|-------------|
| Memory arena (bumpalo) | ~20% |
| Boxing enum variants | ~10% |
| u32 spans (instead of usize) | ~5% on large files |
| Removing string-cache (parallel) | ~30% parallel parsing |
| CompactStr (string inlining) | Significant memory reduction |
| SIMD whitespace/comment skipping | A few percent |
| Parent pointing tree (indextree) | 84x faster than ESLint |
