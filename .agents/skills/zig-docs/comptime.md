# Comptime (Zig)

Zig places importance on whether an expression is known at compile-time. `comptime` is the mechanism for compile-time metaprogramming.

## comptime Expression

```zig
// Force compile-time evaluation
comptime {
    const x = 5;
    // All operations here are evaluated at compile time
}

// comptime variable
fn foo(comptime x: i32) i32 {
    return x * 2;
}
```

## comptime Parameters

```zig
fn max(comptime T: type, a: T, b: T) T {
    return if (a > b) a else b;
}

const m = max(i32, 5, 10);
```

## Generic Data Structures

Types are first-class values. Functions can return types:

```zig
fn List(comptime T: type) type {
    return struct {
        items: []T,
        len: usize,

        pub fn init() List(T) {
            return .{ .items = &.{}, .len = 0 };
        }
    };
}

const IntList = List(i32);
const StringList = List([]const u8);
```

Functions called at compile-time are memoized — calling `List(i32)` twice returns the same type.

## Compile-Time Variables

```zig
// Top-level const is always comptime
const PI = 3.14159;

// comptime var in blocks
comptime {
    var sum = 0;
    for (0..10) |i| {
        sum += i;
    }
}
```

## Compile-Time Function Calls

Any function can be called at compile time if all arguments are comptime-known:

```zig
fn factorial(n: u32) u32 {
    if (n <= 1) return 1;
    return n * factorial(n - 1);
}

// Computed at compile time
const f5 = comptime factorial(5);  // 120
```

## @typeInfo — Type Reflection

```zig
const info = @typeInfo(i32);
// info is a union(enum) with fields for each type kind:
// .int, .float, .bool, .@"struct", .@"enum", .@"union", .pointer, etc.

// For structs:
const Point = struct { x: i32, y: i32 };
const point_info = @typeInfo(Point).@"struct";
// point_info.fields — array of field info
// point_info.decls — array of declarations

// For enums:
const Small = enum { one, two, three };
const enum_info = @typeInfo(Small).@"enum";
// enum_info.fields — field names and values
// enum_info.tag_type — underlying integer type
```

## @Type — Construct Types at Comptime

Build types programmatically at compile time:

```zig
const IntType = @Type(.{
    .@"int" = .{
        .signedness = .signed,
        .bits = 32,
    },
});
// IntType is i32
```

## @TypeOf

Get the type of an expression:

```zig
const x: i32 = 5;
const T = @TypeOf(x);  // i32

// Multiple arguments — peer type resolution
const T2 = @TypeOf(1, 2u8, 3i32);  // i32
```

## @hasDecl / @hasField

Check for declarations at compile time:

```zig
const has_init = @hasDecl(MyStruct, "init");
const has_x = @hasField(MyStruct, "x");
```

## @field

Access fields by comptime-known name:

```zig
const value = @field(obj, "x");  // same as obj.x
@field(obj, "x") = 10;           // same as obj.x = 10
```

## @import

Import other Zig source files:

```zig
const std = @import("std");
const my_module = @import("my_module.zig");
```

## @embedFile

Embed a file as a comptime string:

```zig
const shader_source = @embedFile("shader.glsl");
```

## @compileError

Trigger a compile error:

```zig
if (@import("builtin").os.tag == .fuchsia) {
    @compileError("fuchsia not supported");
}
```

## @compileLog

Debug compile-time values (prints to stderr at compile time):

```zig
comptime {
    @compileLog(@TypeOf(my_var), my_var);
}
```

## Inline Loops

`inline for` and `inline while` unroll loops at compile time:

```zig
inline for (.{ i32, f32, bool }) |T| {
    // T is comptime-known, body is unrolled
    processType(T);
}
```

## Inline Switch

```zig
inline switch (kind) {
    .int => |comptime_kind| {
        // comptime_kind is known at compile time
    },
    else => {},
}
```

## Case Study: print in Zig

`std.debug.print` uses comptime to validate format strings:

```zig
// The format string is validated at compile time
std.debug.print("Hello, {s}!\n", .{"World"});
std.debug.print("{d} + {d} = {d}\n", .{ 1, 2, 3 });
std.debug.print("{x}\n", .{0xff});

// This is a compile error:
// std.debug.print("{d}\n", .{"not a number"});  // ❌ format specifier mismatch
```

## Compile-Time Bounds

The compiler limits comptime operations to prevent infinite loops:

```zig
// Increase the branch quota
@setEvalBranchQuota(10000);

comptime {
    var i = 0;
    while (i < 1000) : (i += 1) {
        // Without @setEvalBranchQuota, this fails
    }
}
```

## @inComptime

Check if currently in a comptime context:

```zig
const is_comptime = @inComptime();
```
