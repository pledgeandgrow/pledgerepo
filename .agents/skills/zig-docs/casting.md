# Casting & Type Coercion (Zig)

Zig has three kinds of type conversion:
1. **Type Coercion** — implicit, safe, unambiguous conversions
2. **Explicit Casts** — via `@as`, `@intCast`, `@floatCast`, etc.
3. **Peer Type Resolution** — when multiple operand types need a common type

## Type Coercion

Automatic, implicit conversions that are known to be completely safe:

```zig
// int to optional
const opt: ?i32 = 5;

// int to error union
const eu: anyerror!i32 = 5;

// slice to const slice
const arr: [5]u8 = .{ 1, 2, 3, 4, 5 };
const slice: []const u8 = &arr;

// array pointer to slice
const ptr: *[5]u8 = &arr;
const s: []u8 = ptr;

// pointer to optional pointer
const p: *u8 = &arr[0];
const op: ?*u8 = p;

// T to ?*T (coercion to optional)
const np: ?*u8 = null;

// comptime_int to any integer type
const x: u64 = 5;  // 5 is comptime_int

// comptime_float to any float type
const f: f64 = 3.14;
```

### Coercion Rules

- `T` → `?T` (value to optional)
- `T` → `anyerror!T` (value to error union)
- `[]T` → `[]const T` (mutable to const slice)
- `*[N]T` → `[]T` (array pointer to slice)
- `*[N]T` → `*[*]T` (array pointer to many-pointer)
- `*[N]T` → `[]const T` (through const)
- `?*T` → `?[*]T` (optional single to optional many)
- `?*[N]T` → `?[]T` (optional array pointer to optional slice)
- `comptime_int` → any integer type
- `comptime_float` → any float type
- `null` → any `?T`
- `error{A}` → `error{A, B}` (subset to superset)

## Explicit Casts

### @as — Safe Type Coercion

Preferred way to convert between types when the conversion is unambiguous and safe:

```zig
const x: i32 = @as(i32, 5);
```

### @intCast — Integer to Integer

Runtime-checked integer conversion. Truncation or overflow is safety-checked:

```zig
const small: u8 = @intCast(large_value);  // panics if > 255 in safe modes
```

### @floatCast — Float to Float

```zig
const f32_val: f32 = @floatCast(f64_val);
```

### @floatFromInt — Integer to Float

```zig
const f: f32 = @floatFromInt(42);
```

### @intFromFloat — Float to Integer

Runtime-checked — panics if float is out of integer range:

```zig
const i: i32 = @intFromFloat(3.14);
```

### @intFromBool — Boolean to Integer

```zig
const b: u1 = @intFromBool(true);  // 1
```

### @intFromEnum — Enum to Integer

```zig
const val: u2 = @intFromEnum(Value.one);
```

### @enumFromInt — Integer to Enum

```zig
const e = @enumFromInt(@as(u2, 1));  // Value.one
```

### @intFromError — Error to Integer

```zig
const code: u16 = @intFromError(error.NotFound);
```

### @errorFromInt — Integer to Error

```zig
const err = @errorFromInt(@as(u16, 1));
```

### @intFromPtr — Pointer to Integer

```zig
const addr: usize = @intFromPtr(ptr);
```

### @ptrFromInt — Integer to Pointer

```zig
const ptr: *u8 = @ptrFromInt(0x1000);
```

### @ptrCast — Pointer Reinterpret

```zig
const byte_ptr: *u8 = @ptrCast(&int_val);
```

### @bitCast — Bit-Level Reinterpret

Reinterpret bits of one type as another. Types must have the same bit size:

```zig
const bits: u32 = @bitCast(f32_val);
const back: f32 = @bitCast(bits);
```

### @truncate — Integer Truncation

Truncate without safety check (drops high bits):

```zig
const low_byte: u8 = @truncate(large_u32);
```

### @constCast — Remove const

```zig
const mut_ptr: *u8 = @constCast(const_ptr);
```

### @volatileCast — Volatile Cast

```zig
const vol_ptr: *volatile u8 = @volatileCast(regular_ptr);
```

## Peer Type Resolution

When multiple types appear in an expression, Zig resolves them to a common type:

```zig
// Integer peer resolution
const a: i32 = 5;
const b: u8 = 10;
const result = a + b;  // result type: i32 (wider type wins)

// Mixed int and comptime_int
const x = if (condition) @as(i32, 5) else 0;  // i32
```

### Rules

1. If any operand is `comptime_int`, result is the other integer type
2. If any operand is `comptime_float`, result is the other float type
3. Signed and unsigned integers: the type that can hold all values wins
4. `?T` and `null` → `?T`
5. `[]T` and `[]const T` → `[]const T`
6. `*T` and `?*T` → `?*T`

## Zero Bit Types

Types with zero bits (void, empty structs, zero-length arrays) have no runtime representation. They can be freely cast to/from:

```zig
const v: void = {};
const empty: [0]u8 = .{};
```
