---
name: rust-docs
version: "1.97.0"
edition: "2024"
tags:
  - rust
  - systems-programming
  - memory-safety
  - ownership
  - borrow-checker
  - concurrency
  - traits
  - generics
description: |
  Comprehensive Rust 1.97.0 reference covering all language features: ownership, borrowing,
  lifetimes, types, variables, control flow, functions, closures, structs, enums, traits,
  generics, error handling, collections, strings, concurrency, async/await, modules, packages,
  macros, attributes, smart pointers, unsafe Rust, FFI, standard library, testing, and Cargo.
  Use whenever the user mentions Rust, ownership/borrowing, lifetimes, traits, generics,
  async/await, Cargo, unsafe Rust, or needs help with any Rust code, build configuration,
  or standard library usage.
---

# Rust Expert (v1.97.0, Edition 2024)

**Official Documentation:**
- Standard Library: https://doc.rust-lang.org/std/index.html
- The Rust Book: https://doc.rust-lang.org/book/index.html
- Rust Reference: https://doc.rust-lang.org/stable/reference/introduction.html

## Quick Reference

| Topic | File |
|-------|------|
| Ownership, borrowing, lifetimes, move semantics | `ownership-borrowing.md` |
| Primitive types, variables, mutability, type inference | `types-variables.md` |
| if, match, loops, blocks, expressions | `control-flow.md` |
| Functions, closures, function pointers, iterators | `functions-closures.md` |
| Structs, enums, pattern matching, unions | `structs-enums.md` |
| Traits, generics, trait objects, impl Trait | `traits-generics.md` |
| Result, Option, panic, error handling patterns | `error-handling.md` |
| Vec, HashMap, String, str, slices, collections | `collections-strings.md` |
| Threads, channels, Mutex, Arc, async/await, Send/Sync | `concurrency-async.md` |
| Modules, crates, use, paths, visibility, workspaces | `modules-packages.md` |
| Macros (declarative & procedural), attributes, derive | `macros-attributes.md` |
| Box, Rc, Arc, RefCell, Cell, Cow, smart pointers | `smart-pointers.md` |
| Unsafe Rust, raw pointers, FFI, extern, unions | `unsafe-ffi.md` |
| Standard library modules, I/O, fs, net, process | `std-library.md` |
| Unit tests, integration tests, doc tests, benchmarks | `testing.md` |
| Cargo, build profiles, dependencies, features, workspaces | `cargo-build-system.md` |

## Core Philosophy

Rust is a **systems programming language** focused on safety, speed, and concurrency. Key principles:

1. **Memory safety without GC** — ownership and borrowing enforce safety at compile time
2. **Zero-cost abstractions** — high-level constructs compile to efficient machine code
3. **No data races** — Send/Sync traits and borrow checker prevent data races at compile time
4. **No null** — Option<T> replaces null pointers
5. **No exceptions** — Result<T, E> for recoverable errors, panic for unrecoverable
6. **Type inference** — compiler infers types, explicit annotations rarely needed
7. **Traits over inheritance** — composition and trait bounds instead of class hierarchies
8. **Fearless concurrency** — compiler guarantees thread safety

## Hello World

```rust
fn main() {
    println!("Hello, World!");
}
```

## Comments

```rust
// Line comment

/// Doc comment — used by rustdoc for documentation
/** Block doc comment */

//! Inner doc comment — describes the containing item (module/crate)
```

## Primitive Types Quick Reference

| Rust Type | Description |
|-----------|-------------|
| `i8`–`i128`, `isize` | Signed integers |
| `u8`–`u128`, `usize` | Unsigned integers |
| `f16`, `f32`, `f64`, `f128` | Floating point |
| `bool` | Boolean (`true`/`false`) |
| `char` | Unicode scalar value (4 bytes) |
| `str` | String slice (UTF-8) |
| `[T; N]` | Fixed-size array |
| `[T]` | Dynamically sized slice |
| `(T, U, ..)` | Tuple |
| `()` | Unit type (empty tuple) |
| `!` | Never type (diverges) |

## Operators

| Operator | Description |
|----------|-------------|
| `+` `-` `*` `/` `%` | Arithmetic |
| `&` `\|` `^` `!` `<<` `>>` | Bitwise AND, OR, XOR, NOT, shifts |
| `&&` `\|\|` `!` | Boolean AND, OR, NOT |
| `==` `!=` `<` `>` `<=` `>=` | Comparison |
| `=` `+=` `-=` `*=` `/=` `%=` | Assignment and compound assignment |
| `&` `*` | Borrow reference, dereference |
| `?` | Error propagation (try operator) |
| `as` | Type casting |
| `..` `..=` | Range (exclusive, inclusive) |
| `\|` | Closure start (pipe) |

## Error Handling Quick Reference

```rust
// Result type
fn read_file(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

// Usage with ?
fn process() -> Result<(), std::io::Error> {
    let content = read_file("file.txt")?;
    println!("{}", content);
    Ok(())
}

// Usage with match
match read_file("file.txt") {
    Ok(content) => println!("{}", content),
    Err(e) => eprintln!("Error: {}", e),
}

// Option type
fn find_first_even(nums: &[i32]) -> Option<i32> {
    nums.iter().find(|&&n| n % 2 == 0).copied()
}
```

## Ownership Quick Reference

```rust
// Move semantics — ownership transferred
let s1 = String::from("hello");
let s2 = s1; // s1 is moved, no longer valid
// println!("{}", s1); // ERROR: value borrowed after move

// Borrow — immutable reference
let s3 = String::from("world");
let r1 = &s3; // borrow
let r2 = &s3; // multiple immutable borrows OK
println!("{} {}", r1, r2);

// Mutable borrow — exclusive
let mut s4 = String::from("foo");
let r3 = &mut s4; // mutable borrow
r3.push_str("bar");
println!("{}", s4); // can use s4 again after r3 scope ends

// Clone — explicit deep copy
let s5 = s4.clone();
```

## Build & Run

```bash
cargo new my_project      # Create new project
cargo build               # Build (debug mode)
cargo build --release     # Build (release mode)
cargo run                 # Build and run
cargo test                # Run tests
cargo check               # Type-check without producing binary
cargo fmt                 # Format code
cargo clippy              # Lint code
cargo doc --open          # Generate and view documentation
cargo add serde           # Add dependency
cargo update              # Update dependencies
```

## Installation

```bash
# Install via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows: download rustup-init.exe from https://rustup.rs
# Or use winget:
winget install Rustlang.Rustup

# Update Rust
rustup update

# Install specific toolchain
rustup toolchain install stable
rustup default stable
```
