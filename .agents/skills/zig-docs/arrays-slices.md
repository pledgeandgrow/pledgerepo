# Arrays, Slices & Vectors (Zig)

## Arrays

Arrays have compile-time known length. The length is part of the type.

```zig
// Array literal
const message = [_]u8{ 'h', 'e', 'l', 'l', 'o' };

// Alternative initialization using result location
const alt_message: [5]u8 = .{ 'h', 'e', 'l', 'l', 'o' };

// Get array size
const len = message.len;  // 5 (comptime known)

// String literals are arrays
const hello = "hello";  // type: *const [5:0]u8
```

### Array Concatenation & Repetition (comptime)

```zig
const part_one = [_]i32{ 1, 2, 3, 4 };
const part_two = [_]i32{ 5, 6, 7, 8 };
const all_of_it = part_one ++ part_two;  // [8]i32

// Repetition with **
const pattern = "ab" ** 3;  // "ababab"
const all_zero = [_]u16{0} ** 10;  // [10]u16
```

### Modifying Arrays

```zig
var some_integers: [100]i32 = undefined;
for (&some_integers, 0..) |*item, i| {
    item.* = @intCast(i);
}
```

### Multidimensional Arrays

```zig
const matrix: [3][3]i32 = .{
    .{ 1, 2, 3 },
    .{ 4, 5, 6 },
    .{ 7, 8, 9 },
};
```

### Sentinel-Terminated Arrays

```zig
const sentinel_array: [4:0]u8 = .{ 'a', 'b', 'c', 'd' };
// The sentinel value 0 is stored at index 4
```

### Destructuring Arrays

```zig
const arr = [_]u8{ 1, 2, 3 };
const { first, rest.. } = arr;  // first = 1, rest = [2, 3]
```

### Compile-Time Array Initialization

```zig
const Point = struct { x: i32, y: i32 };

var fancy_array = init: {
    var initial_value: [10]Point = undefined;
    for (&initial_value, 0..) |*pt, i| {
        pt.* = Point{ .x = @intCast(i), .y = @intCast(i * 2) };
    }
    break :init initial_value;
};
```

## Slices

A slice is a pointer and a length (fat pointer). Length is known at runtime.

```zig
var array = [_]i32{ 1, 2, 3, 4 };
var known_at_runtime_zero: usize = 0;
_ = &known_at_runtime_zero;
const slice = array[known_at_runtime_zero..array.len];  // []i32

// Alternative initialization using result location
const alt_slice: []const i32 = &.{ 1, 2, 3, 4 };
```

### Slice Properties

```zig
slice[0]      // Index access (bounds-checked)
slice[1..3]   // Sub-slice
slice.len     // Length
slice.ptr     // Many-item pointer ([*]i32)
```

### Comptime-Known Slicing

If start and end are comptime-known, the result is a pointer to an array, not a slice:

```zig
const array_ptr = array[0..array.len];  // type: *[array.len]i32
```

### Slice-by-Length

```zig
var runtime_start: usize = 1;
_ = &runtime_start;
const length = 2;
const array_ptr_len = array[runtime_start..][0..length];  // *[2]i32
```

### Empty Slices

```zig
const empty1 = &[0]u8{};
const empty2: []u8 = &.{};
```

### Sentinel-Terminated Slices

```zig
const sentinel_slice: [:0]const u8 = "hello";  // length 5, sentinel 0 at [5]
```

## Vectors

Vectors are SIMD-friendly types for parallel operations on multiple values.

```zig
const std = @import("std");
const Vec4 = @Vector(4, f32);

test "vector operations" {
    const a: Vec4 = .{ 1, 2, 3, 4 };
    const b: Vec4 = .{ 5, 6, 7, 8 };
    const c = a + b;  // .{ 6, 8, 10, 12 }
    try std.testing.expectEqual(@as(f32, 6), c[0]);
}
```

### Relationship with Arrays

Vectors can be converted to/from arrays:

```zig
const arr: [4]f32 = .{ 1, 2, 3, 4 };
const vec: @Vector(4, f32) = arr;
const back: [4]f32 = vec;
```

### Destructuring Vectors

```zig
const v: @Vector(3, i32) = .{ 10, 20, 30 };
const { x, y, z } = v;  // x=10, y=20, z=30
```

### Vector Operations

```zig
const a: @Vector(4, i32) = .{ 1, 2, 3, 4 };
const b: @Vector(4, i32) = .{ 5, 6, 7, 8 };

// Arithmetic
const sum = a + b;
const diff = a - b;
const prod = a * b;

// Comparison (produces vector of bools)
const cmp = a < b;  // .{ true, true, true, true }

// Reduction
const total = @reduce(.Add, a);  // 10
const max_val = @reduce(.Max, a);  // 4
```
