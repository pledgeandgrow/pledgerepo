# Testing (Zig)

## Test Declarations

```zig
test "basic test" {
    try std.testing.expectEqual(4, 2 + 2);
}

test "with description" {
    const result = myFunction();
    try std.testing.expect(result == 42);
}
```

### Doctests (Identifier-based Tests)

A test name can also be written using an identifier instead of a string. This is a **doctest** and serves as documentation for the function:

```zig
test addOne {
    // This is a doctest for `addOne`
    try std.testing.expectEqual(42, addOne(41));
}

/// The function `addOne` adds one to the number given as its argument.
fn addOne(number: i32) i32 {
    return number + 1;
}
```

Doctests appear in generated documentation and serve as executable examples.

## Running Tests

```bash
zig test file.zig               # Run all tests
zig test file.zig --test-filter "basic"  # Run tests matching filter
```

## Test Failure

Tests fail by returning an error or by hitting an assertion:

```zig
test "failing test" {
    try std.testing.expect(false);  // ❌ will fail
}
```

## Skip Tests

```zig
test "skipped test" {
    return error.SkipZigTest;
}
```

## Report Memory Leaks

Use `std.testing.allocator` to detect memory leaks in tests:

```zig
test "memory leak detection" {
    const allocator = std.testing.allocator;
    const memory = try allocator.alloc(u8, 100);
    // If you forget to free, the test will fail with a leak report
    defer allocator.free(memory);
}
```

## Detecting Test Build

Check at compile time if building in test mode:

```zig
const builtin = @import("builtin");

fn init() void {
    if (builtin.is_test) {
        // Test-specific initialization
    }
}
```

## Test Output and Logging

```zig
const std = @import("std");

test "with logging" {
    std.debug.print("Running test...\n", .{});
    // Test output appears in test runner output
}
```

## Testing Namespace

A special `test` namespace in a source file can define test-only declarations:

```zig
// Regular code
pub fn add(a: i32, b: i32) i32 {
    return a + b;
}

// Test-only namespace
test {
    // This runs as part of the test suite
    try std.testing.expectEqual(5, add(2, 3));
}

// Named tests
test "add works" {
    try std.testing.expectEqual(5, add(2, 3));
}
```

## Testing Utilities

### Expect

```zig
try std.testing.expect(true);
try std.testing.expect(condition);
```

### Expect Equal

```zig
try std.testing.expectEqual(@as(i32, 42), value);
try std.testing.expectEqualSlices(u8, expected, actual);
try std.testing.expectEqualStrings("hello", actual);
try std.testing.expectEqualDeep(expected, actual);  // deep comparison
```

### Expect Error

```zig
try std.testing.expectError(error.NotFound, myFunction());
```

### Expect ApproxEq

```zig
try std.testing.expectApproxEqAbs(@as(f64, 3.14), result, 0.001);
try std.testing.expectApproxEqRel(@as(f64, 3.14), result, 0.01);
```

## Test Tool Documentation

Custom test runners can provide additional tooling:

```zig
// In build.zig
const tests = b.addTest(.{
    .root_module = b.createModule(.{
        .root_source_file = b.path("tests.zig"),
    }),
});
```

## Testing Patterns

### Table-Driven Tests

```zig
test "addition table" {
    const cases = [_]struct { a: i32, b: i32, expected: i32 }{
        .{ .a = 1, .b = 2, .expected = 3 },
        .{ .a = -1, .b = 1, .expected = 0 },
        .{ .a = 0, .b = 0, .expected = 0 },
    };
    for (cases) |case| {
        try std.testing.expectEqual(case.expected, add(case.a, case.b));
    }
}
```

### Setup and Teardown

```zig
test "with setup" {
    const allocator = std.testing.allocator;
    var context = try setupContext(allocator);
    defer cleanupContext(&context, allocator);

    // Test using context
}
```

### Testing with Allocators

```zig
test "allocation test" {
    const allocator = std.testing.allocator;

    var list = std.ArrayList(u8).empty;
    defer list.deinit(allocator);

    try list.appendSlice(allocator, "test");
    try std.testing.expectEqualStrings("test", list.items);
}
```

## refAllDecls

Ensure all declarations are analyzed (no compile errors in unused code):

```zig
test {
    std.testing.refAllDecls(@import("std"));
}
```
