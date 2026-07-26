# Ownership, Borrowing, and Lifetimes

**Docs:** https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html | https://doc.rust-lang.org/stable/reference/lifetime.html

## Ownership Rules

1. Each value has a single **owner** — the variable that holds it
2. When the owner goes out of scope, the value is **dropped** (memory freed)
3. Assigning or passing a value **moves** ownership (for non-Copy types)

```rust
{
    let s = String::from("hello");  // s owns the String
    // s is valid here
}   // s goes out of scope, String is dropped, memory freed
```

## Move Semantics

```rust
let s1 = String::from("hello");
let s2 = s1;  // ownership moved to s2

// s1 is no longer valid — using it is a compile error
// println!("{}", s1);  // ERROR: value borrowed here after move

// To keep both, clone explicitly:
let s3 = String::from("world");
let s4 = s3.clone();  // deep copy, both valid
```

### Copy Types

Types that implement the `Copy` trait are copied instead of moved. All primitive types implement `Copy`.

```rust
let x = 5;
let y = x;  // i32 is Copy, so x is still valid
println!("{} {}", x, y);  // OK

// Types that implement Drop cannot implement Copy
// Types where all fields are Copy can derive Copy
#[derive(Clone, Copy)]
struct Point { x: i32, y: i32 }
```

## Borrowing

### Immutable References (`&T`)

```rust
let s = String::from("hello");
let r1 = &s;  // borrow
let r2 = &s;  // multiple immutable borrows allowed
println!("{} {} {}", r1, r2, s);  // OK — any number of immutable borrows
```

### Mutable References (`&mut T`)

```rust
let mut s = String::from("hello");
let r1 = &mut s;       // mutable borrow
r1.push_str(" world");
println!("{}", r1);

// Only ONE mutable reference at a time:
// let r2 = &mut s;  // ERROR: cannot borrow as mutable more than once
```

### Borrowing Rules

- **One mutable reference OR any number of immutable references** — never both simultaneously
- References must always be valid (no dangling pointers)
- These rules are enforced at **compile time**

### NLL (Non-Lexical Lifetimes)

```rust
let mut s = String::from("hello");
let r1 = &s;
let r2 = &s;
println!("{} {}", r1, r2);
// r1 and r2 are no longer used after this point

let r3 = &mut s;  // OK — r1 and r2 are no longer in use
r3.push_str(" world");
println!("{}", r3);
```

## Lifetimes

Lifetimes ensure references are valid as long as needed. The compiler verifies that no reference outlives its data.

### Explicit Lifetimes

```rust
// 'a is a generic lifetime parameter
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
// The returned reference lives as long as the shortest input lifetime
```

### Lifetime Elision Rules

The compiler infers lifetimes when three rules are satisfied:

1. **Each input reference gets its own lifetime**
2. **If exactly one input lifetime, it's assigned to all outputs**
3. **If multiple inputs but one is `&self`/`&mut self`, that lifetime is assigned to all outputs**

```rust
// These are equivalent — elision handles common cases
fn first_word(s: &str) -> &str { /* ... */ }
fn first_word<'a>(s: &'a str) -> &'a str { /* ... */ }
```

### Static Lifetime

`'static` means the reference lives for the entire program duration. All string literals have `'static` lifetime.

```rust
let s: &'static str = "I live forever";  // stored in binary
```

### Lifetime Bounds

```rust
// T must live at least as long as 'a
fn foo<'a, T: 'a>(t: &'a T) { /* ... */ }

// T must be Clone and live at least as long as 'a
fn bar<'a, T: Clone + 'a>(t: &'a T) -> T { t.clone() }
```

### Lifetime in Structs

```rust
struct Excerpt<'a> {
    part: &'a str,
}

fn main() {
    let novel = String::from("Call me Ishmael. Some years ago...");
    let first_sentence = novel.split('.').next().unwrap();
    let excerpt = Excerpt { part: first_sentence };
    // excerpt cannot outlive novel
}
```

## Dangling References — Prevented at Compile Time

```rust
// This won't compile — dangling reference
fn dangle() -> &String {
    let s = String::from("hello");
    &s  // ERROR: s is dropped when function ends, reference would dangle
}

// Correct — return ownership instead
fn no_dangle() -> String {
    let s = String::from("hello");
    s
}
```

## Drop Trait

```rust
struct CustomDrop {
    data: String,
}

impl Drop for CustomDrop {
    fn drop(&mut self) {
        println!("Dropping: {}", self.data);
    }
}

fn main() {
    let _c = CustomDrop { data: String::from("bye") };
    // drop called automatically when _c goes out of scope

    // Manual drop with std::mem::drop
    let c2 = CustomDrop { data: String::from("manual") };
    drop(c2);  // dropped now, not at end of scope
    println!("After manual drop");
}
```

## Common Patterns

### Passing by Reference

```rust
fn calculate_length(s: &String) -> usize {
    s.len()
}   // s is borrowed, not dropped here

fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);  // borrow, s still valid
    println!("'{}' has length {}", s, len);
}
```

### Returning References

```rust
fn largest<'a>(list: &'a [i32]) -> &'a i32 {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}
```

### Interior Mutability Pattern

When you need to mutate through an immutable reference, use `RefCell` (runtime borrow checking):

```rust
use std::cell::RefCell;

let data = RefCell::new(5);
{
    let mut borrow = data.borrow_mut();
    *borrow += 1;
}
println!("{}", data.borrow());  // 6
```
