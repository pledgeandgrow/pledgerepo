# Oxc Parser Tutorial in Rust

Guide to writing a JavaScript parser in Rust, based on the oxc documentation.
Covers: Lexer, AST, Parser, Errors, Semantic Analysis.

---

## Lexer

The lexer (tokenizer/scanner) transforms source text into tokens consumed by the parser.

### Token Structure

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,  // Start offset in source
    pub end: usize,    // End offset in source
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Eof,   // End of file
    Plus,
}
```

### Lexer Abstraction

```rust
use std::str::Chars;

struct Lexer<'a> {
    source: &'a str,
    chars: Chars<'a>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, chars: source.chars() }
    }
}
```

The lifetime `'a` indicates the iterator references a `&'a str`.

### Reading Tokens

```rust
impl<'a> Lexer<'a> {
    fn read_next_kind(&mut self) -> Kind {
        while let Some(c) = self.chars.next() {
            match c {
                '+' => return Kind::Plus,
                _ => {}
            }
        }
        Kind::Eof
    }

    fn read_next_token(&mut self) -> Token {
        let start = self.offset();
        let kind = self.read_next_kind();
        let end = self.offset();
        Token { kind, start, end }
    }

    /// Get the byte offset from the source text
    fn offset(&self) -> usize {
        self.source.len() - self.chars.as_str().len()
    }
}
```

`as_str().len()` is O(1) — it returns a pointer to a string slice and `.len()` reads the metadata (pointer + length).

### Peek (for multi-character operators)

To tokenize `++` or `+=`, a `peek` helper is needed:

```rust
fn peek(&self) -> Option<char> {
    self.chars.clone().next()
}
```

Cloning the `Chars` iterator is cheap — it just copies the tracking and boundary index.

Real-world example from jsparagus:

```rust
'+' => match self.peek() {
    Some('+') => { self.chars.next(); return self.set_result(TerminalId::Increment, ...); }
    Some('=') => { self.chars.next(); return self.set_result(TerminalId::AddAssign, ...); }
    _ => return self.set_result(TerminalId::Plus, ...),
},
```

### JavaScript Lexing

#### Comments
Comments have no semantic meaning. Skip them for runtimes, but preserve for linters/bundlers.

#### Identifiers and Unicode
Per ECMAScript spec (Chapter 12.6), identifiers follow Unicode Standard Annex #31:
- `IdentifierStartChar` :: `UnicodeIDStart` (any code point with "ID_Start" property)
- `IdentifierPartChar` :: `UnicodeIDContinue` (any code point with "ID_Continue" property)

`var ಠ_ಠ` is valid (ಠ has ID_Start), but `var 🦀` is not (🦀 lacks ID_Start).

Use the `unicode-id-start` crate: `unicode_id_start::is_id_start(char)` and `unicode_id_start::is_id_continue(char)`.

#### Keywords

Keywords (`if`, `while`, `for`) need their own token kind enum variants to avoid string comparisons in the parser:

```rust
pub enum Kind {
    Identifier,
    If,
    While,
    For,
}
```

`undefined` is not a keyword — no need to add it.

```rust
fn match_keyword(&self, ident: &str) -> Kind {
    if ident.len() == 1 || ident.len() > 10 { return Kind::Identifier; }
    match ident {
        "if" => Kind::If,
        "while" => Kind::While,
        "for" => Kind::For,
        _ => Kind::Identifier,
    }
}
```

### Token Values

```rust
pub enum TokenValue {
    None,
    Number(f64),
    String(String),  // or Atom for string interning
}

pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub value: TokenValue,
}
```

- **Strings**: `self.source[token.start..token.end].to_string()`
- **Numbers**: `self.source[token.start..token.end].parse::<f64>()`

### Rust Optimizations

#### Smaller Tokens
Do NOT put values inside `Kind` enum — it bloats the enum from 1 byte to many bytes. Keep `Kind` as a small 1-byte enum and store values separately in `TokenValue`.

#### String Interning
`String` is expensive: heap-allocated + O(n) comparison. Use string interning:
- `string-cache` crate provides `Atom` type and `atom!("string")` macro
- String comparison becomes O(1): `matches!(value, TokenValue::String(atom!("string")))`

```rust
pub enum TokenValue {
    None,
    Number(f64),
    String(Atom),  // interned string
}
```

---

## AST (Abstract Syntax Tree)

### Getting Familiar with AST

ASTExplorer (https://astexplorer.net/) is the best way to visualize ASTs. For `var a`:

```json
{
  "type": "Program",
  "start": 0, "end": 5,
  "body": [{
    "type": "VariableDeclaration",
    "start": 0, "end": 5,
    "declarations": [{
      "type": "VariableDeclarator",
      "start": 4, "end": 5,
      "id": { "type": "Identifier", "start": 4, "end": 5, "name": "a" },
      "init": null
    }],
    "kind": "var"
  }]
}
```

### estree Standard

estree is the community standard AST specification. Basic building block:

```rust
#[derive(Debug, Default, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Node {
    pub start: usize,
    pub end: usize,
}
```

### AST for `var a`

```rust
pub struct Program {
    pub node: Node,
    pub body: Vec<Statement>,
}

pub enum Statement {
    VariableDeclarationStatement(VariableDeclaration),
}

pub struct VariableDeclaration {
    pub node: Node,
    pub declarations: Vec<VariableDeclarator>,
}

pub struct VariableDeclarator {
    pub node: Node,
    pub id: BindingIdentifier,
    pub init: Option<Expression>,
}

pub enum Expression { }  // Empty for now
```

Rust has no inheritance — use "composition over inheritance" by adding `Node` to each struct.

`Box` is needed for self-referential types:

```rust
pub enum Expression {
    AwaitExpression(Box<AwaitExpression>),
    YieldExpression(Box<YieldExpression>),
}
```

### Rust Optimizations

#### Memory Arena (bumpalo)

Every `Box` and `Vec` is allocated/dropped individually — slow. Use a memory arena for bulk allocation/deallocation:

```rust
use bumpalo::collections::Vec;
use bumpalo::boxed::Box;

pub enum Expression<'a> {
    AwaitExpression(Box<'a, AwaitExpression>),
    YieldExpression(Box<'a, YieldExpression>),
}
```

Benefits: O(1) allocation (pointer bump), O(1) deallocation (drop arena), cache-friendly linear layout.

#### Enum Size

Rust enum size = union of all variants. An unboxed `Expression` enum with 45 variants can be 200+ bytes. Box the variants to keep enums at 16 bytes:

```rust
pub enum Expression {
    AwaitExpression(Box<AwaitExpression>),
    YieldExpression(Box<YieldExpression>),
}
```

Verify with `static_assert_size!`:

```rust
#[test]
fn no_bloat_enum_sizes() {
    use std::mem::size_of;
    assert_eq!(size_of::<Statement>(), 16);
    assert_eq!(size_of::<Expression>(), 16);
}
```

Find large types with: `RUSTFLAGS=-Zprint-type-sizes cargo +nightly build -p crate --release`

#### JSON Serialization (serde)

```rust
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type")]
#[cfg_attr(feature = "estree", serde(rename = "Identifier"))]
pub struct IdentifierReference {
    #[serde(flatten)]
    pub node: Node,
    pub name: Atom,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Expression<'a> { ... }
```

- `serde(tag = "type")` — adds `"type": "..."` field
- `cfg_attr + serde(rename)` — rename structs for estree compatibility
- `serde(untagged)` — no extra wrapper object for enums
- `serde(flatten)` — inline node fields into parent

---

## Parser

### Helper Functions

```rust
impl<'a> Parser<'a> {
    fn start_node(&self) -> Node {
        let token = self.cur_token();
        Node::new(token.start, 0)
    }

    fn finish_node(&self, node: Node) -> Node {
        Node::new(node.start, self.prev_token_end)
    }

    fn cur_token(&self) -> &Token { &self.cur_token }
    fn cur_kind(&self) -> Kind { self.cur_token.kind }

    /// Checks if the current index has token `Kind`
    fn at(&self, kind: Kind) -> bool { self.cur_kind() == kind }

    /// Advance if we are at `Kind`
    fn bump(&mut self, kind: Kind) {
        if self.at(kind) { self.advance(); }
    }

    /// Advance any token
    fn bump_any(&mut self) { self.advance(); }

    /// Advance and return true if we are at `Kind`, return false otherwise
    fn eat(&mut self, kind: Kind) -> bool {
        if self.at(kind) { self.advance(); return true; }
        false
    }

    fn advance(&mut self) {
        let token = self.lexer.next_token();
        self.prev_token_end = self.cur_token.end;
        self.cur_token = token;
    }
}
```

### Parse Functions

```rust
impl<'a> Parser<'a> {
    pub fn parse(&mut self) -> Program {
        let stmt = self.parse_debugger_statement();
        let body = vec![stmt];
        Program {
            node: Node { start: 0, end: self.source.len() },
            body,
        }
    }

    fn parse_debugger_statement(&mut self) -> Statement {
        let node = self.start_node();
        self.bump_any();
        Statement::DebuggerStatement { node: self.finish_node(node) }
    }
}
```

### Parsing Expressions

Expression grammar is deeply nested and recursive — can cause stack overflow. Use **Pratt Parsing** to avoid recursion. See: [Simple but Powerful Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html).

### Lists (Template Method Pattern)

Use traits to avoid duplication for comma-separated lists:

```rust
fn parse_list(&mut self, p: &mut Parser) -> CompletedMarker {
    let elements = self.start_list(p);
    let mut progress = ParserProgress::default();
    let mut first = true;
    while !p.at(EOF) && !self.is_at_list_end(p) {
        if first { first = false; }
        else {
            self.expect_separator(p);
            if self.allow_trailing_separating_element() && self.is_at_list_end(p) { break; }
        }
        progress.assert_progressing(p);  // prevents infinite loops
        let parsed_element = self.parse_element(p);
        if self.recover(p, parsed_element).is_err() { break; }
    }
    self.finish_list(p, elements)
}
```

### Cover Grammar

Convert `Expression` to `BindingPattern` using a trait:

```rust
pub trait CoverGrammar<'a, T>: Sized {
    fn cover(value: T, p: &mut Parser<'a>) -> Result<Self>;
}

impl<'a> CoverGrammar<'a, Expression<'a>> for BindingPattern<'a> {
    fn cover(expr: Expression<'a>, p: &mut Parser<'a>) -> Result<Self> {
        match expr {
            Expression::Identifier(ident) => Self::cover(ident.unbox(), p),
            Expression::ObjectExpression(expr) => Self::cover(expr.unbox(), p),
            Expression::ArrayExpression(expr) => Self::cover(expr.unbox(), p),
            _ => Err(()),
        }
    }
}
```

Call: `BindingPattern::cover(expression)`

### TypeScript

No specification — the TypeScript parser is in a single file. Key challenges:

- **JSX vs TSX**: `let foo = <string> bar;` is a type assertion in TS but syntax error in TSX
- **Lookahead**: May need to peek 5+ tokens (e.g., for `TSIndexSignature` vs `TSPropertySignature`)
- **Arrow expressions**: Use lookahead (fast path) + backtracking for parenthesized arrow functions

---

## Errors

### Error Trait with thiserror

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SyntaxError>;

#[derive(Debug, Error)]
pub enum SyntaxError {
    #[error("Unexpected Token")]
    UnexpectedToken,
    #[error("Expected a semicolon or an implicit semicolon after a statement, but found none")]
    AutoSemicolonInsertion,
    #[error("Unterminated multi-line comment")]
    UnterminatedMultiLineComment,
}
```

### Expect Helper

```rust
pub fn expect(&mut self, kind: Kind) -> Result<()> {
    if self.at(kind) { return Err(SyntaxError::UnexpectedToken); }
    self.advance(kind);
    Ok(())
}

fn parse_debugger_statement(&mut self) -> Result<Statement> {
    let node = self.start_node();
    self.expect(Kind::Debugger)?;
    Ok(Statement::DebuggerStatement { node: self.finish_node(node) })
}
```

### Fancy Error Reports with miette

```toml
[dependencies]
miette = { version = "5", features = ["fancy"] }
```

```rust
pub fn main() -> Result<()> {
    let source_code = "".to_string();
    let file_path = "test.js".to_string();
    let mut parser = Parser::new(&source_code);
    parser.parse().map_err(|error| {
        miette::Error::new(error)
            .with_source_code(miette::NamedSource::new(file_path, source_code))
    })
}
```

---

## Semantic Analysis

### Context (Grammar Parameters)

ECMAScript has 5 context parameters: `[In]`, `[Return]`, `[Yield]`, `[Await]`, `[Default]`.

Track these during parsing (example from Biome):

```rust
pub(crate) struct ParsingContextFlags: u8 {
    const IN_GENERATOR = 1 << 0;    // Yield parameter
    const IN_FUNCTION = 1 << 1;
    const IN_CONSTRUCTOR = 1 << 2;
    const IN_ASYNC = 1 << 3;        // Async/Await parameter
    const TOP_LEVEL = 1 << 4;
    const BREAK_ALLOWED = 1 << 5;
    const CONTINUE_ALLOWED = 1 << 6;
}
```

### Scope Tree

Use `indextree` for a parent-pointing scope tree:

```rust
use indextree::{Arena, NodeId};
use bitflags::bitflags;

pub type Scopes = Arena<Scope>;
pub type ScopeId = NodeId;

bitflags! {
    #[derive(Default)]
    pub struct ScopeFlags: u8 {
        const TOP = 1 << 0;
        const FUNCTION = 1 << 1;
        const ARROW = 1 << 2;
        const CLASS_STATIC_BLOCK = 1 << 4;
        const VAR = Self::TOP.bits | Self::FUNCTION.bits | Self::CLASS_STATIC_BLOCK.bits;
    }
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub strict_mode: bool,
    pub flags: ScopeFlags,
    pub lexical: IndexMap<Atom, SymbolId, FxBuildHasher>,
    pub var: IndexMap<Atom, SymbolId, FxBuildHasher>,
    pub function: IndexMap<Atom, SymbolId, FxBuildHasher>,
}
```

### ScopeBuilder

```rust
pub struct ScopeBuilder {
    scopes: Scopes,
    root_scope_id: ScopeId,
    current_scope_id: ScopeId,
}

impl ScopeBuilder {
    pub fn enter_scope(&mut self, flags: ScopeFlags) {
        let mut strict_mode = self.scopes[self.root_scope_id].get().strict_mode;
        let parent_scope = self.current_scope();
        if !strict_mode && parent_scope.flags.intersects(ScopeFlags::FUNCTION)
            && parent_scope.strict_mode { strict_mode = true; }
        let scope = Scope::new(flags, strict_mode);
        let new_scope_id = self.scopes.new_node(scope);
        self.current_scope_id.append(new_scope_id, &mut self.scopes);
        self.current_scope_id = new_scope_id;
    }

    pub fn leave_scope(&mut self) {
        self.current_scope_id = self.scopes[self.current_scope_id].parent().unwrap();
    }
}
```

The scope tree can be built inside the parser (for performance) or in a separate AST pass (for simplicity).

### Visitor Pattern

For building scope tree in a separate pass, use the Visitor Pattern for depth-first preorder traversal:

```rust
pub trait Visit<'a>: Sized {
    fn enter_node(&mut self, _kind: AstKind<'a>) {}
    fn leave_node(&mut self, _kind: AstKind<'a>) {}
    fn visit_block_statement(&mut self, stmt: &'a BlockStatement<'a>) {
        let kind = AstKind::BlockStatement(stmt);
        self.enter_node(kind);
        self.visit_statements(&stmt.body);
        self.leave_node(kind);
    }
}
```
