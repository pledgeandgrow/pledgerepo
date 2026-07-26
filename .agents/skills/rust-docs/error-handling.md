# Error Handling

**Docs:** https://doc.rust-lang.org/std/error/index.html | https://doc.rust-lang.org/book/ch09-00-error-handling.html

## Two Types of Errors

1. **Recoverable errors** — `Result<T, E>` (file not found, parse error)
2. **Unrecoverable errors** — `panic!` (invariant violated, impossible state)

## Result Type

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}

// Returning Result
fn parse_int(s: &str) -> Result<i32, std::num::ParseIntError> {
    s.parse::<i32>()
}

// Handling with match
match parse_int("42") {
    Ok(n) => println!("Parsed: {}", n),
    Err(e) => eprintln!("Error: {}", e),
}
```

## The ? Operator

```rust
// ? propagates errors — returns Err early if the expression is Err
fn read_and_parse(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;  // ? propagates io::Error
    let num: i32 = content.trim().parse()?;         // ? propagates ParseIntError
    Ok(num)
}

// ? on Option
fn first_char(s: &str) -> Option<char> {
    let c = s.chars().next()?;  // returns None if empty
    Some(c.to_uppercase().next()?)
}

// ? with From trait — converts error types automatically
fn process() -> Result<(), MyError> {
    let n: i32 = "abc".parse()?;  // ParseIntError converted to MyError via From
    Ok(())
}
```

## panic!

```rust
// Unrecoverable error — aborts the program
fn must_be_positive(x: i32) -> i32 {
    if x < 0 {
        panic!("x must be positive, got: {}", x);
    }
    x
}

// panic with format
panic!("index {} out of bounds {}", idx, len);

// In debug builds, panic prints a backtrace
// Set RUST_BACKTRACE=1 for full backtrace
```

## unwrap and expect

```rust
let x: Option<i32> = Some(42);
let val = x.unwrap();              // panics if None
let val = x.expect("should be Some");  // panics with custom message

let r: Result<i32, &str> = Ok(42);
let val = r.unwrap();              // panics if Err
let val = r.expect("should be Ok");

// Use unwrap/expect for prototyping or when you're certain
// Prefer ? or match for production code
```

## Custom Error Types

```rust
use std::fmt;

#[derive(Debug)]
enum AppError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
    NotFound(String),
    InvalidInput { field: String, value: String },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Parse(e) => write!(f, "Parse error: {}", e),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InvalidInput { field, value } => {
                write!(f, "Invalid {} = {}", field, value)
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

// Error trait full signature:
// trait Error: Debug + Display {
//     fn source(&self) -> Option<&(dyn Error + 'static)> { None }
//     fn type_id(&self) -> TypeId { ... }  // deprecated
//     fn backtrace(&self) -> Option<&Backtrace> { None }  // nightly
//     fn description(&self) -> &str { ... }  // deprecated
//     fn cause(&self) -> Option<&dyn Error> { ... }  // deprecated, use source
//     fn provide<'a>(&'a self, request: &mut Request<'a>) {}
// }

// provide() — error downcasting graph, allows requesting typed data
// request_ref::<T>() / request_value::<T>() — extract provided data
```

// From implementations for ? operator
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        AppError::Parse(e)
    }
}
```

## Using thiserror and anyhow

```rust
// thiserror — for library error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
    #[error("Not found: {0}")]
    NotFound(String),
}

// anyhow — for application error handling
use anyhow::{Context, Result};

fn process_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path))?;
    Ok(content)
}
```

## Common Result Methods

```rust
let r: Result<i32, &str> = Ok(42);

// Transforming Ok value
let mapped = r.map(|x| x * 2);          // Ok(84)
let and_then = r.and_then(|x| Ok(x + 1)); // Ok(43)

// Transforming Err value
let err_mapped = r.map_err(|e| e.to_uppercase());

// Combining Results
let combined = r.and(Ok("done"));       // Ok("done")
let alt = r.or(Ok(0));                  // Ok(42) — keeps original if Ok

// Unwrapping
let val = r.unwrap();
let val = r.unwrap_or(0);
let val = r.unwrap_or_default();
let val = r.unwrap_or_else(|e| {
    eprintln!("Error: {}", e);
    0
});

// Checking
let is_ok = r.is_ok();
let is_err = r.is_err();

// Converting
let opt = r.ok();     // Option<i32> — Some(42)
let err = r.err();    // Option<&str> — None
```

## Converting Between Option and Result

```rust
// Option -> Result
let opt: Option<i32> = Some(42);
let res: Result<i32, &str> = opt.ok_or("was None");
let res: Result<i32, String> = opt.ok_or_else(|| "was None".to_string());

// Result -> Option
let res: Result<i32, String> = Ok(42);
let opt: Option<i32> = res.ok();  // drops the error
```

## Result Method Overview (from std::result)

### Querying the variant
- `is_ok()` / `is_err()` — returns `true` if `Ok` or `Err` respectively
- `is_ok_and(f)` / `is_err_and(f)` — applies `f` to the contained value to produce a bool

### Adapters for working with references
- `as_ref()` — converts `&Result<T, E>` to `Result<&T, &E>`
- `as_mut()` — converts `&mut Result<T, E>` to `Result<&mut T, &mut E>`
- `as_deref()` — converts `&Result<T, E>` to `Result<&T::Target, &E>`
- `as_deref_mut()` — converts `&mut Result<T, E>` to `Result<&mut T::Target, &mut E>`

### Extracting contained values
- `expect(msg)` — panics with custom message if `Err` (requires `E: Debug`)
- `unwrap()` — panics with generic message if `Err` (requires `E: Debug`)
- `unwrap_or(default)` — returns provided default if `Err`
- `unwrap_or_default()` — returns `Default::default()` if `Err` (requires `T: Default`)
- `unwrap_or_else(f)` — returns result of evaluating `f` if `Err`
- `unwrap_unchecked()` — UB if `Err` (unsafe, no panic)
- `expect_err(msg)` — panics if `Ok` (requires `T: Debug`)
- `unwrap_err()` — panics if `Ok` (requires `T: Debug`)

### Transforming contained values
- `map(f)` — `Result<T, E>` → `Result<U, E>` by applying `f` to `Ok` value
- `map_err(f)` — `Result<T, E>` → `Result<T, F>` by applying `f` to `Err` value
- `inspect(f)` — applies `f` to `Ok` value by reference, returns the `Result`
- `inspect_err(f)` — applies `f` to `Err` value by reference, returns the `Result`
- `map_or(default, f)` — applies `f` to `Ok` value, or returns `default` if `Err`
- `map_or_else(fallback, f)` — applies `f` to `Ok`, or `fallback` to `Err` value

### Boolean operators (Ok = true, Err = false)
- `and(other)` — returns `other` if `Ok`, else `Err(e)`
- `or(other)` — returns `Ok(x)` if `Ok`, else `other`
- `and_then(f)` — lazily evaluates `f` on `Ok` value
- `or_else(f)` — lazily evaluates `f` on `Err` value

### Converting between Option and Result
- `ok()` — `Result<T, E>` → `Option<T>` (drops error)
- `err()` — `Result<T, E>` → `Option<E>` (drops ok value)
- `transpose()` — transposes `Result<Option<T>, E>` to `Option<Result<T, E>>`

### Iterating over Result
```rust
// into_iter acts like once(v) if Ok(v), empty() if Err
// iter yields &T, iter_mut yields &mut T

// Flatten to collect only Ok values, ignoring errors:
let nums: Vec<u8> = ["17", "not a number", "99", "-27", "768"]
    .into_iter()
    .map(u8::from_str)
    .flatten()
    .collect();
assert_eq!(nums, [17, 99]);

// Collect into Result — short-circuits on first Err:
let v: Result<Vec<_>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
assert_eq!(v, Ok(vec![1, 2, 3]));
```

### Comparison operators
- `Ok` compares as less than any `Err`
- Two `Ok` or two `Err` compare as their contained values

### `#[must_use]` on Result
Result is annotated with `#[must_use]` — the compiler warns if a Result value is ignored. This ensures errors are not silently dropped.

## Option Method Overview (from std::option)

### Querying the variant
- `is_some()` / `is_none()` — returns `true` if `Some` or `None`
- `is_some_and(f)` / `is_none_or(f)` — applies `f` to contained value to produce bool

### Adapters for working with references
- `as_ref()` — `&Option<T>` → `Option<&T>`
- `as_mut()` — `&mut Option<T>` → `Option<&mut T>`
- `as_deref()` — `&Option<T>` → `Option<&T::Target>`
- `as_deref_mut()` — `&mut Option<T>` → `Option<&mut T::Target>`
- `as_slice()` — returns one-element slice if `Some`, empty if `None`
- `as_pin_ref()` / `as_pin_mut()` — for Pin projections

### Extracting the contained value
- `expect(msg)` — panics with custom message if `None`
- `unwrap()` — panics with generic message if `None`
- `unwrap_or(default)` — returns provided default if `None`
- `unwrap_or_default()` — returns `Default::default()` if `None`
- `unwrap_or_else(f)` — returns result of evaluating `f` if `None`
- `unwrap_unchecked()` — UB if `None` (unsafe)

### Transforming contained values
- `map(f)` — `Option<T>` → `Option<U>` by applying `f` to `Some`
- `filter(pred)` — returns `Some(t)` if `pred(t)` is true, else `None`
- `flatten()` — removes one level of nesting from `Option<Option<T>>`
- `inspect(f)` — applies `f` to contained value by reference if `Some`
- `map_or(default, f)` — applies `f` to `Some`, or returns `default` if `None`
- `map_or_else(fallback, f)` — applies `f` to `Some`, or `fallback()` if `None`

### Boolean operators (Some = true, None = false)
- `and(other)` — returns `other` if `Some`, else `None`
- `or(other)` — returns `Some(x)` if `Some`, else `other`
- `xor(other)` — returns `Some` if exactly one is `Some`, else `None`
- `and_then(f)` — lazily evaluates `f` on `Some` value
- `or_else(f)` — lazily evaluates `f` if `None`

### Combining two Options
- `zip(other)` — returns `Some((s, o))` if both `Some`, else `None`
- `zip_with(other, f)` — returns `Some(f(s, o))` if both `Some`, else `None`

### Modifying in-place
- `insert(val)` — inserts value, dropping old contents, returns `&mut T`
- `get_or_insert(val)` — gets current value, inserts `val` if `None`
- `get_or_insert_default()` — gets current value, inserts `Default` if `None`
- `get_or_insert_with(f)` — gets current value, inserts `f()` if `None`
- `take()` — takes ownership, replaces with `None`
- `replace(val)` — takes ownership, replaces with `Some(val)`

### Iterating over Option
```rust
// into_iter acts like once(v) if Some(v), empty() if None
// iter yields &T, iter_mut yields &mut T

// Chain Option into an iterator pipeline:
let nums: Vec<i32> = (0..4).chain(Some(42)).chain(4..8).collect();
assert_eq!(nums, [0, 1, 2, 3, 42, 4, 5, 6, 7]);

let nums: Vec<i32> = (0..4).chain(None).chain(4..8).collect();
assert_eq!(nums, [0, 1, 2, 3, 4, 5, 6, 7]);
```

### Collecting into Option
```rust
// FromIterator for Option — returns None if any element is None
let v = [Some(2), Some(4), None, Some(8)];
let res: Option<Vec<_>> = v.into_iter().collect();
assert_eq!(res, None);

let v = [Some(2), Some(4), Some(8)];
let res: Option<Vec<_>> = v.into_iter().collect();
assert_eq!(res, Some(vec![2, 4, 8]));

// Product and Sum for Option
let v = [None, Some(1), Some(2), Some(3)];
let res: Option<i32> = v.into_iter().sum();
assert_eq!(res, None);

let v = [Some(1), Some(2), Some(21)];
let res: Option<i32> = v.into_iter().product();
assert_eq!(res, Some(42));
```

### Null pointer optimization (NPO)
Rust guarantees `Option<T>` has the same size as `T` for these types:
- `Box<U>` (where `U: Sized`)
- `&U` (where `U: Sized`)
- `&mut U` (where `U: Sized`)
- `fn`, `extern "C" fn`
- `num::NonZero*` types
- `ptr::NonNull<U>` (where `U: Sized`)
- `#[repr(transparent)]` structs wrapping one of the above

### Comparison operators for Option
- `None` compares as less than any `Some`
- Two `Some` compare as their contained values

## Error Handling Best Practices

1. **Use `?` for propagation** — don't match manually unless you need custom logic
2. **Use `thiserror` for libraries** — clean derive-based error types
3. **Use `anyhow` for applications** — simple error chaining with context
4. **Don't `unwrap()` in production** — use `?`, `expect()` with context, or proper error handling
5. **Implement `std::error::Error`** for custom error types
6. **Use `From` to convert errors** — enables `?` across error types
7. **Panic for impossible states** — use `assert!`, `unreachable!`, `panic!` for invariants
8. **Never ignore `Result`** — `#[must_use]` warns if you do; use `let _ =` explicitly if intentional

## Error Report (std::error::Report)

```rust
use std::error::Report;

// Report wraps an error for pretty-printing with backtraces
// Useful for top-level error handling in main():
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = do_something();
    if let Err(e) = result {
        eprintln!("{}", Report::new(e));  // pretty-prints error chain
    }
    Ok(())
}

// Report displays the full error chain:
// Error: IO error
// Caused by: No such file or directory (os error 2)

// In main, returning Result uses Report automatically:
fn main() -> Result<(), std::io::Error> {
    // If this errors, main prints it using Report format
    std::fs::read_to_string("missing.txt")?;
    Ok(())
}
// Output:
// Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

## expect() Message Styles

```rust
// "expect as error message" — describes what went wrong:
let path = std::env::var("IMPORTANT_PATH")
    .expect("env variable `IMPORTANT_PATH` is not set");

// "expect as precondition" — describes what SHOULD have happened:
let path = std::env::var("IMPORTANT_PATH")
    .expect("env variable `IMPORTANT_PATH` should be set by `wrapper_script.sh`");

// The precondition style adds new info beyond the source error,
// and reads better in panic output.
```

## Panic Handling (std::panic)

### catch_unwind — Catching Panics

```rust
use std::panic;

// catch_unwind catches a panic, returning Result
let result = panic::catch_unwind(|| {
    panic!("boom");
});
assert!(result.is_err());

let result = panic::catch_unwind(|| {
    42
});
assert_eq!(result, Ok(42));

// Caveats:
// - Only catches unwinding panics, not aborts (panic = "abort")
// - Not for general error handling — use Result for that
// - Requires UnwindSafe boundary (or AssertUnwindSafe wrapper)
// - Across FFI boundaries, panics are aborted (not unwound)
```

### UnwindSafe and RefUnwindSafe

```rust
use std::panic::{UnwindSafe, AssertUnwindSafe};

// UnwindSafe — type is safe to access after a panic unwind
// Most types are UnwindSafe, but &mut T and &RefCell<T> are NOT

// AssertUnwindSafe — opt-out of UnwindSafe checking
let mut data = vec![1, 2, 3];
let result = panic::catch_unwind(AssertUnwindSafe(|| {
    data.push(4);
    42
}));
// data may be in inconsistent state if panic occurred
```

### Panic Hooks

```rust
use std::panic;

// Set a custom panic hook (called before unwinding)
panic::set_hook(Box::new(|info| {
    // info: &PanicHookInfo
    let location = info.location();
    let payload = info.payload();
    eprintln!("Panic at {:?}: {:?}", location, payload);
}));

// Take (remove) the current hook
let old_hook = panic::take_hook();

// Chain hooks — call old hook then add behavior
panic::set_hook(Box::new(move |info| {
    old_hook(info);  // call previous hook
    eprintln!("Additional panic info");
}));

// always_abort — force abort on panic regardless of panic strategy
// panic::always_abort();
```

### resume_unwind — Re-throwing Panics

```rust
use std::panic;

// resume_unwind re-raises a caught panic (useful for FFI boundary)
let err = panic::catch_unwind(|| panic!("original")).unwrap_err();
panic::resume_unwind(err);  // re-throws with original payload
```

### Backtrace Configuration

```rust
use std::panic::BacktraceStyle;

// Configure backtrace style:
// - Short: concise backtrace
// - Full: complete backtrace
// - Off: no backtrace
// - Auto: environment-dependent (default)
panic::set_backtrace_style(BacktraceStyle::Full);
let style = panic::get_backtrace_style();
```

## Backtrace (std::backtrace)

```rust
use std::backtrace::{Backtrace, BacktraceStatus};

// Backtrace::capture — respects RUST_BACKTRACE / RUST_LIB_BACKTRACE env vars
let bt = Backtrace::capture();
match bt.status() {
    BacktraceStatus::Supported => println!("{}", bt),
    BacktraceStatus::Disabled => println!("backtrace disabled"),
    BacktraceStatus::Unsupported => println!("not supported on this platform"),
    _ => {}
}

// Backtrace::force_capture — ignores env vars, always captures
let bt = Backtrace::force_capture();

// Attach backtrace to errors:
struct MyError {
    msg: String,
    backtrace: Backtrace,
}

// Env vars:
// RUST_BACKTRACE=1 — enable backtraces
// RUST_LIB_BACKTRACE=1 — same, but takes precedence
// RUST_BACKTRACE=0 — disable (default if unset)
// Note: env var state is cached on first backtrace creation

// Backtrace::disabled() — create an empty/disabled backtrace (no allocation)
let bt = Backtrace::disabled();
```

## Error Reporting and Requests (std::error)

```rust
use std::error::{Error, Report, Request, request_ref, request_value};

// Report — formatted error report with backtrace and source chain:
// Wraps any Error type for display with full context
let result: Result<(), MyError> = do_something();
if let Err(e) = result {
    let report = Report::new(e);
    eprintln!("{:?}", report);  // pretty-printed with source chain
    eprintln!("{:}", report);   // single-line display
}

// Request — type-erased request for additional error context:
// Allows asking an error for typed supplementary data
// error::request_ref::<T>(&error) → Option<&T>
// error::request_value::<T>(&error) → Option<T>

// Example: requesting a backtrace from an error:
// if let Some(bt) = request_ref::<Backtrace>(&error) {
//     eprintln!("{}", bt);
// }

// Error trait methods:
// trait Error: Debug + Display {
//     fn source(&self) -> Option<&(dyn Error + 'static)>;  // source chain
//     fn provide<'a>(&'a self, req: &mut Request<'a>);     // typed data provision
//     fn backtrace(&self) -> Option<&Backtrace>;           // backtrace (nightly)
// }
// The provide() method replaces downcast_ref for non-'static data
// Types can provide multiple typed values via Request::provide_ref/value
```
