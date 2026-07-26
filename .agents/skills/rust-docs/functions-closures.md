# Functions and Closures

**Docs:** https://doc.rust-lang.org/stable/reference/items/functions.html | https://doc.rust-lang.org/book/ch03-03-how-functions-work.html

## Function Definitions

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b  // last expression is the return value (no semicolon)
}

// Explicit return
fn early_return(x: i32) -> i32 {
    if x < 0 {
        return -1;
    }
    x * 2
}

// No return value (unit type)
fn print_value(x: i32) {
    println!("Value: {}", x);
}

// Diverging function (never returns)
fn panic_if_negative(x: i32) -> ! {
    if x < 0 {
        panic!("negative: {}", x);
    }
    continue_processing(x);
}
```

## Parameters

```rust
// By value
fn take_ownership(s: String) { /* s is moved in */ }

// By reference
fn borrow(s: &String) { /* s is borrowed */ }

// By mutable reference
fn modify(s: &mut String) { s.push_str("!"); }

// Multiple return values via tuple
fn swap(a: i32, b: i32) -> (i32, i32) { (b, a) }

// Variadic functions are not supported in safe Rust
// Use macros (like println!) or slices for similar functionality
```

## Generic Functions

```rust
fn identity<T>(x: T) -> T {
    x
}

// With trait bounds
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

// Multiple bounds with where clause
fn foo<T, U>(t: T, u: U) -> i32
where
    T: Display + Clone,
    U: Debug + PartialOrd,
{
    42
}

// const generics
fn array_sum<const N: usize>(arr: [i32; N]) -> i32 {
    arr.iter().sum()
}
```

## Closures

Closures are anonymous functions that can capture their environment.

```rust
// Syntax: |params| { body } or |params| expression
let add = |a, i32| a + b;
let add = |a, b| a + b;             // type inferred
let greet = || println!("hello");    // no params
let square = |x: i32| x * x;

// Call closures like functions
let result = add(2, 3);  // 5
```

### Capture Modes

```rust
let s = String::from("hello");

// By reference (immutable borrow)
let print = || println!("{}", s);       // &String
print();

// By mutable reference
let mut s = String::from("hello");
let mut push = || s.push_str(" world"); // &mut String
push();

// By value (move)
let s = String::from("hello");
let consume = move || {
    println!("{}", s);
    drop(s);  // s moved into closure
};
consume();
// s is no longer available here

// move keyword forces capture by value
let data = vec![1, 2, 3];
let f = move || {
    println!("{:?}", data);  // data moved into closure
};
```

### Closure Types

```rust
// Fn — captures by reference (immutable)
fn apply_fn(f: impl Fn(i32) -> i32, x: i32) -> i32 { f(x) }

// FnMut — captures by mutable reference
fn apply_fnmut(mut f: impl FnMut(i32) -> i32, x: i32) -> i32 { f(x) }

// FnOnce — captures by value (consumed)
fn apply_fnonce(f: impl FnOnce(i32) -> i32, x: i32) -> i32 { f(x) }

// Fn implements FnMut, which implements FnOnce
// So any Fn can be used where FnMut or FnOnce is expected
```

### Returning Closures

```rust
fn make_adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y
}

let add5 = make_adder(5);
println!("{}", add5(3));  // 8
```

### Storing Closures

```rust
// Use Box for storing closures of different types
let closures: Vec<Box<dyn Fn(i32) -> i32>> = vec![
    Box::new(|x| x + 1),
    Box::new(|x| x * 2),
    Box::new(|x| x - 3),
];

for (i, f) in closures.iter().enumerate() {
    println!("{}: {}", i, f(10));
}
```

## Function Pointers

```rust
// fn type — function pointer (not a closure)
fn add(a: i32, b: i32) -> i32 { a + b }

let f: fn(i32, i32) -> i32 = add;
println!("{}", f(2, 3));  // 5

// Function pointers don't capture environment
// They implement Fn, FnMut, and FnOnce
fn apply(f: fn(i32, i32) -> i32, a: i32, b: i32) -> i32 {
    f(a, b)
}
println!("{}", apply(add, 2, 3));
```

## Methods and Associated Functions

```rust
struct Rectangle {
    width: f64,
    height: f64,
}

impl Rectangle {
    // Associated function (like a static method)
    fn new(width: f64, height: f64) -> Self {
        Rectangle { width, height }
    }

    // Method — takes self as first parameter
    fn area(&self) -> f64 {
        self.width * self.height
    }

    // Mutable method
    fn scale(&mut self, factor: f64) {
        self.width *= factor;
        self.height *= factor;
    }

    // Consuming method — takes ownership
    fn into_square(self) -> f64 {
        self.width.max(self.height)
    }
}

// Method call syntax
let mut rect = Rectangle::new(10.0, 20.0);
println!("{}", rect.area());  // 200
rect.scale(2.0);
println!("{}", rect.area());  // 800
```

## Iterators

### The three forms of iteration
- `iter()` — iterates over `&T` (immutable references)
- `iter_mut()` — iterates over `&mut T` (mutable references)
- `into_iter()` — iterates over `T` (consumes the collection)

### The Iterator trait
```rust
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```
All other iterator methods are default methods built on top of `next`. Iterators are **lazy** — nothing happens until you call `next` or consume the iterator.

### for loops and IntoIterator
Rust's `for` loop is sugar for `IntoIterator`:
```rust
// This:
for x in values { println!("{x}"); }

// Desugars to:
match IntoIterator::into_iter(values) {
    mut iter => loop {
        match iter.next() {
            Some(val) => { let x = val; println!("{x}"); }
            None => break,
        }
    }
}
```
All `Iterator`s implement `IntoIterator` by returning themselves.

### Implementing a custom iterator
```rust
struct Counter { count: usize }

impl Counter {
    fn new() -> Counter { Counter { count: 0 } }
}

impl Iterator for Counter {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        self.count += 1;
        if self.count < 6 { Some(self.count) } else { None }
    }
}

let mut counter = Counter::new();
assert_eq!(counter.next(), Some(1));
assert_eq!(counter.next(), Some(2));
// ...
assert_eq!(counter.next(), None);
```

### Iterator adaptors (lazy — produce new iterators)
```rust
let nums = vec![1, 2, 3, 4, 5];

// Creating iterators
let iter = nums.iter();          // yields &i32
let into_iter = nums.into_iter(); // yields i32 (consumes)
let mut_iter = nums.iter_mut();  // yields &mut i32

// Transforming
let doubled: Vec<i32> = nums.iter().map(|&x| x * 2).collect();
let evens: Vec<&i32> = nums.iter().filter(|&&x| x % 2 == 0).collect();

// Consuming (eager)
let sum: i32 = nums.iter().sum();
let max: Option<&i32> = nums.iter().max();
let first_even = nums.iter().find(|&&x| x % 2 == 0);
let any_positive = nums.iter().any(|&x| x > 0);
let all_positive = nums.iter().all(|&x| x > 0);

// Chaining
let result: Vec<i32> = (1..=10)
    .filter(|x| x % 2 == 0)
    .map(|x| x * x)
    .collect();
// [4, 16, 36, 64, 100]

// Enumerate
for (i, val) in nums.iter().enumerate() {
    println!("{}: {}", i, val);
}

// Zip
let names = vec!["Alice", "Bob"];
let ages = vec![30, 25];
for (name, age) in names.iter().zip(ages.iter()) {
    println!("{}: {}", name, age);
}
```

### Laziness warning
```rust
// This does NOTHING — iterators are lazy:
v.iter().map(|x| println!("{x}"));

// Use for_each or a for loop instead:
v.iter().for_each(|x| println!("{x}"));
// or:
for x in &v { println!("{x}"); }
```

### Iterating by reference
```rust
// into_iter() takes self by value — consumes the collection
// Use &collection or &mut collection for non-consuming iteration:
let mut values = vec![41];
for x in &mut values { *x += 1; }  // same as values.iter_mut()
for x in &values { assert_eq!(*x, 42); }  // same as values.iter()
// values is still owned here
```

### Iterator panic safety
If an iterator adapter panics, the iterator will be in an unspecified (but memory safe) state. Avoid relying on exact values from a panicked iterator.

### Advanced Iterator Traits (std::iter)

```rust
use std::iter;

// DoubleEndedIterator — can iterate from both ends:
// next_back() pops from the end
let v = vec![1, 2, 3, 4, 5];
let mut iter = v.iter();
assert_eq!(iter.next(), Some(&1));
assert_eq!(iter.next_back(), Some(&5));
// rfold, rfind, rposition — reverse-direction methods

// ExactSizeIterator — knows exact remaining length:
let iter = vec![1, 2, 3].into_iter();
assert_eq!(iter.len(), 3);  // ExactSizeIterator::len
assert_eq!(iter.is_empty(), false);

// FusedIterator — after returning None, always returns None:
// Most std iterators are fused. Useful for safety guarantees.

// TrustedLen — unsafe trait, guarantees exact size hint:
// size_hint() returns (exact, Some(exact))
// Used internally by collect for pre-allocation

// Step — trait for range step types (integers):
// Required for Range<Step>::step_by

// FromIterator — collect() source:
// FromIterator<T> for Vec<T> — collect into Vec
// FromIterator<T> for HashSet<T> — collect into HashSet
// FromIterator<Result<T, E>> for Result<Vec<T>, E> — short-circuits

// Extend — extend a collection from an iterator:
// vec.extend(other_iter) — uses Extend::extend

// Product / Sum — reduce via * or +:
let product: i32 = (1..=5).product();  // 120
let sum: i32 = (1..=5).sum();          // 15
```

### Iterator Constructor Functions (std::iter)

```rust
use std::iter;

// empty — yields nothing:
let mut it: iter::Empty<i32> = iter::empty();
assert_eq!(it.next(), None);

// once — yields exactly one value:
let mut it = iter::once(42);
assert_eq!(it.next(), Some(42));
assert_eq!(it.next(), None);

// once_with — lazily compute one value:
let mut it = iter::once_with(|| expensive());
assert_eq!(it.next(), Some(result));

// repeat — repeat a value forever:
let mut it = iter::repeat(5);
assert_eq!(it.next(), Some(5));  // infinite

// repeat_n — repeat a value N times:
let v: Vec<i32> = iter::repeat_n(7, 3).collect();  // [7, 7, 7]

// repeat_with — repeat a closure forever:
let mut rng = rand::thread_rng();
let randoms: Vec<u32> = iter::repeat_with(|| rng.gen()).take(5).collect();

// successors — generate from previous value:
let fib = iter::successors(Some((0, 1)), |&(a, b)| Some((b, a + b)))
    .map(|(a, _)| a)
    .take(10)
    .collect::<Vec<_>>();  // [0, 1, 1, 2, 3, 5, 8, 13, 21, 34]

// from_fn — generate from a closure returning Option:
let mut i = 0;
let mut it = iter::from_fn(|| {
    i += 1;
    if i <= 3 { Some(i) } else { None }
});
assert_eq!(it.collect::<Vec<_>>(), vec![1, 2, 3]);

// zip — combine two iterators (also a free function):
let pairs: Vec<_> = iter::zip(&[1, 2, 3], &["a", "b", "c"]).collect();
// [(1, "a"), (2, "b"), (3, "c")]

// chain — also a free function:
let combined: Vec<_> = iter::chain([1, 2], [3, 4]).collect();  // [1, 2, 3, 4]

// Additional iterator adaptors:
// intersperse — insert a separator between elements:
let v: Vec<_> = [1, 2, 3].iter().copied().intersperse(0).collect();
// [1, 0, 2, 0, 3]

// intersperse_with — separator from a closure:
let mut toggle = true;
let v: Vec<_> = [1, 2, 3].iter().copied()
    .intersperse_with(|| { toggle = !toggle; if toggle { -1 } else { 0 } })
    .collect();

// map_while — map until first None, then stop:
let v: Vec<_> = [1, 2, 3, -1, 4].iter()
    .map_while(|x| if *x > 0 { Some(x * 2) } else { None })
    .collect();
// [2, 4, 6]

// map_windows — sliding window map (returns arrays):
let v: Vec<[i32; 2]> = [1, 2, 3, 4].iter().copied()
    .map_windows(|w: &[i32; 2]| [w[0], w[1]])
    .collect();
// [[1,2], [2,3], [3,4]]

// array_chunks — chunk into arrays (nightly):
// let v: Vec<[i32; 2]> = [1, 2, 3, 4].iter().copied().array_chunks::<2>().collect();

// repeat_n — repeat element exactly n times (stable since 1.97):
// Unlike repeat().take(n), repeat_n can return the original value
// (not a clone) for the last element.
use std::iter;
let mut four_fours = iter::repeat_n(4, 4);
assert_eq!(Some(4), four_fours.next());
assert_eq!(Some(4), four_fours.next());
assert_eq!(Some(4), four_fours.next());
assert_eq!(Some(4), four_fours.next());
assert_eq!(None, four_fours.next());

// For non-Copy types, last element is the original (not cloned):
let v: Vec<i32> = Vec::with_capacity(123);
let mut it = iter::repeat_n(v, 5);
for _ in 0..4 { let _cloned = it.next().unwrap(); }
let last = it.next().unwrap();  // original — capacity 123 preserved
```

## Recursion

```rust
fn factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}

// Tail recursion is not optimized — prefer iteration for deep recursion
```
