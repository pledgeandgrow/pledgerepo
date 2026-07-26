# Control Flow (Zig)

## if

`if` expressions work with `bool`, optionals (`?T`), and error unions (`!T`):

```zig
// Boolean
const result = if (a != b) 47 else 3089;

// if/else if/else
if (a != b) {
    // ...
} else if (a == 9) {
    // ...
} else {
    // ...
}
```

### if with Optionals

```zig
const optional: ?i32 = 5;
if (optional) |value| {
    // value is i32 (unwrapped)
} else {
    // optional was null
}

// Pointer capture
var opt: ?i32 = 3;
if (opt) |*value| {
    value.* = 9;
}
```

### if with Error Unions

```zig
const result: anyerror!u32 = error.BadValue;
if (result) |value| {
    // success — value is u32
} else |err| {
    // error — err is the error value
}

// Ignore error value
if (result) |_| {} else |_| {}

// Check only error
if (result) |_| {} else |err| {
    // handle err
}
```

## switch

```zig
const a: u64 = 10;
const b = switch (a) {
    1, 2, 3 => 0,           // multiple cases
    5...100 => 1,           // inclusive range
    101 => blk: {           // complex branch
        const c: u64 = 5;
        break :blk c * 2 + 1;
    },
    else => 9,              // catch-all (mandatory unless exhaustive)
};
```

### Exhaustive Switching

Switching on enums is exhaustive — all fields must be handled or `else` provided:

```zig
const Foo = enum { string, number, none };
const p = Foo.number;
const what = switch (p) {
    .string => "this is a string",
    .number => "this is a number",
    .none => "this is a none",
};
```

### Switching on Errors

```zig
const result = readFile() catch |err| switch (err) {
    error.NotFound => -1,
    error.PermissionDenied => -2,
    else => -3,
};
```

### Switching on Tagged Unions

```zig
const Item = union(enum) {
    a: u32,
    c: Point,
    d,
    e: u32,
};

var a = Item{ .c = .{ .x = 1, .y = 2 } };
const b = switch (a) {
    .a, .e => |item| item,       // same payload type — combined
    .c => |*item| blk: {         // pointer capture for mutation
        item.*.x += 1;
        break :blk 6;
    },
    .d => 8,
};
```

### Labeled switch

```zig
const result = sw: switch (val) {
    .a => continue :sw .b,  // jump to another prong
    .b => 42,
};
```

### Inline Switch Prongs

```zig
// `inline` captures the comptime value of the prong
const result = switch (enum_val) {
    inline .a, .b => |comptime_tag| {
        // comptime_tag is known at compile time
        @tagName(comptime_tag)
    },
    else => "other",
};
```

## while

```zig
var i: usize = 0;
while (i < 10) {
    i += 1;
}

// break and continue
while (true) {
    if (i == 10) break;
    i += 1;
    if (i < 5) continue;
}
```

### Continue Expression

```zig
while (i < 10) : (i += 1) {
    // i is incremented after each iteration
}

// Complex continue expression
while (i * j < 2000) : ({ i *= 2; j *= 3; }) {
    // ...
}
```

### while with Optionals

```zig
var optional: ?i32 = 5;
while (optional) |val| {
    optional = null;
    // use val
}
```

### while with Error Unions

```zig
var result: anyerror!i32 = 0;
while (result) |val| {
    result = computeNext();
    // use val
} else |err| {
    // handle error
}
```

### while as Expression

```zig
fn rangeHasNumber(begin: usize, end: usize, number: usize) bool {
    var i = begin;
    return while (i < end) : (i += 1) {
        if (i == number) {
            break true;
        }
    } else false;
}
```

### Labeled while

```zig
outer: while (true) {
    while (true) {
        break :outer;  // breaks outer loop
    }
}
```

### inline while

```zig
comptime {
    var i = 0;
    inline while (i < 3) : (i += 1) {
        // loop body is unrolled at compile time
    }
}
```

## for

```zig
const items = [_]i32{ 4, 5, 3, 4, 0 };
var sum: i32 = 0;
for (items) |value| {
    if (value == 0) continue;
    sum += value;
}
```

### With Index

```zig
for (items, 0..) |value, i| {
    // i is usize index
}
```

### Range Syntax

```zig
for (0..5) |i| {
    sum += i;
}
```

### Multi-Object Iteration

```zig
const items = [_]usize{ 1, 2, 3 };
const items2 = [_]usize{ 4, 5, 6 };
for (items, items2) |i, j| {
    count += i + j;
}
```

### By Reference

```zig
var items = [_]i32{ 3, 4, 2 };
for (&items) |*value| {
    value.* += 1;
}
```

### for as Expression (with else)

```zig
const result = for (items) |value| {
    if (value != null) {
        sum += value.?;
    }
} else blk: {
    break :blk sum;
};
```

### Labeled for

```zig
outer: for (matrix) |row| {
    for (row) |val| {
        if (val == target) break :outer;
    }
}
```

### inline for

```zig
inline for (array_of_types) |T| {
    // T is comptime-known, body is unrolled
}
```

## Blocks

Blocks limit scope. Labeled blocks can return values via `break`:

```zig
const x = blk: {
    y += 1;
    break :blk y;
};
```

### Shadowing

Identifiers in inner blocks cannot shadow outer identifiers.

## defer

`defer` executes an expression when the block exits (in reverse order):

```zig
{
    defer std.debug.print("third\n", .{});
    defer std.debug.print("second\n", .{});
    std.debug.print("first\n", .{});
}
// Output: first, second, third
```

### Common Use: Resource Cleanup

```zig
const file = try openFile();
defer file.close();
// use file — close() guaranteed to run
```

### errdefer

Like `defer` but only executes if the block exits with an error:

```zig
fn createThing(allocator: Allocator) !*Thing {
    const ptr = try allocator.create(Thing);
    errdefer allocator.destroy(ptr);
    // If initialization fails, ptr is freed
    ptr.* = try initializeThing();
    return ptr;
}
```

## unreachable

`unreachable` asserts that this code path is never executed. In Debug/ReleaseSafe, it panics. In ReleaseFast, it is undefined behavior.

```zig
const x = 1;
if (x == 1) {
    // ...
} else {
    unreachable;  // we know x is always 1
}
```

## noreturn

Functions that never return: `break`, `continue`, `return`, `unreachable`, infinite loops, calling other `noreturn` functions.

```zig
fn abort() noreturn {
    @branchHint(.cold);
    while (true) {}
}
```
