# Functions (Zig)

## Declaration

```zig
fn add(a: i8, b: i8) i8 {
    if (a == 0) return b;
    return a + b;
}
```

## Specifiers

| Specifier | Description |
|-----------|-------------|
| `pub` | Visible when imported |
| `export` | Externally visible in generated object file, C ABI |
| `extern` | Resolved at link time |
| `inline` | Force inline at all call sites (compile error if impossible) |
| `naked` | No function prologue/epilogue (for assembly integration) |

```zig
// Export with C ABI
export fn sub(a: i8, b: i8) i8 {
    return a - b;
}

// External function
extern "c" fn atan2(a: f64, b: f64) f64;

// External with calling convention
extern "kernel32" fn ExitProcess(exit_code: u32) callconv(.winapi) noreturn;

// Public (importable)
pub fn sub2(a: i8, b: i8) i8 {
    return a - b;
}

// Inline
inline fn shiftLeftOne(a: u32) u32 {
    return a << 1;
}
```

## Calling Conventions

```zig
fn foo() callconv(.c) void { }       // C calling convention
fn bar() callconv(.winapi) void { }  // Windows API
fn baz() callconv(.naked) void { }   // No prologue/epilogue
fn qux() callconv(.inline) void { }  // Force inline
```

## Function Pointers

Function pointers are prefixed with `*const `:

```zig
const Call2Op = *const fn (a: i8, b: i8) i8;

fn doOp(fnCall: Call2Op, op1: i8, op2: i8) i8 {
    return fnCall(op1, op2);
}

// Usage
const result = doOp(add, 5, 6);  // 11
```

**Note:** Function bodies are comptime-only types. Function pointers may be runtime-known.

## Pass-by-Value Parameters

Zig passes parameters by value or by reference — the compiler decides based on size and optimization. Small types are passed by value; large types by reference. The semantics are always pass-by-value (immutable copy).

## Function Parameter Type Inference

```zig
fn doSomething(x: anytype) void {
    // @TypeOf(x) gives the actual type
    const T = @TypeOf(x);
    // ...
}
```

## inline fn

`inline fn` forces inlining at all call sites. If the function cannot be inlined, it is a compile-time error.

```zig
inline fn double(x: u32) u32 {
    return x * 2;
}
```

## @branchHint

Hint the optimizer about branch likelihood:

```zig
fn handleError() noreturn {
    @branchHint(.cold);  // rarely called
    while (true) {}
}
```

## Function Reflection

```zig
const info = @typeInfo(@TypeOf(my_fn));
// info.@"fn".params — parameter info
// info.@"fn".return_type — return type
// info.@"fn".calling_convention
```

## Variadic Functions

Only available with C calling convention:

```zig
extern fn printf(format: [*:0]const u8, ...) c_int;
```

## Recursion

```zig
fn factorial(n: u32) u32 {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}
```

## Generic Functions

Use `comptime` parameters for generics:

```zig
fn max(comptime T: type, a: T, b: T) T {
    return if (a > b) a else b;
}

// Usage
const m = max(i32, 5, 10);  // 10
```

Or use `anytype` for inferred generics:

```zig
fn maxAny(a: anytype, b: @TypeOf(a)) @TypeOf(a) {
    return if (a > b) a else b;
}

// Usage
const m = maxAny(5, 10);  // 10, type inferred as comptime_int
```

## Entry Point

```zig
// Standard main with init
pub fn main(init: std.process.Init) !void {
    try std.Io.File.stdout().writeStreamingAll(init.io, "Hello!\n");
}

// Simple main (no errors)
pub fn main() void {
    std.debug.print("Hello!\n", .{});
}
```

## @call

Control function calling behavior:

```zig
// Normal call
const result = @call(.auto, my_fn, .{ arg1, arg2 });

// Compile-time call
const result = @call(.compile_time, my_fn, .{ arg1, arg2 });

// No tail call optimization
const result = @call(.no_tail, my_fn, .{ arg1, arg2 });

// Tail call
const result = @call(.tail, my_fn, .{ arg1, arg2 });
```
