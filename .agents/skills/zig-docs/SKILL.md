---
name: zig-docs
version: "0.16.0"
tags:
  - zig
  - systems-programming
  - low-level
  - memory-management
  - comptime
  - safety
  - c-interop
description: |
  Comprehensive Zig 0.16.0 reference covering all language features: primitive types,
  arrays, slices, pointers, structs, enums, unions, control flow, functions, errors,
  optionals, casting, comptime, builtins, memory management, build system, C interop,
  standard library, testing, targets, and style guide. Use whenever the user mentions
  Zig, systems programming, memory safety without GC, comptime metaprogramming,
  cross-compilation, C interop, or needs help with any Zig code, build configuration,
  or standard library usage.
---

# Zig Expert (v0.16.0)

**Official Documentation:** https://ziglang.org/documentation/0.16.0/

## Quick Reference

| Topic | File |
|-------|------|
| Primitive types, integers, floats, bool, values | `values-types.md` |
| Arrays, slices, sentinel-terminated, vectors | `arrays-slices.md` |
| Pointers, alignment, volatile, allowzero | `pointers.md` |
| Structs, default fields, extern, packed, tuples | `structs.md` |
| Enums, unions, tagged unions, non-exhaustive | `enums-unions.md` |
| if, switch, while, for, blocks, defer, unreachable | `control-flow.md` |
| Functions, inline fn, calling conventions, fn pointers | `functions.md` |
| Error sets, error unions, try, catch, error return traces | `errors.md` |
| Optionals, null, orelse, optional pointers | `optionals.md` |
| Type coercion, explicit casts, peer type resolution | `casting.md` |
| comptime, compile-time concepts, generic data structures | `comptime.md` |
| All builtin @functions reference | `builtins.md` |
| Memory management, allocators, ownership, lifetime | `memory-allocators.md` |
| Build modes, build.zig, compilation model, compile variables | `build-system.md` |
| C interop, @cImport, C types, exporting C libraries | `c-interop.md` |
| Standard library modules, data structures, I/O | `std-library.md` |
| Zig test, testing namespace, test output, leak detection | `testing.md` |
| Targets, cross-compilation, WebAssembly | `targets-wasm.md` |
| Style guide, naming conventions, keyword reference, grammar | `style-guide.md` |
| Inline assembly, global assembly, constraints, clobbers | `assembly.md` |
| opaque, result location semantics, zero bit types, atomics, async, containers | `advanced-concepts.md` |

## Core Philosophy

Zig is a **general-purpose programming language and toolchain** for maintaining robust, optimal, and reusable software. Key principles:

1. **No hidden control flow** — no hidden allocations, no hidden memory allocations
2. **No hidden memory allocations** — no garbage collector, no runtime
3. **No preprocessor, no macros** — comptime replaces macros
4. **No operator overloading** — what you see is what you get
5. **Manual memory management** — explicit allocators, no default allocator
6. **Compile-time metaprogramming** — comptime code runs at compile time
7. **Safety by default** — bounds checking, overflow detection in Debug mode
8. **C interop without FFI** — direct C header import, no bindings needed

## Hello World

```zig
const std = @import("std");

pub fn main(init: std.process.Init) !void {
    try std.Io.File.stdout().writeStreamingAll(init.io, "Hello, World!\n");
}
```

Simpler version using stderr (no error handling needed):

```zig
const std = @import("std");

pub fn main() void {
    std.debug.print("Hello, {s}!\n", .{"World"});
}
```

## Comments

```zig
// Line comments only — no multiline comments in Zig
// Each line can be tokenized out of context

/// Doc comment — used by compiler for package documentation
//! Top-level doc comment — describes the container itself
```

## Primitive Types Quick Reference

| Zig Type | C Equivalent | Description |
|----------|-------------|-------------|
| `i8`–`i128` | `int8_t`–`__int128` | Signed integers |
| `u8`–`u128` | `uint8_t`–`unsigned __int128` | Unsigned integers |
| `isize` | `intptr_t` | Pointer-sized signed integer |
| `usize` | `uintptr_t` / `size_t` | Pointer-sized unsigned integer |
| `f16`, `f32`, `f64`, `f80`, `f128` | Floats | Floating point |
| `bool` | `bool` | Boolean (`true`/`false`) |
| `void` | `void` | Unit type (`void{}`) |
| `anyopaque` | `void*` | Opaque type |
| `noreturn` | — | Never returns (break, continue, return, unreachable) |
| `type` | — | Type type |
| `anyerror` | — | Global error set |
| `comptime_int` | — | Compile-time integer |
| `comptime_float` | — | Compile-time float |

Arbitrary bit-width: `i7`, `u42`, etc. (max 65535 bits)

## Operators

| Operator | Description |
|----------|-------------|
| `+` `-` `*` `/` `%` | Arithmetic |
| `++` | Array/string concatenation (comptime) |
| `**` | Array repetition (comptime) |
| `&` `\|` `^` `~` | Bitwise AND, OR, XOR, NOT |
| `<<` `>>` | Bit shift |
| `&` `\|` `^` `!` | Boolean AND, OR, XOR, NOT |
| `==` `!=` `<` `>` `<=` `>=` | Comparison |
| `and` `or` `!` | Boolean keywords |
| `orelse` | Unwrap optional with fallback |
| `try` | Unwrap error union or return error |
| `catch` | Handle error union |

## Error Handling Quick Reference

```zig
// Error set
const FileError = error{ NotFound, PermissionDenied, Corrupted };

// Error union
fn readFile() FileError![]u8 { ... }

// Usage
const data = readFile() catch |err| switch (err) {
    error.NotFound => return,
    else => return err,
};

// or
const data = try readFile();
```

## Memory Management Quick Reference

```zig
const std = @import("std");

// FixedBufferAllocator — stack-allocated buffer
var buffer: [100]u8 = undefined;
var fba = std.heap.FixedBufferAllocator.init(&buffer);
const allocator = fba.allocator();

// Allocate
const memory = try allocator.alloc(u8, 100);
defer allocator.free(memory);

// General purpose
var gpa = std.heap.DebugAllocator(.{}).init;
defer _ = gpa.deinit();
const alloc = gpa.allocator();
```

## Build & Run

```bash
zig build-exe hello.zig       # Build executable
zig build hello.zig           # Use build system
zig test hello.zig            # Run tests
zig run hello.zig             # Build and run
zig fmt hello.zig             # Format code
zig std                       # Local std docs server
zig targets                   # List supported targets
```

## Installation

```bash
# Download from https://ziglang.org/download/
# Or use a version manager:
# Linux/macOS:
curl -L https://ziglang.org/download/0.16.0/zig-linux-x86_64-0.16.0.tar.xz | tar -xJ
# Windows:
# Download zig-windows-x86_64-0.16.0.zip and extract
```
