# Macros and Attributes

**Docs:** https://doc.rust-lang.org/stable/reference/macros.html | https://doc.rust-lang.org/stable/reference/attributes.html

## Declarative Macros (macro_rules!)

```rust
// Basic macro
macro_rules! say_hello {
    () => {
        println!("Hello!");
    };
}

say_hello!();  // expands to println!("Hello!")

// With arguments
macro_rules! vec_of_strings {
    ($($x:expr),*) => {
        vec![$($x.to_string()),*]
    };
}

let v = vec_of_strings!("a", "b", "c");

// Pattern matching
macro_rules! min {
    ($a:expr) => { $a };
    ($a:expr, $b:expr) => {
        if $a < $b { $a } else { $b }
    };
    ($a:expr, $($rest:expr),+) => {
        min!($a, min!($($rest),+))
    };
}

let m = min!(3, 1, 4, 1, 5);  // 1
```

### Macro Token Types

| Token | Matches | Example |
|-------|---------|---------|
| `ident` | Identifier | `foo` |
| `expr` | Expression | `2 + 3` |
| `ty` | Type | `i32`, `Vec<T>` |
| `pat` | Pattern | `Some(x)` |
| `stmt` | Statement | `let x = 5;` |
| `item` | Item | `fn foo() {}` |
| `block` | Block | `{ ... }` |
| `path` | Path | `std::collections::HashMap` |
| `literal` | Literal | `42`, `"hello"` |
| `meta` | Meta item | `#[inline]` content |
| `tt` | Token tree | any valid token tree |
| `vis` | Visibility | `pub`, `pub(crate)` |
| `lifetime` | Lifetime | `'a` |

### Repetition

```rust
// $(...),* — zero or more, comma-separated
// $(...),+ — one or more, comma-separated
// $(...)? — zero or one (optional)

macro_rules! hashmap {
    ($($key:expr => $value:expr),*) => {
        {
            let mut map = std::collections::HashMap::new();
            $(
                map.insert($key, $value);
            )*
            map
        }
    };
}

let m = hashmap!("a" => 1, "b" => 2, "c" => 3);
```

## Procedural Macros

### Derive Macros

```rust
// In a separate crate (proc-macro = true)
use proc_macro::TokenStream;

#[proc_macro_derive(HelloMacro)]
pub fn hello_macro_derive(input: TokenStream) -> TokenStream {
    // Generate code that implements a trait
    // ...
}

// Usage in another crate:
#[derive(HelloMacro)]
struct Pancakes;
// Generates impl HelloMacro for Pancakes { ... }
```

### Attribute Macros

```rust
#[proc_macro_attribute]
pub fn log_call(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Modify or wrap the annotated item
    // ...
}

#[log_call]
fn my_function() { }
```

### Function-like Macros

```rust
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    // Parse SQL-like syntax and generate Rust code
    // ...
}

let results = sql!("SELECT * FROM users WHERE age > 18");
```

## Common Derive Macros

```rust
// Built-in derives
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct MyStruct { /* ... */ }

// From external crates
#[derive(Serialize, Deserialize)]  // serde
#[derive(thiserror::Error)]         // thiserror
#[derive(Getters, Setters)]         // custom
```

## Attributes

### Built-in Attributes

```rust
// Function attributes
#[inline]          // suggest inlining
#[inline(always)]  // force inlining
#[inline(never)]   // prevent inlining
#[cold]            // function is unlikely to be called
#[track_caller]    // capture caller location for panic messages

// Test attributes
#[test]            // mark as test function
#[ignore]          // ignore test unless --ignored
#[should_panic]    // test should panic
#[should_panic(expected = "exact message")]

// Conditional compilation
#[cfg(target_os = "linux")]
#[cfg(debug_assertions)]
#[cfg(feature = "my_feature")]
#[cfg(all(unix, target_arch = "x86_64"))]
#[cfg(any(windows, target_os = "macos"))]
#[cfg(not(test))]

// cfg_attr — conditional attribute
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]

// Documentation
#[doc = "Inline doc"]
#[doc(hidden)]         // hide from docs
#[doc(alias = "name")] // add search alias

// Linting
#[allow(dead_code)]
#[allow(unused_variables)]
#[deny(warnings)]
#[warn(unused_imports)]
#[must_use]            // warn if return value ignored

// Foreign function interface
#[repr(C)]             // C-compatible layout
#[repr(u8)]            // specify enum/struct representation
#[repr(transparent)]   // same layout as single field
#[repr(packed)]        // no padding
#[repr(align(16))]     // force alignment

// Misc
#[non_exhaustive]      // can add fields/variants in future
#[deprecated = "use new_fn instead"]
#[derive(Debug)]       // derive trait implementations
```

### Crate-level Attributes

```rust
// At top of lib.rs or main.rs
#![allow(unused)]
#![deny(warnings)]
#![feature(async_closure)]  // nightly feature gate
#![crate_type = "lib"]
#![crate_name = "my_lib"]
```

### cfg Macro

```rust
// cfg! macro — boolean at compile time
if cfg!(target_os = "linux") {
    println!("Running on Linux");
}

// cfg attribute on blocks/items
#[cfg(target_pointer_width = "64")]
const BITS: usize = 64;

#[cfg(not(target_pointer_width = "64"))]
const BITS: usize = 32;
```

## Standard Library Macros

```rust
// Printing
println!("Hello {}", "world");
eprintln!("Error: {}", msg);
print!("No newline");
eprint!("Error no newline");

// Formatting
let s = format!("{} + {} = {}", 1, 2, 3);
let s = format!("{:>10}", "right");   // right-aligned width 10
let s = format!("{:0>5}", 42);        // "00042"
let s = format!("{:.2}", 3.14159);    // "3.14"
let s = format!("{:b}", 10);          // "1010" binary
let s = format!("{:#x}", 255);        // "0xff"

// Debugging
dbg!(x);              // prints [src/main.rs:5] x = 42
dbg!(x, y);           // multiple values

// Panicking
panic!("something went wrong");
unreachable!("should never reach here");
todo!("not implemented yet");
unimplemented!("not implemented");

// Assertions
assert!(condition);
assert_eq!(a, b);
assert_ne!(a, b);
debug_assert!(condition);     // only in debug builds
debug_assert_eq!(a, b);

// Environment
let path = env!("CARGO_MANIFEST_DIR");
let version = env!("CARGO_PKG_VERSION");
let opt = option_env!("MY_VAR");  // Option<&str>

// Compile-time
compile_error!("this feature requires nightly");
concat!("a", "b");        // "ab"
stringify!(x + y);        // "x + y"
file!();                  // current file
line!();                  // current line
column!();                // current column
module_path!();           // module path string

// Include
include_str!("file.txt");     // file contents as &str
include_bytes!("file.bin");   // file contents as &[u8]
include!("file.rs");          // include and parse as Rust code

// vec! and vec-like
let v = vec![1, 2, 3];
let m = hashmap!["a" => 1];  // if macro defined

// matches!
let result = matches!(opt, Some(1));  // bool
let result = matches!(val, 1..=5 | 10);

// write! and writeln!
use std::io::Write;
let mut buf = String::new();
write!(buf, "{}", "hello").unwrap();
writeln!(buf, "{}", "world").unwrap();
```
