# Pointers (Zig)

Zig has two kinds of pointers: single-item and many-item.

## Pointer Types

| Type | Description |
|------|-------------|
| `*T` | Single-item pointer to one item |
| `[*]T` | Many-item pointer to unknown number of items |
| `*[N]T` | Pointer to N items (array pointer) |
| `[]T` | Slice (fat pointer: `[*]T` + length) |

## Single-Item Pointers (`*T`)

```zig
const x: i32 = 1234;
const x_ptr = &x;  // type: *const i32

// Dereference
const val = x_ptr.*;  // 1234

// Mutable
var y: i32 = 5678;
const y_ptr = &y;  // type: *i32
y_ptr.* += 1;
```

### Supported Operations

- Deref syntax: `ptr.*`
- Slice syntax: `ptr[0..1]`
- Pointer subtraction: `ptr - ptr`

## Many-Item Pointers (`[*]T`)

```zig
var array = [_]u8{ 1, 2, 3, 4, 5 };
const ptr: [*]u8 = &array;

// Index access
ptr[0]  // 1
ptr[2]  // 3

// Pointer arithmetic
const next = ptr + 1;  // points to array[1]

// Slice syntax
const slice = ptr[0..3];  // [*]u8 or []u8
```

### Supported Operations

- Index syntax: `ptr[i]`
- Slice syntax: `ptr[start..end]` and `ptr[start..]`
- Pointer-integer arithmetic: `ptr + int`, `ptr - int`
- Pointer subtraction: `ptr - ptr`

**Note:** `T` must have a known size — cannot be `anyopaque` or opaque.

## Array Pointers (`*[N]T`)

```zig
var array = [_]i32{ 1, 2, 3, 4, 5 };
const ptr: *[5]i32 = &array;

ptr[0]       // Index access
ptr[1..3]    // Slice
ptr.len      // Length (comptime known: 5)
```

## Slices (`[]T`)

A slice is a fat pointer containing a `[*]T` pointer and a length.

```zig
var array = [_]i32{ 1, 2, 3, 4 };
const slice: []i32 = array[0..];

slice[0]     // Index access (bounds-checked)
slice[1..3]  // Sub-slice
slice.len    // Length (runtime known)
slice.ptr    // [*]i32 (many-item pointer)
```

## Address-Of Operator (`&`)

```zig
var x: i32 = 1234;
const ptr = &x;  // *i32 (or *const i32 if x is const)
```

Taking the address of an individual array element gives a single-item pointer:

```zig
var array = [_]u8{ 1, 2, 3, 4, 5 };
const ptr = &array[2];  // *u8 — no pointer arithmetic
ptr.* += 1;  // array[2] is now 4
```

## volatile

`volatile` keyword on pointers tells the compiler that loads/stores have side effects and should not be optimized away. Useful for memory-mapped I/O.

```zig
// MMIO register
const register: *volatile u32 = @ptrFromInt(0x40021000);
register.* = 0x1;  // Write to hardware register
```

## Alignment

Pointers carry alignment information. Use `@alignOf` to get the required alignment.

```zig
// Explicitly aligned pointer
const aligned_ptr: *align(16) u8 = ...;

// @alignOf returns required alignment
const align_val = @alignOf(u32);  // typically 4

// Pointer alignment is implied by child type
// *u32 == *align(@alignOf(u32)) u32
```

### `@alignCast`

Changes pointer alignment. Adds a runtime safety check:

```zig
fn foo(ptr: *anyopaque) void {
    const aligned: *u32 = @alignCast(@ptrCast(ptr));
}
```

## allowzero

By default, pointers cannot be zero (null). `allowzero` allows zero-address pointers (useful for address 0 being valid on some embedded targets):

```zig
const ptr: *allowzero u8 = @ptrFromInt(0);
```

## Sentinel-Terminated Pointers

```zig
// [*:0]u8 — many-item pointer terminated by 0
const str: [*:0]const u8 = "hello";

// Can iterate until sentinel
var i: usize = 0;
while (str[i] != 0) : (i += 1) {
    // process str[i]
}
```

## Pointer Conversion

```zig
// Pointer to integer
const addr = @intFromPtr(ptr);  // usize

// Integer to pointer
const ptr: *u8 = @ptrFromInt(addr);

// Pointer reinterpret cast
const float_ptr: *f32 = @ptrCast(@alignCast(int_ptr));

// const to mutable
const mut_ptr = @constCast(const_ptr);
```

## Pointer Safety

- **No null pointers** — use optionals (`?*T`) instead
- **Bounds checking** — slice indexing is bounds-checked in safe modes
- **Alignment checking** — `@alignCast` adds runtime alignment checks
- **No pointer arithmetic on single-item pointers** — only `[*]T` supports `+ int`
