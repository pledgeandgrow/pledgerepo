# Style Guide & Keywords (Zig)

## Naming Conventions

### Avoid Redundancy in Names

```zig
// ❌ Bad
const FileError = error{ FileNotFound };
const FileNotFound = FileError.FileNotFound;

// ✅ Good
const FileError = error{ NotFound };
```

### Avoid Redundant Names in Fully-Qualified Namespaces

```zig
// ❌ Bad
std.debug.debugPrint("hello", .{});

// ✅ Good
std.debug.print("hello", .{});
```

### Refrain from Underscore Prefixes

```zig
// ❌ Avoid
const _internal = 5;
fn _helper() void {}

// ✅ Use regular names
const internal = 5;
fn helper() void {}
```

### Whitespace

- Use spaces, not tabs
- 4 spaces per indent level
- No trailing whitespace
- LF (Unix) line endings

### Names

| Category | Convention | Example |
|----------|-----------|---------|
| Types | PascalCase | `HashMap`, `ArrayList` |
| Functions | camelCase | `getFileSize`, `readFile` |
| Variables | camelCase | `fileCount`, `bufferSize` |
| Constants | camelCase | `defaultBufferSize` |
| Namespaces | camelCase | `std`, `crypto` |
| Enum fields | camelCase | `.ok`, `.notFound` |
| Struct fields | camelCase | `.x`, `.fileSize` |
| Error values | camelCase | `error.NotFound` |

### Examples

```zig
// ✅ Good Zig style
const std = @import("std");

const Point = struct {
    x: f32,
    y: f32,

    pub fn distance(self: Point, other: Point) f32 {
        const dx = self.x - other.x;
        const dy = self.y - other.y;
        return @sqrt(dx * dx + dy * dy);
    }
};

pub fn main() void {
    const p1 = Point{ .x = 0, .y = 0 };
    const p2 = Point{ .x = 3, .y = 4 };
    std.debug.print("distance: {d}\n", .{p1.distance(p2)});
}
```

## Doc Comment Guidance

```zig
/// Doc comments use triple slash
/// They document the declaration that follows

/// Returns the square root of x.
/// Returns NaN for negative inputs.
fn sqrt(x: f32) f32 { ... }

//! Top-level doc comments use `//!`
//! They describe the file/module itself
```

### Doc Comment Rules

- Use complete sentences
- Start with a capital letter
- End with a period
- Document all public declarations
- Explain *why*, not just *what*

## Source Encoding

Zig source files must be UTF-8 encoded.

## Keyword Reference

| Keyword | Description |
|---------|-------------|
| `addrspace` | Address space qualifier |
| `align` | Alignment qualifier |
| `allowzero` | Allow zero pointer |
| `and` | Boolean AND |
| `asm` | Inline assembly |
| `anyframe` | Async frame type (currently regressed) |
| `anytype` | Generic parameter type |
| `break` | Break from loop/block |
| `callconv` | Calling convention |
| `catch` | Error handling |
| `comptime` | Compile-time execution |
| `const` | Immutable binding |
| `continue` | Continue loop |
| `defer` | Defer execution to block exit |
| `else` | Else branch |
| `enum` | Enum declaration |
| `errdefer` | Defer on error exit |
| `error` | Error set declaration |
| `export` | Export symbol |
| `extern` | External symbol |
| `fn` | Function declaration |
| `for` | For loop |
| `if` | If expression |
| `inline` | Inline modifier |
| `linksection` | Link section qualifier |
| `noalias` | No-alias parameter |
| `noinline` | Prevent inlining |
| `noreturn` | Never returns type |
| `nosuspend` | Suspend async (currently regressed) |
| `opaque` | Opaque type |
| `or` | Boolean OR |
| `orelse` | Unwrap optional |
| `packed` | Packed struct/union |
| `pub` | Public visibility |
| `resume` | Resume async (currently regressed) |
| `return` | Return from function |
| `struct` | Struct declaration |
| `suspend` | Suspend async (currently regressed) |
| `switch` | Switch expression |
| `test` | Test declaration |
| `threadlocal` | Thread-local variable |
| `try` | Unwrap error union |
| `union` | Union declaration |
| `unreachable` | Unreachable code |
| `var` | Mutable binding |
| `volatile` | Volatile pointer |
| `while` | While loop |

## Zen of Zig

- Communicate intent precisely
- Edge cases matter
- Favor reading convenience over writing convenience
- The only data is masked data
- Don't hide what's happening
- Constants are values, not types
- Zero is a value, not a type
- Prefer composition to inheritance
- Prefer explicit to implicit
- Prefer correctness to convenience
- Prefer unique semantics to overlapping semantics
- Prefer debuggability to optimizability
- Prefer compile errors to runtime errors
- Optimize for the reader, not the writer
- Optimize for the maintainer, not the author
- Optimize for the future, not the past
- Optimize for the user, not the platform
- Optimize for the system, not the individual
- Optimize for the community, not the corporation
- Optimize for the world, not the country
- Optimize for the planet, not the moon
- Optimize for the universe, not the earth

## Containers

A container in Zig is any syntactical construct that acts as a namespace to hold variable and function declarations. Containers are also type definitions which can be instantiated.

**Container types:**
- Structs
- Enums
- Unions
- Opaques
- Zig source files themselves

Although containers use curly braces, they should not be confused with blocks or functions. **Containers do not contain statements** — only declarations.

## Grammar

Zig's grammar is defined in the language reference. Key points:

- Declarations are order-independent at container level
- Each line can be tokenized out of context (no multiline comments)
- Semicolons terminate statements
- Blocks are expressions (can return values via labeled break)
- Control flow constructs are expressions
- Identifiers: `[A-Za-z_][A-Za-z0-9_]*` or `@"string"`
- Builtin identifiers: `@[A-Za-z_][A-Za-z0-9_]*`
- Integer literals: `0b` (binary), `0o` (octal), `0x` (hex), decimal
- Float literals: `0x` hex float, decimal float, with `e`/`p` exponents
- String literals: `"..."` or `\\` multiline
- Char literals: `'...'`

### Grammar Summary

```
Root <- skip ContainerMembers eof
ContainerMembers <- container_doc_comment? ContainerDeclaration* (ContainerField COMMA)*
Decl <- (KEYWORD_export / KEYWORD_inline / KEYWORD_noinline)? FnProto (SEMICOLON / Block)
FnProto <- KEYWORD_fn IDENTIFIER? LPAREN ParamDeclList RPAREN ByteAlign? AddrSpace? LinkSection? CallConv? EXCLAMATIONMARK? TypeExpr
Block <- LBRACE BlockStatement* RBRACE
SwitchExpr <- KEYWORD_switch LPAREN Expr RPAREN LBRACE SwitchProngList RBRACE
```
