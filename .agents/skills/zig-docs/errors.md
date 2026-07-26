# Errors (Zig)

## Error Set Type

```zig
const FileError = error{
    NotFound,
    PermissionDenied,
    Corrupted,
};

// Errors can be merged
const CombinedError = error{NotFound} || error{PermissionDenied};
```

### Inferred Error Sets

If error set is not specified, it is inferred from the function body:

```zig
fn readFile() ![]u8 {
    // Inferred error set includes all errors from try/catch in body
    return error.NotFound;
}
```

### anyerror

`anyerror` is the global error set containing all possible errors. It means the error set is known only at runtime. Prefer inferred error sets for better compile-time checking.

## Error Union Type

The `!` operator creates an error union: `ErrorSet!T`

```zig
const result: FileError!u32 = error.NotFound;
// or
const result: FileError!u32 = 42;
```

### Unwrapping

```zig
// try — unwrap or return error to caller
const value = try readFile();

// catch — handle the error
const value = readFile() catch 0;

// catch with error capture
const value = readFile() catch |err| switch (err) {
    error.NotFound => -1,
    else => -2,
};

// catch with payload and fallback
const value = readFile() catch |err| blk: {
    std.debug.print("Error: {}\n", .{err});
    break :blk 0;
};
```

### Error Propagation

```zig
fn processFile() FileError!void {
    const data = try readFile();  // error propagates to caller
    try processData(data);        // error propagates to caller
}
```

### catch Unreachable

```zig
// Assert that an error will never occur
const value = definitelyOk() catch unreachable;
```

### Error Union in if

```zig
const result: anyerror!u32 = 42;
if (result) |value| {
    // success
} else |err| {
    // error
}

// Pointer capture
var c: anyerror!u32 = 3;
if (c) |*value| {
    value.* = 9;
} else |_| {}
```

### Error Union in while

```zig
var result: anyerror!i32 = 0;
while (result) |val| {
    result = computeNext(val);
} else |err| {
    // handle final error
}
```

## Error Return Traces

Zig maintains error return traces (more detailed than stack traces) that show the full chain of error returns. Enabled by default in Debug/ReleaseSafe.

```zig
// Check if error return tracing is available
const has_trace = @import("builtin").have_error_return_tracing;

// Get current error return trace
const trace = @errorReturnTrace();
```

## Error Casting

```zig
// Cast error to integer
const code: u16 = @intFromError(error.NotFound);

// Cast integer to error
const err = @errorFromInt(@as(u16, 1));

// Cast between error sets
const specific: FileError = @errorCast(general_error);
```

## Error Name

```zig
const name = @errorName(error.NotFound);  // "NotFound"
```

## Error Set Operations

```zig
// Subset check (comptime)
const is_subset = error{NotFound} subsetOf anyerror;

// Merge error sets
const Merged = error{A} || error{B};
```

## Best Practices

1. **Use inferred error sets** — let the compiler figure out the error set
2. **Use specific error sets** when you want to limit what errors a function can return
3. **Use `try` for propagation** — cleaner than manual catch-and-return
4. **Use `errdefer` for cleanup** — free resources on error paths
5. **Avoid `anyerror`** in function signatures when possible — loses type safety
6. **Handle errors explicitly** — Zig has no exceptions, all errors must be handled

## Common Error Handling Patterns

```zig
// Pattern: try with fallback
const data = readFile() catch |err| {
    std.debug.print("Failed: {}\n", .{err});
    return;
};

// Pattern: retry
var attempts: u8 = 0;
while (attempts < 3) : (attempts += 1) {
    if (tryOperation()) |_| break else |err| switch (err) {
        error.Timeout => continue,
        else => return err,
    }
}

// Pattern: map errors
const result = operation() catch |err| switch (err) {
    error.NotFound => error.NotFound,
    error.PermissionDenied => error.PermissionDenied,
    else => error.Unknown,
};
```
