# Structs and Enums

**Docs:** https://doc.rust-lang.org/stable/reference/items/structs.html | https://doc.rust-lang.org/stable/reference/items/enumerations.html

## Structs

### Named Field Structs

```rust
struct User {
    name: String,
    age: u32,
    active: bool,
}

let user = User {
    name: String::from("Alice"),
    age: 30,
    active: true,
};

// Access fields
println!("{}", user.name);

// Mutable struct — can modify fields
let mut user = user;
user.age = 31;

// Field init shorthand
fn build_user(name: String, age: u32) -> User {
    User { name, age, active: true }  // name and age shorthand
}

// Struct update syntax — spread remaining fields from another instance
let user2 = User {
    name: String::from("Bob"),
    ..user  // rest of fields from user
};
```

### Tuple Structs

```rust
struct Color(i32, i32, i32);
struct Point(i32, i32, i32);

let red = Color(255, 0, 0);
let origin = Point(0, 0, 0);

// Access by index
println!("R: {}", red.0);

// Color and Point are different types even with same fields
// fn process(c: Color) { }
// process(origin);  // ERROR: type mismatch
```

### Unit Structs

```rust
struct AlwaysEqual;

// No fields — useful for implementing traits
impl AlwaysEqual {
    fn new() -> Self { AlwaysEqual }
}
```

### Derive Common Traits

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

let p = Point { x: 1, y: 2 };
println!("{:?}", p);   // Debug: Point { x: 1, y: 2 }
println!("{:#?}", p);  // Pretty Debug
let p2 = p;            // Copy — p still valid
assert_eq!(p, p2);     // PartialEq
```

## Enums

### Basic Enums

```rust
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

let dir = Direction::Up;

match dir {
    Direction::Up => println!("Going up"),
    Direction::Down => println!("Going down"),
    Direction::Left | Direction::Right => println!("Horizontal"),
}
```

### Enums with Data

```rust
enum Message {
    Quit,                          // no data
    Move { x: i32, y: i32 },      // named fields (struct-like)
    Write(String),                 // single value (tuple-like)
    ChangeColor(i32, i32, i32),    // multiple values
}

let msg = Message::Move { x: 10, y: 20 };

match msg {
    Message::Quit => println!("Quit"),
    Message::Move { x, y } => println!("Move to ({}, {})", x, y),
    Message::Write(text) => println!("Write: {}", text),
    Message::ChangeColor(r, g, b) => println!("RGB({}, {}, {})", r, g, b),
}
```

### Option Enum

```rust
enum Option<T> {
    Some(T),
    None,
}

// No null in Rust — use Option<T>
let some: Option<i32> = Some(42);
let none: Option<i32> = None;

// Pattern matching
match some {
    Some(n) => println!("Got: {}", n),
    None => println!("Nothing"),
}

// Common methods
let val = some.unwrap();          // panics if None
let val = some.unwrap_or(0);      // default if None
let val = some.unwrap_or_else(|| compute_default());
let val = some.expect("should be Some");
let mapped = some.map(|n| n * 2); // Option<i32>
let filtered = some.filter(|&n| n > 10);
let and_then = some.and_then(|n| Some(n + 1));
let or = none.or(Some(5));        // Some(5)
let is_some = some.is_some();
let is_none = none.is_none();
```

### Repr for Enums

```rust
// Default repr — Rust layout (may reorder variants)
enum Foo { A, B, C }

// C repr — like a C enum, values assigned sequentially
#[repr(C)]
enum CFoo { A, B, C }

// Primitive repr — specify integer size
#[repr(u8)]
enum Small { A, B, C }  // each variant is a u8

// Explicit discriminant values
#[repr(u8)]
enum Status {
    Ok = 0,
    Error = 1,
    Pending = 2,
}
```

### Non-exhaustive Enums

```rust
// In another crate:
#[non_exhaustive]
pub enum Error {
    NotFound,
    PermissionDenied,
    // could add more variants in future
}

// When matching from another crate, must have _ arm
match error {
    Error::NotFound => {},
    Error::PermissionDenied => {},
    _ => {},  // required for non-exhaustive
}
```

## Pattern Matching

### Patterns

```rust
// Literals
match x { 1 => {}, 2 | 3 => {}, _ => {} }

// Ranges
match x { 1..=5 => {}, _ => {} }

// Bindings
match opt { Some(x) => println!("{}", x), None => {} }

// Struct destructuring
match point { Point { x, y } => println!("({}, {})", x, y) }
match point { Point { x, .. } => println!("{}", x) }  // ignore fields

// Tuple destructuring
match (a, b) { (0, 0) => {}, (x, 0) => {}, (0, y) => {}, (x, y) => {} }

// Enum variant destructuring
match msg {
    Message::Move { x, y } if x > 0 => {},  // with guard
    Message::Write(ref s) if s.is_empty() => {},
    _ => {},
}

// @ binding
match age {
    n @ 0..=17 => println!("minor: {}", n),
    n @ 18..=64 => println!("adult: {}", n),
    n => println!("senior: {}", n),
}
```

### Ref and Ref Mut in Patterns

```rust
// ref creates a reference in the pattern
let s = String::from("hello");
match s {
    ref r => println!("{}", r),  // r: &String, s not moved
}

// ref mut creates a mutable reference
let mut s = String::from("hello");
match s {
    ref mut r => { r.push_str(" world"); }
}
```

## Unions

```rust
// Unsafe to read — only one field is valid at a time
#[repr(C)]
union IntOrFloat {
    i: i32,
    f: f32,
}

let mut u = IntOrFloat { i: 42 };
unsafe {
    println!("int: {}", u.i);  // unsafe — may be UB if wrong field
}
u.f = 3.14;
unsafe {
    println!("float: {}", u.f);
}
// Unions are mainly for FFI with C
```

## Methods with impl Blocks

```rust
struct Circle {
    radius: f64,
}

impl Circle {
    // Associated function (constructor)
    fn new(radius: f64) -> Self {
        Circle { radius }
    }

    // &self method
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    // &mut self method
    fn grow(&mut self, factor: f64) {
        self.radius *= factor;
    }

    // self method (consuming)
    fn into_diameter(self) -> f64 {
        self.radius * 2.0
    }
}

// Multiple impl blocks are allowed
impl Circle {
    fn circumference(&self) -> f64 {
        2.0 * std::f64::consts::PI * self.radius
    }
}
```
