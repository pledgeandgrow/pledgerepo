# Values & Primitive Types (Zig)

## Primitive Types

| Zig Type | C Equivalent | Description |
|----------|-------------|-------------|
| `i8` | `int8_t` | Signed 8-bit integer |
| `u8` | `uint8_t` | Unsigned 8-bit integer |
| `i16` | `int16_t` | Signed 16-bit integer |
| `u16` | `uint16_t` | Unsigned 16-bit integer |
| `i32` | `int32_t` | Signed 32-bit integer |
| `u32` | `uint32_t` | Unsigned 32-bit integer |
| `i64` | `int64_t` | Signed 64-bit integer |
| `u64` | `uint64_t` | Unsigned 64-bit integer |
| `i128` | `__int128` | Signed 128-bit integer |
| `u128` | `unsigned __int128` | Unsigned 128-bit integer |
| `isize` | `intptr_t` | Pointer-sized signed integer |
| `usize` | `uintptr_t` / `size_t` | Pointer-sized unsigned integer |
| `c_char` | `char` | C ABI char |
| `c_short` | `short` | C ABI short |
| `c_ushort` | `unsigned short` | C ABI unsigned short |
| `c_int` | `int` | C ABI int |
| `c_uint` | `unsigned int` | C ABI unsigned int |
| `c_long` | `long` | C ABI long |
| `c_ulong` | `unsigned long` | C ABI unsigned long |
| `c_longlong` | `long long` | C ABI long long |
| `c_ulonglong` | `unsigned long long` | C ABI unsigned long long |
| `c_longdouble` | `long double` | C ABI long double |
| `f16` | `_Float16` | 16-bit float |
| `f32` | `float` | 32-bit float |
| `f64` | `double` | 64-bit float |
| `f80` | `long double` | 80-bit float (x86) |
| `f128` | `_Float128` | 128-bit float |
| `bool` | `bool` | Boolean |
| `void` | `void` | Unit type — value is `void{}` |
| `anyopaque` | `void*` | Opaque type (unknown size) |
| `noreturn` | — | Never returns |
| `type` | — | Type of types |
| `anyerror` | — | Global error set |
| `comptime_int` | — | Compile-time integer literal |
| `comptime_float` | — | Compile-time float literal |

### Arbitrary Bit-Width Integers

Use `i` or `u` followed by digits. Max bit-width is 65535.

```zig
const small: i7 = -5;
const wide: u42 = 42;
```

## Primitive Values

| Value | Type | Description |
|-------|------|-------------|
| `true` | `bool` | Boolean true |
| `false` | `bool` | Boolean false |
| `null` | `?T` | Optional null |
| `undefined` | any | Uninitialized value (use with caution) |

## Integers

### Integer Literals

```zig
const decimal = 98222;
const hex = 0xff;
const binary = 0b1010;
const octal = 0o755;
const underscored = 1_000_000;  // underscores for readability
```

### Runtime Integer Values

Integer operations are safe by default in Debug/ReleaseSafe — overflow is detected. Use wrapping operators (`+%`, `-%`, `*%`) for explicit wrapping behavior.

```zig
const std = @import("std");

test "integer operations" {
    try std.testing.expectEqual(@as(i32, 10), 5 + 5);
    try std.testing.expectEqual(@as(i32, 20), 5 * 4);
    // Wrapping arithmetic
    try std.testing.expectEqual(@as(u8, 0), 255 +% 1);
}
```

## Floats

### Float Literals

```zig
const f: f32 = 3.14159;
const sci: f64 = 2.1e4;
const hex_float: f64 = 0x1.8p1;  // 1.5 * 2^1 = 3.0
```

### Floating Point Operations

Float operations follow IEEE 754. Special values: `inf`, `-inf`, `nan`.

```zig
const std = @import("std");

test "float operations" {
    try std.testing.expectEqual(2.3333333, @as(f32, 7.0 / 3.0));
    try std.testing.expect(std.math.isNan(0.0 / 0.0));
}
```

## Boolean

```zig
const a = true and false;  // false
const b = true or false;   // true
const c = !true;           // false
```

## Assignment

Assignment is a statement, not an expression. You cannot use `a = b = c`.

```zig
var x: i32 = 5;
x = 10;
// x = y = 5;  // ❌ Error — assignment is not an expression
```

### Destructuring Assignment

Destructuring separates elements of indexable aggregate types (Tuples, Arrays, Vectors):

```zig
var x: u32 = undefined;
var y: u32 = undefined;
var z: u32 = undefined;

const tuple = .{ 1, 2, 3 };
x, y, z = tuple;  // x=1, y=2, z=3

const array = [_]u32{ 4, 5, 6 };
x, y, z = array;  // x=4, y=5, z=6

const vector: @Vector(3, u32) = .{ 7, 8, 9 };
x, y, z = vector;  // x=7, y=8, z=9
```

### Mixed Declaration Destructuring

The LHS can mix existing lvalues and new variable declarations:

```zig
var x: u32 = undefined;
const tuple = .{ 1, 2, 3 };
x, var y: u32, const z = tuple;
// y is mutable, z is const
y = 100;

// Use _ to discard unwanted values
_, x, _ = tuple;  // only x is assigned (x = 2)
```

### comptime Destructuring

```zig
comptime x, var y, const z = .{ 1, 2, 3 };
// Entire destructure evaluated at comptime
// y is a comptime var, z is a comptime const
```

A destructuring expression may only appear within a block (not at container scope).

## String Literals

String literals are single-item pointers to sentinel-terminated arrays:

```zig
const msg = "hello";  // type: *const [5:0]u8
// String literals are immutable and null-terminated
```

### Multiline String Literals

```zig
const multi =
    \\Line 1
    \\Line 2
    \\Line 3
;
```

### Unicode Code Point Literals

```zig
const euro = '€';   // type: comptime_int, value: 0x20AC
const heart = '❤';  // type: comptime_int, value: 0x2764
```

## undefined

`undefined` is a special value that can be assigned to any type. It leaves the memory uninitialized. Use only when you will immediately initialize it:

```zig
var buffer: [100]u8 = undefined;
// Must fill buffer before reading it
```

## void

The `void` type has exactly one value: `void{}`. It has zero size.

```zig
fn doNothing() void {}
const v: void = {};
```

## Zero Bit Types

Types where `@sizeOf` is 0:
- `void`
- `u0` and `i0`
- Arrays/Vectors with len 0, or with zero-bit element type
- An enum with only 1 tag
- A struct with all fields being zero bit types
- A union with only 1 field which is a zero bit type

These types generate no machine code. See `advanced-concepts.md` for full details.

## Identifiers

Identifiers must start with an alphabetic character or underscore, followed by alphanumeric characters or underscores. They must not overlap with keywords.

### String Identifier Syntax

For names that don't fit the rules (e.g. linking with external libraries), use `@""` syntax:

```zig
const @"identifier with spaces in it" = 0xff;
const @"1SmallStep4Man" = 112358;

pub extern "c" fn @"error"() void;
pub extern "c" fn @"fstat$INODE64"(fd: c.fd_t, buf: *c.Stat) c_int;

const Color = enum {
    red,
    @"really red",
};
const color: Color = .@"really red";
```

## Variables

```zig
// Immutable (const)
const x: i32 = 5;

// Mutable (var)
var y: i32 = 10;
y += 1;
```

Variables must be initialized. Use `undefined` to leave uninitialized:

```zig
var x: i32 = undefined;
x = 1;
```

In Debug and ReleaseSafe mode, Zig writes `0xaa` bytes to undefined memory to catch bugs early.

### Container Level Variables

Top-level `const` and `var` in a source file. Order-independent.

### Static Local Variables

Local `const` values are computed at compile time. `var` locals are runtime.

### Thread Local Variables

```zig
threadlocal var counter: i32 = 0;
```

## Operator Precedence

From highest to lowest:

```
x() x[] x.y x.* x.? a!b x{}    // suffix
!x -x -%x ~x &x ?x             // prefix
* / % ** *% *|                  // multiplication
+ - ++ +% -% +| -|              // addition
<< >> <<|                       // bit shift
& ^ |                            // bitwise
orelse catch                     // optional/error
== != < > <= >=                  // comparison
and or                           // boolean
= *= /= += -= <<= >>= &= ^= |=  // assignment
```

## All Operators

### Arithmetic

| Operator | Description |
|----------|-------------|
| `a + b` | Addition (can overflow) |
| `a +% b` | Wrapping addition |
| `a +\| b` | Saturating addition (clamps at max) |
| `a - b` | Subtraction (can overflow) |
| `a -% b` | Wrapping subtraction |
| `a -\| b` | Saturating subtraction (clamps at 0) |
| `a * b` | Multiplication (can overflow) |
| `a *% b` | Wrapping multiplication |
| `a *\| b` | Saturating multiplication |
| `a / b` | Division (signed must be comptime-known positive; use @divTrunc/@divFloor/@divExact otherwise) |
| `a % b` | Remainder (signed/float must be comptime-known positive; use @rem/@mod otherwise) |
| `-a` | Negation (can overflow) |
| `-%a` | Wrapping negation |

### Bitwise

| Operator | Description |
|----------|-------------|
| `a & b` | Bitwise AND |
| `a \| b` | Bitwise OR |
| `a ^ b` | Bitwise XOR |
| `~a` | Bitwise NOT |
| `a << b` | Left shift (b must be comptime-known or log2 bits) |
| `a <<\| b` | Saturating left shift (clamps at max) |
| `a >> b` | Right shift |

### Boolean

| Operator | Description |
|----------|-------------|
| `a and b` | Boolean AND (short-circuits) |
| `a or b` | Boolean OR (short-circuits) |
| `!a` | Boolean NOT |

### Comparison

| Operator | Description |
|----------|-------------|
| `a == b` | Equal (also works on type, packed struct) |
| `a != b` | Not equal |
| `a < b` | Less than |
| `a > b` | Greater than |
| `a <= b` | Less than or equal |
| `a >= b` | Greater than or equal |

### Optional & Error

| Operator | Description |
|----------|-------------|
| `a orelse b` | Unwrap optional, fallback to b |
| `a.?` | Unwrap optional (panics if null) |
| `a catch b` | Unwrap error union, fallback to b |
| `a catch \|err\| b` | Unwrap error union, capture error |
| `a == null` | Check if optional is null |
| `a != null` | Check if optional is non-null |

### Array & Error Set

| Operator | Description |
|----------|-------------|
| `a ++ b` | Array concatenation (comptime only) |
| `a ** b` | Array repetition (comptime only) |
| `a \|\| b` | Error set merging |

### Pointer

| Operator | Description |
|----------|-------------|
| `a.*` | Dereference pointer |
| `&a` | Address-of |
