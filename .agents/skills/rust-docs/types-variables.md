# Types and Variables

**Docs:** https://doc.rust-lang.org/stable/reference/types.html | https://doc.rust-lang.org/book/ch03-02-data-types.html

## Primitive Types

### Integers

| Type | Signed | Unsigned | Size |
|------|--------|----------|------|
| 8-bit | `i8` | `u8` | 1 byte |
| 16-bit | `i16` | `u16` | 2 bytes |
| 32-bit | `i32` | `u32` | 4 bytes |
| 64-bit | `i64` | `u64` | 8 bytes |
| 128-bit | `i128` | `u128` | 16 bytes |
| Pointer-sized | `isize` | `usize` | 4/8 bytes (platform-dependent) |

```rust
let a: i32 = 42;
let b: u64 = 1_000_000;
let c = 255u8;
let d = 0xFF;        // hex
let e = 0o777;       // octal
let f = 0b1111_0000; // binary
let g = b'A';        // byte (u8)
```

### Floating Point

```rust
let x: f32 = 3.14;
let y: f64 = 2.718281828;
let z = 1.0e10;      // scientific notation
```

`f16` and `f128` are available on supported platforms.

### Boolean

```rust
let t: bool = true;
let f: bool = false;
```

### Character

```rust
let c: char = 'A';
let emoji: char = '😀';   // 4 bytes, Unicode scalar value
let newline: char = '\n';
```

### Never Type (`!`)

```rust
fn diverge() -> ! {
    panic!("never returns");
}

// ! coerces to any type
let x: i32 = match Some(1) {
    Some(n) => n,
    None => diverge(),  // ! coerces to i32
};
```

## Compound Types

### Tuples

```rust
let tup: (i32, f64, char) = (42, 3.14, 'x');
let (a, b, c) = tup;       // destructuring
let first = tup.0;         // access by index
let unit: () = ();         // unit type
```

### Arrays

```rust
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let zeros = [0; 10];       // 10 zeros
let first = arr[0];
let len = arr.len();

// Arrays are stack-allocated, fixed size
// Out of bounds access panics at runtime (with bounds checking)
```

### Slices

```rust
let arr = [1, 2, 3, 4, 5];
let slice: &[i32] = &arr[1..4];   // [2, 3, 4]
let full: &[i32] = &arr[..];      // entire array
let from_start = &arr[..2];       // [1, 2]
let to_end = &arr[2..];           // [3, 4, 5]
```

## Strings

```rust
// &str — string slice, immutable, can be static or borrowed
let s1: &str = "hello";           // static string literal
let s2: &str = &String::from("world");

// String — owned, heap-allocated, mutable, growable
let mut s3 = String::from("foo");
s3.push_str("bar");
s3.push('!');

// Conversions
let owned: String = "literal".to_string();
let borrowed: &str = &owned;
let from_bytes = String::from_utf8(vec![0x48, 0x69]).unwrap();
```

## Variables

### let and mut

```rust
let x = 5;          // immutable by default
// x = 10;          // ERROR: cannot assign to immutable variable

let mut y = 5;      // mutable
y = 10;             // OK

let z;              // declare without value
z = 5;              // assign later (must be before use)
```

### Shadowing

```rust
let x = 5;
let x = x + 1;      // shadows previous x, same name, new value
let x = x * 2;      // shadows again
// Shadowing can change type:
let s = "hello";    // &str
let s = s.len();    // usize — type changed via shadowing
```

### const

```rust
const MAX_POINTS: u32 = 100_000;
// Must be explicitly typed
// Must be a constant expression (evaluated at compile time)
// No fixed memory address — inlined wherever used
// Valid in any scope including global
```

### static

```rust
static COUNTER: i32 = 0;
static mut STATE: i32 = 0;  // mutable static — unsafe to access

// static has a fixed memory address
// static lives for entire program ('static lifetime)
// Mutable static requires unsafe block for access
```

## Type Inference

```rust
let x = 5;           // inferred as i32 (default integer type)
let y = 3.14;        // inferred as f64 (default float type)
let z = vec![1, 2, 3];  // inferred as Vec<i32>

// Type annotation needed when inference is ambiguous
let nums: Vec<i32> = (0..10).collect();

// Turbofish syntax for type annotation in expressions
let nums = (0..10).collect::<Vec<i32>>();
let first = "42".parse::<i32>().unwrap();
```

## Type Aliases

```rust
type Kilometers = i32;
type IntVec = Vec<i32>;

let distance: Kilometers = 100;
let numbers: IntVec = vec![1, 2, 3];
```

## Type Casting

```rust
let integer: i32 = 42;
let float: f64 = integer as f64;
let truncated: i32 = 3.99_f64 as i32;  // 3
let char_val: u32 = 'A' as u32;        // 65

// as is for primitive numeric casts only
// For trait conversions, use From/Into/TryFrom/TryInto
```

## Destructuring

```rust
// Tuple destructuring
let (a, b, c) = (1, 2.0, 'x');
let (first, ..) = (1, 2, 3, 4);  // ignore rest

// Struct destructuring
struct Point { x: i32, y: i32 }
let Point { x, y } = Point { x: 1, y: 2 };
let Point { x, .. } = Point { x: 1, y: 2 };  // ignore y

// Array destructuring
let [a, b, c] = [1, 2, 3];
let [first, .., last] = [1, 2, 3, 4, 5];
```

## Dynamically Sized Types (DSTs)

Types whose size is not known at compile time: `str`, `[T]`, `dyn Trait`.

```rust
// DSTs can only be used behind a pointer
let s: &str = "hello";       // &str is a fat pointer (ptr + len)
let slice: &[i32] = &[1, 2, 3];  // fat pointer (ptr + len)
let trait_obj: &dyn ToString = &42;  // fat pointer (ptr + vtable)

// Sized trait — most types implement it
// Generic bounds default to T: Sized
fn foo<T: ?Sized>(t: &T) { /* works with DSTs */ }
```

## NonZero Integer Types (std::num)

`NonZero<T>` wraps an integer with the guarantee that it is never zero. Enables null pointer optimization when used inside `Option`.

```rust
use std::num::NonZero;

// NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize
// NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize

let n: NonZero<u32> = NonZero::new(42).unwrap();  // returns None if 0
let opt: Option<NonZero<u32>> = NonZero::new(0);  // None

// NPO: Option<NonZero<u32>> is same size as u32
assert_eq!(std::mem::size_of::<Option<NonZero<u32>>>(), std::mem::size_of::<u32>());

// Convert back to primitive:
let val: u32 = n.get();
```

## Wrapping and Saturating Arithmetic (std::num)

```rust
use std::num::{Wrapping, Saturating};

// Wrapping — arithmetic wraps around on overflow (no panic)
let Wrapping(max_plus_one) = Wrapping(u32::MAX) + Wrapping(1);
assert_eq!(max_plus_one, 0);

// Saturating — arithmetic saturates at min/max on overflow
let Saturating(max_val) = Saturating(u32::MAX) + Saturating(1);
assert_eq!(max_val, u32::MAX);

// Methods on integer primitives:
let w = 255u8.wrapping_add(1);    // 0
let s = 255u8.saturating_add(1);  // 255
let c = 255u8.checked_add(1);     // None (overflow detected)
let o = 255u8.overflowing_add(1); // (0, true) — value + overflow flag
```

## Floating Point Categories (std::num::FpCategory)

```rust
use std::num::FpCategory;

// FpCategory classifies floating point values:
// - Nan: Not a number
// - Infinite: Positive or negative infinity
// - Zero: Positive or negative zero
// - Subnormal: Subnormal (denormalized) number
// - Normal: Normal number

assert_eq!(f64::INFINITY.classify(), FpCategory::Infinite);
assert_eq!(0.0f64.classify(), FpCategory::Zero);
assert_eq!(1.0f64.classify(), FpCategory::Normal);
assert_eq!(f64::NAN.classify(), FpCategory::Nan);
```

## Parse Errors (std::num)

```rust
use std::num::{ParseIntError, ParseFloatError, IntErrorKind};

// Parsing returns ParseIntError / ParseFloatError:
let err = "abc".parse::<i32>().unwrap_err();

// IntErrorKind variants:
// - Empty: empty string
// - InvalidDigit: invalid character
// - PosOverflow: too large for target type
// - NegOverflow: too small for target type
// - Zero: zero where non-zero required (e.g., NonZero parsing)

match err.kind() {
    IntErrorKind::InvalidDigit => println!("invalid character"),
    IntErrorKind::PosOverflow => println!("too large"),
    _ => println!("other parse error"),
}
```

## Extended Floating-Point Types (std::f16, std::f128)

```rust
// f16 — 16-bit half-precision float (IEEE 754 binary16, stable in 1.97)
let half: f16 = 1.5_f16;
let inf: f16 = f16::INFINITY;
let nan: f16 = f16::NAN;
let eps: f16 = f16::EPSILON;  // ~0.000977
// f16::MIN, f16::MAX, f16::MIN_POSITIVE
// Constants in std::f16::consts: PI, E, LN_2, SQRT_2, etc.

// f128 — 128-bit quad-precision float (IEEE 754 binary128, stable in 1.97)
let quad: f128 = 3.141592653589793238462643383279_f128;
let eps: f128 = f128::EPSILON;  // ~1.93e-34
// f128::INFINITY, f128::NAN, f128::MIN, f128::MAX
// Constants in std::f128::consts: PI, E, etc.

// Both support: +, -, *, /, %, comparisons, .sqrt(), .ln(), .exp(), etc.
// Conversions:
let f: f32 = half.to_f32();  // f16 -> f32
let f16_val = 1.0_f32.to_f16();  // f32 -> f16 (may lose precision)
// Also: to_f64(), from_f32(), from_f64()

// Use cases: f16 for ML/GPU, f128 for high-precision scientific computing
```

## Generic NonZero (std::num::NonZero)

```rust
use std::num::NonZero;

// NonZero<T> — generic non-zero wrapper (1.79+):
// Replaces individual NonZeroI32, NonZeroU64, etc.
// Requires T: ZeroablePrimitive (all int/float types)
let n: NonZero<u32> = NonZero::new(42).unwrap();
let zero_attempt = NonZero::<u32>::new(0);  // None

// NonZero has niche optimization — Option<NonZero<u32>> is same size as u32
// Individual type aliases still exist:
// NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroIsize
// NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize
// All are aliases for NonZero<type>
```
