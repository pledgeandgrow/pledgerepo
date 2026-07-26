# Modules, Crates, and Packages

**Docs:** https://doc.rust-lang.org/stable/reference/items/modules.html | https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html

## Crate and Package

- **Package**: A Cargo project with one or more crates, defined by `Cargo.toml`
- **Crate**: A compilation unit — either a binary or a library
- **Module**: A namespace within a crate for organizing code

```toml
# Cargo.toml — defines the package
[package]
name = "my_project"
version = "0.1.0"
edition = "2024"

[lib]
name = "my_lib"
path = "src/lib.rs"

[[bin]]
name = "my_app"
path = "src/main.rs"
```

## Module System

```rust
// src/lib.rs
mod garden;  // loads src/garden.rs or src/garden/mod.rs

pub mod math {
    pub fn add(a: i32, b: i32) -> i32 { a + b }
    fn private_helper() {}  // private by default

    pub mod geometry {
        pub fn area(width: f64, height: f64) -> f64 {
            width * height
        }
    }
}

// Use the module
use crate::math::add;
use crate::math::geometry::area;
```

## File-based Modules

```
src/
├── main.rs          // crate root for binary
├── lib.rs           // crate root for library
├── garden/
│   ├── mod.rs       // module garden
│   ├── vegetable.rs // submodule garden::vegetable
│   └── fruit.rs     // submodule garden::fruit
└── utils.rs         // module utils
```

```rust
// src/main.rs
mod garden;  // loads garden/mod.rs
mod utils;   // loads utils.rs

use garden::vegetable;
```

## use Statements

```rust
// Import specific items
use std::collections::HashMap;
use std::io::{self, Read, Write};

// Rename with as
use std::collections::HashMap as Map;

// Import all with glob (use sparingly)
use std::io::*;

// Nested paths (Rust 2018+)
use std::{collections::HashMap, fs::File, io::Read};

// Re-export with pub use
pub use crate::math::add;
```

## Visibility

```rust
// pub — visible everywhere
// (default) — visible only in current module
// pub(crate) — visible within the crate
// pub(super) — visible in parent module
// pub(in path) — visible in specified module tree

pub fn public_function() {}
fn private_function() {}
pub(crate) fn crate_function() {}
pub(super) fn parent_function() {}

mod inner {
    pub(in crate::inner) fn restricted() {}
}
```

## Paths

```rust
// crate:: — absolute path from crate root
// self:: — current module
// super:: — parent module

mod a {
    pub mod b {
        pub fn func() {
            // From here:
            crate::a::b::func();  // absolute
            self::func();          // current module
            super::b::func();      // parent module
        }
    }
}
```

## Workspaces

```toml
# Cargo workspace — Cargo.toml at root
[workspace]
members = [
    "core",
    "api",
    "cli",
]
resolver = "2"

# Each member has its own Cargo.toml:
# core/Cargo.toml
[package]
name = "core"
version = "0.1.0"

# api/Cargo.toml
[package]
name = "api"
version = "0.1.0"
[dependencies]
core = { path = "../core" }
```

## External Dependencies

```toml
# Cargo.toml
[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
my_lib = { path = "../my_lib" }
my_git = { git = "https://github.com/user/repo" }
my_git_branch = { git = "https://github.com/user/repo", branch = "dev" }
```

```rust
// Using external crates
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    value: i32,
}
```

## Editions

```toml
# Cargo.toml
[package]
edition = "2024"  # Current edition (2021, 2018, 2015 also exist)
```

Edition changes are about language semantics, not compiler features. The compiler supports all editions simultaneously. Edition 2024 includes changes like:
- `gen` and `try` reserved keywords
- Unsafe attributes
- Tail expression temporary scope changes
- `impl Trait` lifetime capture rules

## Prelude

The prelude is a set of items automatically imported into every module. The standard prelude includes `Option`, `Result`, `Vec`, `String`, `Box`, `Iterator`, `Clone`, `Copy`, `Debug`, `Display`, `Drop`, and more.

Edition 2024 prelude also includes `Future` and `IntoFuture`.
