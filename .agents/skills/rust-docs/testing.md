# Testing

**Docs:** https://doc.rust-lang.org/book/ch11-00-testing.html | https://doc.rust-lang.org/std/macro.test.html

## Unit Tests

```rust
// In the same file as the code being tested
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }

    #[test]
    fn test_add_overflow() {
        // Use should_panic for expected panics
        // Integer overflow panics in debug mode
    }
}
```

## Test Attributes

```rust
#[test]
fn basic_test() {
    assert!(true);
}

#[test]
#[ignore]  // skipped unless --ignored is passed
fn slow_test() {
    // expensive test
}

#[test]
#[should_panic]
fn should_panic_test() {
    panic!("expected");
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn should_panic_with_message() {
    let v = vec![1];
    let _ = v[10];
}
```

## Assertions

```rust
#[test]
fn assertions() {
    // assert! — boolean
    assert!(true);
    assert!(5 > 3, "5 should be greater than 3");

    // assert_eq! — equality
    assert_eq!(2 + 2, 4);
    assert_eq!(2 + 2, 4, "math is broken");

    // assert_ne! — inequality
    assert_ne!(2 + 2, 5);

    // debug_assert! — only in debug builds
    debug_assert!(expensive_check());

    // Custom comparison with PartialEq
    assert_eq!(
        Point { x: 1, y: 2 },
        Point { x: 1, y: 2 },
    );
}
```

## Integration Tests

```
my_project/
├── src/
│   └── lib.rs
├── tests/
│   ├── integration_test.rs   // each file is a separate crate
│   └── another_test.rs
```

```rust
// tests/integration_test.rs
use my_project::add;  // import from library crate

#[test]
fn test_add_integration() {
    assert_eq!(add(1, 2), 3);
}
```

## Doc Tests

```rust
/// Adds two numbers.
///
/// # Examples
///
/// ```
/// use my_project::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```bash
# Run doc tests
cargo test --doc

# Doc tests are compiled and run as separate programs
# Each code block in /// comments is a test
```

## Running Tests

```bash
cargo test                    # Run all tests
cargo test test_name          # Run tests matching name
cargo test -- --ignored       # Run ignored tests
cargo test -- --test-threads=1  # Single-threaded
cargo test -- --nocapture     # Show println! output
cargo test -- --show-output   # Show captured output
cargo test --test integration_test  # Run specific integration test file
cargo test --doc              # Run only doc tests
cargo test --lib              # Run only library unit tests
cargo test --bin my_app       # Run tests in binary
cargo bench                   # Run benchmarks
```

## Test Organization

```rust
// Common pattern: test module at bottom of each file
pub fn function_under_test() -> i32 { 42 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(function_under_test(), 42);
    }

    // Nested test modules for organization
    mod edge_cases {
        use super::*;

        #[test]
        fn empty_input() {
            // ...
        }
    }
}
```

## Using Test Helpers

```rust
// tests/common/mod.rs — shared test utilities
pub fn setup() -> TempDir {
    tempfile::tempdir().unwrap()
}

// tests/integration_test.rs
mod common;

#[test]
fn test_with_setup() {
    let dir = common::setup();
    // ...
}
```

## Mocking and Property Testing

```rust
// mockall crate for mocking
use mockall::{automock, mock};

#[automock]
trait Database {
    fn get_user(&self, id: u32) -> Option<User>;
}

#[test]
fn test_with_mock() {
    let mut mock_db = MockDatabase::new();
    mock_db.expect_get_user()
        .with(eq(1))
        .returning(|_| Some(User { name: "Alice".into() }));

    let user = mock_db.get_user(1);
    assert_eq!(user.unwrap().name, "Alice");
}

// proptest or quickcheck for property-based testing
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_add_commutative(a in -1000i32..1000, b in -1000i32..1000) {
        prop_assert_eq!(add(a, b), add(b, a));
    }
}
```

## Async Testing

```rust
// Using tokio::test
#[tokio::test]
async fn test_async() {
    let result = async_function().await;
    assert_eq!(result, 42);
}

// Using async_std
#[async_std::test]
async fn test_async_std() {
    let result = async_function().await;
    assert_eq!(result, 42);
}
```

## Benchmarking

```rust
// Using criterion (recommended)
// Cargo.toml: [dev-dependencies] criterion = { version = "0.5", features = ["html_reports"] }

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_add(c: &mut Criterion) {
    c.bench_function("add", |b| {
        b.iter(|| add(black_box(2), black_box(3)))
    });
}

criterion_group!(benches, benchmark_add);
criterion_main!(benches);
```

## CI Testing

```bash
# Common CI test commands
cargo test --all              # Test all crates in workspace
cargo test --all-features     # Test with all features enabled
cargo test --no-default-features  # Test without default features
cargo clippy -- -D warnings   # Lint with warnings as errors
cargo fmt -- --check          # Check formatting
cargo tarpaulin               # Code coverage (external tool)
```
