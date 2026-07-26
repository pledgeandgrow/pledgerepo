# Cargo and Build System

**Docs:** https://doc.rust-lang.org/cargo/index.html | https://doc.rust-lang.org/cargo/reference/index.html

## Cargo Commands

```bash
# Project management
cargo new my_project              # Create binary project
cargo new my_lib --lib            # Create library project
cargo init                        # Init project in current directory
cargo init --name custom_name     # Init with custom name

# Building
cargo build                       # Build in debug mode
cargo build --release             # Build in release mode
cargo check                       # Type-check without producing binary (faster)
cargo build --target x86_64-unknown-linux-gnu  # Cross-compile

# Running
cargo run                         # Build and run
cargo run -- args                 # Pass arguments to binary
cargo run --release               # Run release build

# Testing
cargo test                        # Run all tests
cargo test --test integration     # Run specific test file
cargo test -- --nocapture         # Show println! output
cargo bench                       # Run benchmarks

# Dependencies
cargo add serde                   # Add dependency
cargo add serde --features derive # Add with features
cargo add tokio --features full   # Add with specific features
cargo remove old_dep              # Remove dependency
cargo update                      # Update all dependencies
cargo update -p serde             # Update specific dependency
cargo tree                        # Show dependency tree
cargo tree -d                     # Show duplicate dependencies

# Code quality
cargo fmt                         # Format code
cargo fmt -- --check              # Check formatting without changes
cargo clippy                      # Run linter
cargo clippy --fix                # Auto-fix lint issues
cargo fix                         # Auto-fix compiler warnings

# Documentation
cargo doc                         # Generate documentation
cargo doc --open                  # Generate and open in browser
cargo doc --no-deps               # Only your crate, not dependencies

# Publishing
cargo login                       # Login to crates.io
cargo publish                     # Publish to crates.io
cargo publish --dry-run           # Test publish without uploading
cargo yank --vers 1.0.0           # Yank a version

# Workspace
cargo build --all                 # Build all workspace members
cargo test --all                  # Test all workspace members
cargo run -p specific_crate       # Run specific workspace member
```

## Cargo.toml Reference

```toml
[package]
name = "my_project"
version = "0.1.0"
edition = "2024"
authors = ["Author <email@example.com>"]
description = "A short description"
license = "MIT OR Apache-2.0"
repository = "https://github.com/user/repo"
homepage = "https://project.example.com"
documentation = "https://docs.example.com"
readme = "README.md"
keywords = ["api", "web"]
categories = ["web-programming", "api-bindings"]
rust-version = "1.75"  # Minimum supported Rust version

[dependencies]
serde = "1.0"
serde_json = "1"
tokio = { version = "1", features = ["full"] }
local_dep = { path = "../local_dep" }
git_dep = { git = "https://github.com/user/repo" }
git_dep_branch = { git = "https://github.com/user/repo", branch = "dev" }
git_dep_tag = { git = "https://github.com/user/repo", tag = "v1.0" }
optional_dep = { version = "1.0", optional = true }
renamed = { package = "actual_name", version = "1.0" }

[dev-dependencies]
proptest = "1"
criterion = "0.5"

[build-dependencies]
cc = "1.0"
bindgen = "0.69"

[features]
default = ["feature1"]
feature1 = []
feature2 = ["dep:optional_dep", "serde/derive"]
all_features = ["feature1", "feature2"]

[[bin]]
name = "my_app"
path = "src/main.rs"

[[bin]]
name = "tool"
path = "src/bin/tool.rs"

[lib]
name = "my_lib"
path = "src/lib.rs"
crate-type = ["cdylib", "rlib"]  # cdylib for FFI, rlib for Rust

[profile.dev]
opt-level = 0
debug = true
overflow-checks = true

[profile.release]
opt-level = 3
debug = false
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization, slower compile
strip = true        # Strip symbols
panic = "abort"     # Smaller binary, no unwinding

[profile.release.package."*"]
opt-level = 3  # Override for specific dependency

[workspace]
members = ["core", "api", "cli"]
resolver = "2"
```

## Build Profiles

| Profile | Optimization | Debug Info | Use |
|---------|-------------|------------|-----|
| `dev` (default) | 0 | full | Development |
| `release` | 3 | none | Production |
| `test` | 0 | full | Testing |
| `bench` | 3 | none | Benchmarking |

```toml
# Custom profile
[profile.release-with-debug]
inherits = "release"
debug = true

# Use with: cargo build --profile release-with-debug
```

## Features

```toml
[features]
default = ["json"]
json = ["serde", "serde_json"]
yaml = ["serde_yaml"]
full = ["json", "yaml"]

[dependencies]
serde = { version = "1", optional = true }
serde_json = { version = "1", optional = true }
serde_yaml = { version = "1", optional = true }
```

```bash
cargo build --features json,yaml    # Enable specific features
cargo build --all-features          # Enable all features
cargo build --no-default-features   # Disable default features
```

## build.rs — Build Script

```rust
// build.rs — runs before compilation
fn main() {
    // Set configuration
    println!("cargo:rustc-cfg=feature_x");

    // Set environment variable for compile-time use
    println!("cargo:rustc-env=MY_VAR=value");

    // Rerun if file changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // Link library
    println!("cargo:rustc-link-lib=mylib");

    // Compile C code
    cc::Build::new()
        .file("src/native.c")
        .compile("native");
}
```

## Conditional Compilation

```rust
// In code, use cfg attributes
#[cfg(feature = "json")]
mod json_support;

#[cfg(not(feature = "json"))]
mod json_support {
    pub fn parse(_s: &str) -> Option<()> { None }
}

// Platform-specific
#[cfg(target_os = "linux")]
fn platform_func() -> &'static str { "linux" }

#[cfg(target_os = "windows")]
fn platform_func() -> &'static str { "windows" }

#[cfg(target_arch = "wasm32")]
fn platform_func() -> &'static str { "wasm" }
```

## Workspaces

```toml
# Root Cargo.toml
[workspace]
members = ["crate_a", "crate_b"]
resolver = "2"

# Shared dependencies (Rust 1.64+)
[workspace.dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }

# In member crate Cargo.toml:
[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
```

## Cross-compilation

```bash
# Add target
rustup target add wasm32-unknown-unknown
rustup target add x86_64-unknown-linux-gnu

# Build for target
cargo build --target wasm32-unknown-unknown

# List installed targets
rustup target list --installed

# Cross-compilation with cross (uses Docker)
cargo install cross
cross build --target aarch64-unknown-linux-gnu
```

## Common Cargo.toml Patterns

```toml
# Binary + library in same package
[lib]
name = "my_lib"
path = "src/lib.rs"

[[bin]]
name = "my_app"
path = "src/main.rs"

# Workspace with shared metadata
[workspace.package]
version = "0.1.0"
edition = "2024"
authors = ["Team <team@example.com>"]
license = "MIT"

# In members:
[package]
version.workspace = true
edition.workspace = true
```
