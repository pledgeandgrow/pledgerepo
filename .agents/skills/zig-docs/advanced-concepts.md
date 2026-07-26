# Advanced Concepts (Zig)

## opaque

`opaque {}` declares a new type with an unknown (but non-zero) size and alignment. It can contain declarations the same as structs, unions, and enums.

```zig
const Derp = opaque {};
const Wat = opaque {};

extern fn bar(d: *Derp) void;

fn foo(w: *Wat) callconv(.c) void {
    bar(w);  // ❌ Error: expected *Derp, found *Wat
}
```

Used for type safety when interacting with C code that does not expose struct details. `void` has known size 0; `anyopaque` has unknown non-zero size.

## Zero Bit Types

Types where `@sizeOf` is 0:
- `void`
- `u0` and `i0`
- Arrays and Vectors with len 0, or with element type that is a zero bit type
- An enum with only 1 tag
- A struct with all fields being zero bit types
- A union with only 1 field which is a zero bit type

These types can only ever have one possible value, requiring 0 bits. Code using these types generates no machine code:

```zig
export fn entry() void {
    var x: void = {};
    var y: void = {};
    x = y;
    y = x;
}
// Generates only function prologue/epilogue — no body code
```

### void as Generic Parameter

`void` is useful for instantiating generic types. Use `void` as the Value type to make a Map into a Set:

```zig
test "turn HashMap into a set with void" {
    var map = std.AutoHashMap(i32, void).init(std.testing.allocator);
    defer map.deinit();

    try map.put(1, {});
    try map.put(2, {});
    try std.testing.expect(map.contains(2));
    try std.testing.expect(!map.contains(3));
    _ = map.remove(2);
    try std.testing.expect(!map.contains(2));
}
```

Using `void` as the value type means the hash map entry type has no value field, taking up less space. All code dealing with storing/loading the value is deleted.

### Ignoring Expression Values

Expressions of type `void` are the only ones whose value can be ignored. Ignoring a non-void expression is a compile error:

```zig
test "ignoring expression value" {
    foo();  // ❌ Error: value of type 'i32' ignored
}
fn foo() i32 { return 1234; }

// Fix: explicitly discard with _
test "explicitly ignoring" {
    _ = foo();  // ✅ OK
}
```

## Result Location Semantics

Every Zig expression and sub-expression is assigned optional result location information:
- **Result Type** — what type the expression should have
- **Result Location** — where the resulting value should be placed in memory

This is the primary mechanism of type inference in Zig.

### Result Types

Result types are propagated recursively through expressions:

```zig
const S = struct { x: u32 };
const val: u64 = 123;
const s: S = .{ .x = @intCast(val) };
// .{ .x = @intCast(val) } has result type `S` (from type annotation)
// @intCast(val) has result type `u32` (from field type S.x)
// val has no result type (can be any integer type)
```

Cast builtins like `@intCast` use their result type to determine the target type. Where the result type is not known from context, use `@as`:

```zig
const x = @as(u32, 42);
```

### Result Type Propagation Table

| Expression | Sub-expression x gets result type |
|------------|----------------------------------|
| `const val: T = x` | `T` |
| `var val: T = x` | `T` |
| `val = x` | `@TypeOf(val)` |
| `@as(T, x)` | `T` |
| `&x` (result type `*T`) | `T` |
| `&x` (result type `[]T`) | `T` |
| `f(x)` | parameter type of `f` |
| `.{x}` (result type `T`) | `@FieldType(T, "0")` |
| `.{ .a = x }` (result type `T`) | `@FieldType(T, "a")` |
| `x << y` | `y` gets `std.math.Log2IntCeil(@TypeOf(x))` |

### Result Locations

Result locations prevent intermediate copies. For `x = e`, the expression `e` is given a result location of `&x`, writing directly to `x`.

```zig
foo = .{ .a = x, .b = y };
// Desugars to:
// foo.a = x;
// foo.b = y;
```

**Important caveat:** This can cause unexpected behavior when swapping:

```zig
var arr: [2]u32 = .{ 1, 2 };
arr = .{ arr[1], arr[0] };
// Equivalent to:
// arr[0] = arr[1];  // arr[0] is now 2
// arr[1] = arr[0];  // arr[1] is now 2 (not 1!)
// This does NOT swap!
```

## Atomics

Zig provides atomic operations via builtins. All atomic operations require a memory ordering.

### Atomic Orderings

```zig
const AtomicOrder = std.builtin.AtomicOrder;
// .unordered — no ordering constraints
// .monotonic — no ordering of other operations
// .acquire — subsequent reads see all writes before release
// .release — prior writes visible to acquire
// .acq_rel — both acquire and release
// .seq_cst — sequentially consistent (strongest)
```

### Atomic Operations

```zig
var counter = std.atomic.Value(u32).init(0);

// Load
const val = counter.load(.seq_cst);

// Store
counter.store(42, .seq_cst);

// Read-Modify-Write
const old = counter.fetchAdd(1, .seq_cst);
const old2 = counter.fetchSub(1, .seq_cst);
const old3 = counter.swap(5, .seq_cst);

// Compare-Exchange
const result = counter.cmpxchgStrong(expected, new, .seq_cst, .seq_cst);
// Returns null on success, ?T (old value) on failure

const result2 = counter.cmpxchgWeak(expected, new, .seq_cst, .seq_cst);
// May spuriously fail even if expected matches
```

### Builtin Atomic Functions

```zig
@atomicLoad(T, ptr, ordering) T
@atomicStore(T, ptr, value, ordering) void
@atomicRmw(T, ptr, op, operand, ordering) T  // returns old value
@cmpxchgStrong(T, ptr, expected, new, success_order, fail_order) ?T
@cmpxchgWeak(T, ptr, expected, new, success_order, fail_order) ?T
```

AtomicRmwOp: `.add`, `.sub`, `.and`, `.or`, `.xor`, `.nand`, `.min`, `.max`, `.xchg`

## Async Functions

Async functions regressed with the release of 0.11.0. The current plan is to reintroduce them as lower-level primitives that power I/O implementations.

Tracking issue: [Proposal: stackless coroutines as low-level primitives](https://github.com/ziglang/zig/issues/23446)

Keywords `async`, `await`, `suspend`, `resume`, `nosuspend`, and `anyframe` are reserved but not fully functional.

## Containers

A container in Zig is any syntactical construct that acts as a namespace to hold variable and function declarations. Containers are also type definitions which can be instantiated.

**Container types:**
- Structs
- Enums
- Unions
- Opaques
- Zig source files themselves

Although containers use curly braces, they should not be confused with blocks or functions. **Containers do not contain statements** — only declarations.

## Shadowing

Identifiers in inner blocks **cannot shadow** outer identifiers:

```zig
test "shadowing" {
    var x: i32 = 1;
    {
        var x: i32 = 2;  // ❌ Error: redefinition of 'x'
    }
}
```

## Empty Blocks

Empty blocks are valid and have type `void`:

```zig
const x = {};  // type: void
```
