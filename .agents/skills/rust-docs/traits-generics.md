# Traits and Generics

**Docs:** https://doc.rust-lang.org/stable/reference/items/traits.html | https://doc.rust-lang.org/book/ch10-00-generics.html

## Trait Definitions

```rust
trait Summary {
    fn summarize(&self) -> String;

    // Default method implementation
    fn preview(&self) -> String {
        format!("{}...", &self.summarize()[..50.min(self.summarize().len())])
    }
}

// Default method can be overridden
trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> String;
    fn info(&self) -> String {
        format!("{} says {}", self.name(), self.sound())
    }
}
```

## Implementing Traits

```rust
struct Article {
    title: String,
    content: String,
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{}: {}", self.title, self.content)
    }
}

struct Tweet {
    username: String,
    text: String,
}

impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.username, self.text)
    }
    // preview() uses default implementation
}
```

## Trait Bounds (Generics)

```rust
// Function with trait bound
fn print_summary<T: Summary>(item: &T) {
    println!("{}", item.summarize());
}

// Multiple trait bounds
fn display_summary<T: Summary + Display>(item: &T) {
    println!("{}: {}", item, item.summarize());
}

// where clause (cleaner for complex bounds)
fn foo<T, U>(t: T, u: U) -> String
where
    T: Summary + Clone,
    U: Debug + PartialOrd,
{
    t.summarize()
}

// impl Trait syntax (syntactic sugar for trait bounds)
fn print(item: &impl Summary) {
    println!("{}", item.summarize());
}
// Equivalent to: fn print<T: Summary>(item: &T)
```

## Generic Types and Methods

```rust
struct Pair<T> {
    x: T,
    y: T,
}

impl<T> Pair<T> {
    fn new(x: T, y: T) -> Self {
        Pair { x, y }
    }
}

// impl block with additional trait bounds
impl<T: PartialOrd + Display> Pair<T> {
    fn cmp_display(&self) {
        if self.x >= self.y {
            println!("x >= y: {} >= {}", self.x, self.y);
        } else {
            println!("x < y: {} < {}", self.x, self.y);
        }
    }
}

// Implement a trait for a generic type
impl<T: Display> Summary for Pair<T> {
    fn summarize(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }
}
```

## Trait Objects (Dynamic Dispatch)

```rust
// dyn Trait — dynamic dispatch via vtable
fn print_summary(item: &dyn Summary) {
    println!("{}", item.summarize());
}

// Box<dyn Trait> — owned trait object
let items: Vec<Box<dyn Summary>> = vec![
    Box::new(Article { title: "T".into(), content: "C".into() }),
    Box::new(Tweet { username: "U".into(), text: "T".into() }),
];

for item in &items {
    println!("{}", item.summarize());
}

// Trait objects are DSTs — must be behind a pointer
// &dyn Trait, Box<dyn Trait>, Rc<dyn Trait>, Arc<dyn Trait>
```

## impl Trait

```rust
// Return position impl Trait (RPIT)
fn create_iterator() -> impl Iterator<Item = i32> {
    (0..10).filter(|x| x % 2 == 0)
}

// In argument position — anonymous generic
fn process(iter: impl Iterator<Item = i32>) {
    for x in iter {
        println!("{}", x);
    }
}

// impl Trait in return can only return one concrete type
// This WON'T work:
// fn foo(flag: bool) -> impl Trait {
//     if flag { TypeA } else { TypeB }  // ERROR: different types
// }
// Use Box<dyn Trait> for this case
```

## Common Standard Library Traits

```rust
// Display — for {} formatting
impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// Debug — for {:?} formatting (usually derived)
#[derive(Debug)]
struct Point { x: i32, y: i32 }

// Clone — explicit deep copy
#[derive(Clone)]
struct Data { values: Vec<i32> }

// Copy — bitwise copy (implicit, requires Clone)
#[derive(Clone, Copy)]
struct Coord { x: i32, y: i32 }

// PartialEq, Eq — equality comparison
#[derive(PartialEq, Eq)]
struct Id(u32);

// PartialOrd, Ord — ordering comparison
#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct Priority(i32);

// Hash — for use in HashMap/HashSet
#[derive(Hash, PartialEq, Eq)]
struct Key { id: u32 }

// Default — default value
#[derive(Default)]
struct Config {
    timeout: u32,  // defaults to 0
    retries: u32,
}
// Config::default() gives Config { timeout: 0, retries: 0 }

// From / Into — conversions
impl From<i32> for MyType {
    fn from(value: i32) -> Self {
        MyType(value)
    }
}
// Into is automatically implemented when From is implemented
let val: MyType = 42i32.into();

// TryFrom / TryInto — fallible conversions
impl TryFrom<&str> for Age {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse().map(Age).map_err(|e| e.to_string())
    }
}
```

## Associated Types

```rust
trait Container {
    type Item;  // associated type

    fn first(&self) -> Option<&Self::Item>;
    fn add(&mut self, item: Self::Item);
}

struct VecContainer<T>(Vec<T>);

impl<T> Container for VecContainer<T> {
    type Item = T;

    fn first(&self) -> Option<&T> {
        self.0.first()
    }

    fn add(&mut self, item: T) {
        self.0.push(item);
    }
}

// Using associated types in bounds
fn process<C: Container>(c: &C) where C::Item: Display {
    if let Some(item) = c.first() {
        println!("{}", item);
    }
}
```

## Default Generics (Type Parameters with Defaults)

```rust
trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}

// Default type parameter allows omitting the type
struct Counter(i32);
impl Add for Counter {  // Rhs defaults to Self (Counter)
    type Output = Counter;
    fn add(self, rhs: Counter) -> Counter {
        Counter(self.0 + rhs.0)
    }
}
```

## Supertraits

```rust
// Circle: Shape requires Shape to be implemented first
trait Shape: Display {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
}

// Equivalent to:
trait Shape where Self: Display {
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
}
```

## Object Safety

A trait is object-safe (can be used as `dyn Trait`) if:

1. It does not require `Self: Sized`
2. All methods use `&self` or `&mut self` (no `self` by value)
3. All methods don't use `Self` in arguments or return type
4. No generic methods

```rust
// Object-safe
trait Draw {
    fn draw(&self);
}

// NOT object-safe
trait Clone2 {
    fn clone(&self) -> Self;  // returns Self — not object-safe
}
```

## Newtype Pattern

```rust
// Wrap a type to implement foreign traits on foreign types
struct Meters(f64);
struct Feet(f64);

impl std::fmt::Display for Meters {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} m", self.0)
    }
}
```

## Operator Overloading (std::ops)

All operators in Rust are backed by traits in `std::ops`. Implementing these traits allows custom behavior for operators.

| Operator | Trait | Assignment Trait |
|----------|-------|-----------------|
| `+` | `Add` | `AddAssign` |
| `-` | `Sub` | `SubAssign` |
| `*` | `Mul` | `MulAssign` |
| `/` | `Div` | `DivAssign` |
| `%` | `Rem` | `RemAssign` |
| `&` | `BitAnd` | `BitAndAssign` |
| `\|` | `BitOr` | `BitOrAssign` |
| `^` | `BitXor` | `BitXorAssign` |
| `<<` | `Shl` | `ShlAssign` |
| `>>` | `Shr` | `ShrAssign` |
| `-` (unary) | `Neg` | — |
| `!` (unary) | `Not` | — |
| `*v` | `Deref` | `DerefMut` |
| `v[i]` | `Index` | `IndexMut` |

```rust
use std::ops::{Add, Sub};

#[derive(Debug, Copy, Clone, PartialEq)]
struct Point { x: i32, y: i32 }

impl Add for Point {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Sub for Point {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y }
    }
}

assert_eq!(Point { x: 3, y: 3 }, Point { x: 1, y: 0 } + Point { x: 2, y: 3 });
assert_eq!(Point { x: -1, y: -3 }, Point { x: 1, y: 0 } - Point { x: 2, y: 3 });
```

### Fn, FnMut, FnOnce traits
These traits are implemented by types that can be invoked like functions:
- `Fn` takes `&self` — call-by-reference (can be called multiple times)
- `FnMut` takes `&mut self` — call-by-mutable-reference (can mutate captured variables)
- `FnOnce` takes `self` — call-by-value (consumes captured variables, called once)

```rust
// Fn bound — takes closure by reference
fn call_with_one<F>(func: F) -> usize where F: Fn(usize) -> usize {
    func(1)
}

// FnMut bound — closure can mutate captured variables
fn do_twice<F>(mut func: F) where F: FnMut() {
    func(); func();
}

// FnOnce bound — closure consumes captured variables
fn consume_with_relish<F>(func: F) where F: FnOnce() -> String {
    println!("Consumed: {}", func());
}
```

### Other std::ops traits
- `Drop` — called when a value goes out of scope (destructor)
- `Try` — backs the `?` operator (for `Result` and `Option`)
  - `FromResidual` — converts residual values (enables `?` on custom types)
  - `Residual` — the "error part" of a `Try` type
- `RangeBounds` — implemented by range types (`..`, `a..`, `..b`, `a..b`, `a..=b`)
- `OneSidedRange` — marker for one-sided ranges (`a..`, `..b`, `..=b`)
- `IntoBounds` — converts values into range bounds
- `Coroutine` — for coroutine state machines (generators)
- `Yeet` — `do yeet <value>` operator for early `Try` break (nightly)
  - `ops::Yeet<T>` wraps a value for `try` block early exit
  - `do yeet Err("fail")` is equivalent to `return Err("fail")` inside `try` blocks
- `Deref` / `DerefMut` — deref coercion (`*v`, `&T → &U`)
- `DerefPure` — marker: Deref/DerefMut don't have side effects (unsafe trait)
- `CoerceUnsized` — compiler-implemented coercion (e.g., `&[T; N]` → `&[T]`)
- `DispatchFromDyn` — enables dynamic dispatch through coercion
- `Receiver` — allows `self` methods on non-reference types (e.g., `Arc<Self>`)
- `AsyncFn` / `AsyncFnMut` / `AsyncFnOnce` — async closures (1.97+)
  - Like `Fn`/`FnMut`/`FnOnce` but for `async fn` closures

### ops::Bound — Range Bound Enumeration

```rust
use std::ops::Bound;

// Bound represents endpoints of ranges:
enum Bound<T> {
    Included(T),  // a..=b → Included(b)
    Excluded(T),  // a..b  → Excluded(b)
    Unbounded,    // a..   → Unbounded
}

// Used by RangeBounds::bound() to query range endpoints:
use std::ops::RangeBounds;
let r = 1..5;
assert_eq!(r.start_bound(), Bound::Included(&1));
assert_eq!(r.end_bound(), Bound::Excluded(&5));

let r = ..=10;
assert_eq!(r.start_bound(), Bound::Unbounded);
assert_eq!(r.end_bound(), Bound::Included(&10));
```

### ops::ControlFlow — Iterator/Coroutine Control

```rust
use std::ops::ControlFlow;

// ControlFlow tells an operation whether to continue or break:
enum ControlFlow<B, C = ()> {
    Continue(C),
    Break(B),
}

// Used by iterator methods like try_fold, try_for_each:
let result = (1..=10).try_fold(0, |acc, x| {
    if acc + x > 15 {
        ControlFlow::Break(acc)
    } else {
        ControlFlow::Continue(acc + x)
    }
});
// result = ControlFlow::Break(15) — stopped early

// Also used by Coroutine::resume() to indicate completion
```

### ops::CoroutineState — Coroutine States

```rust
use std::ops::CoroutineState;

// CoroutineState represents the state of a coroutine:
enum CoroutineState<Y, R> {
    Yielded(Y),  // coroutine yielded a value, not done
    Complete(R), // coroutine finished with return value
}

// Used with the Coroutine trait (nightly):
// trait Coroutine<R> {
//     type Yield;
//     type Return;
//     fn resume(self: Pin<&mut Self>, arg: R) -> CoroutineState<Self::Yield, Self::Return>;
// }
```

## Conversion Traits (std::convert)

| Trait | Direction | Description |
|-------|-----------|-------------|
| `From<T>` | T → Self | infallible conversion |
| `Into<T>` | Self → T | reciprocal of `From` (auto-implemented) |
| `TryFrom<T>` | T → Self | fallible conversion (returns `Result`) |
| `TryInto<T>` | Self → T | reciprocal of `TryFrom` (auto-implemented) |
| `AsRef<T>` | &Self → &T | cheap reference-to-reference conversion |
| `AsMut<T>` | &mut Self → &mut T | mutable reference-to-reference conversion |

Key rules:
- `From<U> for T` implies `Into<T> for U` (auto-implemented by compiler)
- `TryFrom<U> for T` implies `TryInto<T> for U` (auto-implemented)
- `From` and `Into` are reflexive — all types can convert to/from themselves
- `AsRef`/`AsMut` auto-dereference if the inner type is a reference
- `Infallible` is the error type for infallible conversions (never occurs, like `!`)
- `FloatToInt` — trait for unchecked float-to-int conversion (`to_int_unchecked`)
- `convert::identity(x)` — returns x unchanged (identity function)

```rust
// From implies Into
let s: String = "hello".to_string();
let s2: String = String::from("hello");  // equivalent

// TryFrom / TryInto for fallible conversions
let n: i32 = 42;
let small: Result<u8, _> = n.try_into();  // Err if > 255

// AsRef for cheap reference conversions
fn print_len<T: AsRef<str>>(s: T) {
    println!("{}", s.as_ref().len());
}
print_len("hello");       // &str
print_len(String::from("hello"));  // String

// #[derive(From)] — auto-generate From impls (unstable, std::from):
// #[derive(From)]
// struct Meters(i32);  // generates: impl From<i32> for Meters
// Also works for structs with multiple fields (generates From<(T1, T2)>)
```

## Formatting Traits (std::fmt)

The `format!` / `println!` macros use formatting traits:

| Format | Trait | Description |
|--------|-------|-------------|
| `{}` | `Display` | User-facing output |
| `{:?}` | `Debug` | Debug output (derive with `#[derive(Debug)]`) |
| `{:x?}` | `Debug` | Debug with lower-case hex integers |
| `{:X?}` | `Debug` | Debug with upper-case hex integers |
| `{:o}` | `Octal` | Octal representation |
| `{:x}` | `LowerHex` | Lower-case hexadecimal |
| `{:X}` | `UpperHex` | Upper-case hexadecimal |
| `{:p}` | `Pointer` | Pointer address |
| `{:b}` | `Binary` | Binary representation |
| `{:e}` | `LowerExp` | Lower-case exponential notation |
| `{:E}` | `UpperExp` | Upper-case exponential notation |

### Formatting parameters
```rust
// Width — minimum field width
println!("{:5}!", "x");    // "x    !"
println!("{:1$}!", "x", 5); // "x    !" (positional width)
println!("{:width$}!", "x", width = 5); // "x    !" (named width)

// Fill/Alignment
println!("{:<5}!", "x");   // "x    !" (left-aligned)
println!("{:-<5}!", "x");  // "x----!" (left, fill with -)
println!("{:^5}!", "x");   // "  x  !" (center-aligned)
println!("{:>5}!", "x");   // "    x!" (right-aligned)

// Sign / # / 0
println!("{:+}", 5);       // "+5" (always show sign)
println!("{:#x}", 27);     // "0x1b" (alternate hex form)
println!("{:05}", 5);      // "00005" (zero-padded, width 5)
println!("{:05}", -5);     // "-0005" (zero-padded with sign)
println!("{:#010x}", 27);  // "0x0000001b" (alternate + zero-padded)

// Precision
println!("{:.3}", 3.14159);  // "3.142"
println!("{:.precision$}", 3.14159, precision = 2); // "3.14"

// Named arguments
let people = "Rustaceans";
println!("Hello {people}!");  // "Hello Rustaceans!"
println!("{value}", value = 4); // "4"
```

### Implementing Display
```rust
use std::fmt;

struct Vector2D { x: isize, y: isize }

impl fmt::Display for Vector2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// Display vs Debug:
// - Display: user-facing, not all types implement it
// - Debug: for debugging, implement for all public types (use #[derive(Debug)])
```

### Manual Debug Implementation with Builders

```rust
use std::fmt;

// When #[derive(Debug)] isn't enough, use Debug builders:
struct Tree {
    value: i32,
    children: Vec<Tree>,
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // DebugStruct — for struct-like output
        let mut s = f.debug_struct("Tree");
        s.field("value", &self.value);
        if !self.children.is_empty() {
            s.field("children", &self.children);
        }
        s.finish()  // "Tree { value: 42, children: [...] }"
    }
}

// Other debug builders:
// f.debug_list().entries(items).finish()       — [1, 2, 3]
// f.debug_set().entries(items).finish()        — {1, 2, 3}
// f.debug_map().entries(pairs).finish()        — {"a": 1, "b": 2}
// f.debug_tuple("Point").field(&x).field(&y).finish()  — Point(1, 2)

// Pretty printing: if f.alternate() is true, use {:#?}
// debug_struct etc. automatically handle pretty-printing
```

### Write Trait (std::fmt::Write)

```rust
use std::fmt::Write;

// Write trait — for formatting into String buffers:
let mut output = String::new();
write!(output, "Hello, {}!", "world").unwrap();
writeln!(output, " Value: {}", 42).unwrap();
// output = "Hello, world! Value: 42\n"

// Write is also implemented by fmt::Formatter (used in Display/Debug impls)
// Other formatting traits: Binary (b), Octal (o), LowerHex (x), UpperHex (X),
// LowerExp (e), UpperExp (E), Pointer (p)

// format_args! — create Arguments without allocating:
let args = format_args!("{} + {} = {}", 1, 2, 3);
// Can be passed to write! or used in const contexts
```

## Comparison Traits (std::cmp)

| Trait | Description | Derive? |
|-------|-------------|---------|
| `PartialEq<Rhs>` | Equality comparison — `==` / `!=` | `#[derive(PartialEq)]` |
| `Eq` | Reflexive, symmetric, transitive equality (marker trait) | `#[derive(Eq)]` |
| `PartialOrd<Rhs>` | Ordering comparison — `<`, `>`, `<=`, `>=` | `#[derive(PartialOrd)]` |
| `Ord` | Total ordering — `cmp()` method | `#[derive(Ord)]` |

Key distinctions:
- `PartialEq` allows types that can't be compared (e.g., `f64` with `NaN != NaN`)
- `Eq` is a marker trait requiring `PartialEq<Self>` — guarantees equivalence relation
- `PartialOrd` requires `PartialEq` — returns `Option<Ordering>`
- `Ord` requires `Eq + PartialOrd<Self>` — returns `Ordering` (total order)

### Ordering enum
```rust
enum Ordering { Less, Equal, Greater }

// cmp method (from Ord):
assert_eq!(5.cmp(&10), Ordering::Less);
assert_eq!(10.cmp(&10), Ordering::Equal);
assert_eq!(15.cmp(&10), Ordering::Greater);

// partial_cmp method (from PartialOrd):
assert_eq!(5.0.partial_cmp(&10.0), Some(Ordering::Less));
assert_eq!(f64::NAN.partial_cmp(&10.0), None);  // NaN is not ordered
```

### Reverse wrapper
```rust
use std::cmp::Reverse;

// Reverse wraps a value and reverses its ordering
// Useful for sorting in descending order:
let mut v = vec![3, 1, 4, 1, 5, 9, 2, 6];
v.sort_by_key(|&x| Reverse(x));  // descending: [9, 6, 5, 4, 3, 2, 1, 1]
```

### Comparison helper functions
```rust
use std::cmp::{min, max, min_by, max_by, min_by_key, max_by_key, minmax};

assert_eq!(min(1, 2), 1);
assert_eq!(max(1, 2), 2);
assert_eq!(min_by(1, 2, |a, b| a.cmp(b)), 1);
assert_eq!(max_by_key("hello", "world", |s| s.len()), "world");
assert_eq!(minmax(1, 2), (1, 2));
assert_eq!(cmp::minmax_by(1, 2, |a, b| a.cmp(b)), (1, 2));
assert_eq!(cmp::minmax_by_key("hi", "world", |s| s.len()), ("hi", "world"));
```

### Any trait and type introspection (std::any)
```rust
use std::any::{Any, TypeId, type_name, type_name_of_val};

// type_name — get the type name as &str (for debugging, not guaranteed stable):
let name = type_name::<Vec<i32>>();  // "alloc::vec::Vec<i32>"

// type_name_of_val — get type name from a value:
let x = 42;
let name = type_name_of_val(&x);  // "i32"

// TypeId — unique identifier for a type:
let id = TypeId::of::<String>();

// Any — trait for dynamic type checking:
let value: Box<dyn Any> = Box::new(42i32);
if let Some(n) = value.downcast_ref::<i32>() {
    println!("Got i32: {}", n);
}
// value.is::<i32>(), value.downcast_mut::<i32>(), value.downcast::<i32>()
```

### Implementing comparison traits
```rust
#[derive(PartialEq, Eq)]
struct Version { major: u32, minor: u32, patch: u32 }

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major.cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}
```

## Clone and Copy Traits (std::clone, std::marker)

### Clone — explicit deep copy
```rust
// Clone is for types that cannot be "implicitly copied"
// Requires explicit .clone() call
#[derive(Clone)]
struct Morpheus { blue_pill: f32, red_pill: i64 }

let f = Morpheus { blue_pill: 0.0, red_pill: 0 };
let copy = f.clone();

// Manual implementation:
impl Clone for MyType {
    fn clone(&self) -> Self {
        MyType { data: self.data.clone() }
    }
    // fn clone_from(&mut self, source: &Self) — optional, default calls *self = source.clone()
}
```

### Copy — implicit bitwise copy (marker trait)
```rust
// Copy is a subtrait of Clone
// Types implementing Copy are copied instead of moved
// Requirements: all fields must be Copy, must not implement Drop
#[derive(Clone, Copy)]
struct Point { x: i32, y: i32 }

let p1 = Point { x: 1, y: 2 };
let p2 = p1;  // copied, not moved — p1 still valid

// Additional clone traits (std::clone):
// CloneToUninit — unsafe trait: clone into uninitialized memory (nightly)
// TrivialClone — marker: clone is trivially equivalent to Copy (nightly)
// UseCloned — trait for using cloned values in closures (nightly)
// cell::CloneFromCell — clone from a Cell<T> where T: Clone (nightly)
```

## Default Trait (std::default)

```rust
// Default provides a default value for a type
#[derive(Default)]
struct Config {
    timeout: u32,       // default: 0
    retries: u32,       // default: 0
    name: String,       // default: ""
    enabled: bool,      // default: false
}

let cfg = Config::default();
// Config { timeout: 0, retries: 0, name: "", enabled: false }

// Manual implementation:
impl Default for Config {
    fn default() -> Self {
        Config { timeout: 30, retries: 3, name: "default".into(), enabled: true }
    }
}

// Used by:
// - unwrap_or_default() on Option/Result
// - Vec::new() (empty vec is default)
// - struct update syntax with ..Default::default()
let cfg = Config { timeout: 60, ..Default::default() };
```

## Hash Trait (std::hash)

Used by `HashMap` and `HashSet` to compute hash values.

```rust
use std::hash::{Hash, Hasher, DefaultHasher};

// Easiest: derive Hash
#[derive(Hash)]
struct Person { id: u32, name: String, phone: u64 }

// Manual implementation — control which fields are hashed:
impl Hash for Person {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);    // only hash id and phone
        self.phone.hash(state);  // skip name — persons with same id+phone are equal in hash
    }
}

// Computing a hash manually:
fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

// Key types:
// - Hash: trait implemented by hashable types
// - Hasher: trait for hashing algorithms (write_u8, write_u32, finish, etc.)
// - BuildHasher: trait for creating Hasher instances
// - DefaultHasher: default hashing algorithm (SipHash)
// - RandomState: default BuildHasher used by HashMap/HashSet (randomized seed)
```

### Hash and Eq must agree
If two values are equal (`==`), they must produce the same hash. If you implement `Hash` manually, ensure consistency with `Eq`/`PartialEq`.

## Borrow Traits (std::borrow)

```rust
use std::borrow::{Borrow, BorrowMut, ToOwned};

// Borrow<T> — type can be borrowed as &T
// String implements Borrow<str>, Vec<T> implements Borrow<[T]>, Box<T> implements Borrow<T>
fn lookup(map: &HashMap<String, i32>, key: &str) -> Option<&i32> {
    // HashMap uses Borrow to accept &str for lookup in HashMap<String, _>
    map.get(key)
}

// ToOwned — converts borrowed to owned (generalizes Clone)
// str: ToOwned<Owned = String>
// [T]: ToOwned<Owned = Vec<T>>
// Path: ToOwned<Owned = PathBuf>

// Cow uses ToOwned:
use std::borrow::Cow;
let s: Cow<str> = Cow::Borrowed("hello");  // no allocation
let s: Cow<str> = Cow::Owned("hello".to_string());  // owned
```

## Any Trait (std::any) — Runtime Type Information

```rust
use std::any::{Any, TypeId, type_name, type_name_of_val};

// Any is automatically implemented for all 'static types
// TypeId identifies a type at runtime
let id = TypeId::of::<i32>();
assert_eq!(id, TypeId::of::<i32>());
assert_ne!(id, TypeId::of::<u32>());

// Downcasting with dyn Any
let value: Box<dyn Any> = Box::new(42_i32);

if value.is::<i32>() {
    let val = value.downcast_ref::<i32>().unwrap();
    assert_eq!(*val, 42);
}

// downcast_ref — get &T if type matches
// downcast_mut — get &mut T if type matches
// downcast (on Box<dyn Any>) — get Box<T> if type matches

// type_name — get type name as &str (for debugging only, not guaranteed unique)
assert_eq!(type_name::<i32>(), "i32");

// Note: &dyn Any can only test concrete types, not trait implementations
// Warning: calling .type_id() on Box<dyn Any> gives TypeId of Box<dyn Any>,
// not the inner type. Use (&*boxed).type_id() to get the inner type's ID.
```

### Additional Traits — convert, iter, pin

```rust
// ─────────────────────────────────────────────────────────
// convert::FloatToInt — unchecked float-to-int conversion (nightly)
// ─────────────────────────────────────────────────────────
// Trait for types that can be converted from floats to integers.
// Used by `hint::float_to_int_unchecked` and related intrinsics.
// pub trait FloatToInt<Int> { fn to_int_unchecked(self) -> Int; }
// # Safety: float must be representable in target int type.
#![feature(float_to_int)]
let f: f64 = 42.0;
let i: i32 = unsafe { f.to_int_unchecked() };  // 42

// ─────────────────────────────────────────────────────────
// iter::Step — stepping through range values (nightly)
// ─────────────────────────────────────────────────────────
// Trait for types that can be used as range step values.
// All integer types implement Step. Enables `Range::next()` etc.
// pub trait Step: PartialOrd + Clone {
//     fn steps_between(start: &Self, end: &Self) -> Option<usize>;
//     fn forward_checked(start: Self, count: usize) -> Option<Self>;
//     fn backward_checked(start: Self, count: usize) -> Option<Self>;
//     // ... expanded methods
// }
// Used internally by range iteration — rarely implemented manually.

// ─────────────────────────────────────────────────────────
// iter::TrustedLen — marker for exact-size iterators (unsafe trait)
// ─────────────────────────────────────────────────────────
// Marker trait indicating an iterator's exact length is known.
// Like ExactSizeIterator but stronger — the length is trusted.
// # Safety: size_hint().1 must be Some(n) and exact.
unsafe impl TrustedLen for MyIter { /* ... */ }

// ─────────────────────────────────────────────────────────
// iter::Product / iter::Sum — aggregation traits
// ─────────────────────────────────────────────────────────
// Product — multiply all items: iterator.product()
// Sum — add all items: iterator.sum()
// Implemented for all numeric types.
let sum: i32 = [1, 2, 3, 4].iter().sum();       // 10
let product: i32 = [1, 2, 3, 4].iter().product(); // 24

// ─────────────────────────────────────────────────────────
// iter::TrustedStep — marker for Step impls that are trusted (nightly)
// ─────────────────────────────────────────────────────────
// Marker trait indicating Step impl is correct.
// # Safety: Step methods must return correct values.

// ─────────────────────────────────────────────────────────
// pin::PinCoerceUnsized — pin-aware unsized coercion (nightly)
// ─────────────────────────────────────────────────────────
// Trait for types that support unsized coercion while pinned.
// Enables Pin<Box<T>> → Pin<Box<dyn Trait>> coercions.
// Implemented automatically for types where DerefPure holds.
```
