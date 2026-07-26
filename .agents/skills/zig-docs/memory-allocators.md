# Memory & Allocators (Zig)

Zig performs no memory management on behalf of the programmer. No garbage collector, no runtime. Functions that need to allocate accept an `Allocator` parameter.

## Core Principle

> "Where are the bytes?"

Every Zig programmer must always be able to answer this question. Memory is explicitly managed.

## Allocator Interface

```zig
const std = @import("std");
const Allocator = std.mem.Allocator;

// Functions that allocate take an Allocator
fn concat(allocator: Allocator, a: []const u8, b: []const u8) ![]u8 {
    const result = try allocator.alloc(u8, a.len + b.len);
    @memcpy(result[0..a.len], a);
    @memcpy(result[a.len..], b);
    return result;
}
```

## Choosing an Allocator

### FixedBufferAllocator

Stack-allocated buffer. No heap allocation. Fast, deterministic:

```zig
var buffer: [100]u8 = undefined;
var fba = std.heap.FixedBufferAllocator.init(&buffer);
const allocator = fba.allocator();

const result = try concat(allocator, "foo", "bar");
defer allocator.free(result);
```

### DebugAllocator (General Purpose)

General purpose allocator with leak detection in Debug mode:

```zig
var gpa = std.heap.DebugAllocator(.{}).init;
defer _ = gpa.deinit();
const allocator = gpa.allocator();
```

### ArenaAllocator

All allocations freed at once. Fast for batch operations:

```zig
var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
defer arena.deinit();
const allocator = arena.allocator();

// All allocations freed when arena.deinit() is called
```

### page_allocator

OS page allocator. Lowest level, allocates whole pages:

```zig
const allocator = std.heap.page_allocator;
```

### c_allocator

Wraps C's `malloc`/`free`. Requires linking libc:

```zig
const allocator = std.heap.c_allocator;
```

### Testing Allocator

For tests — detects leaks:

```zig
test "my test" {
    const allocator = std.testing.allocator;
    const result = try concat(allocator, "foo", "bar");
    defer allocator.free(result);
    try std.testing.expectEqualStrings("foobar", result);
}
```

## Allocation Patterns

### Allocate and Free

```zig
const memory = try allocator.alloc(u8, 100);
defer allocator.free(memory);
```

### Create and Destroy

```zig
const point = try allocator.create(Point);
defer allocator.destroy(point);
point.* = .{ .x = 1, .y = 2 };
```

### Resize

```zig
const bigger = try allocator.realloc(old_slice, new_len);
```

### toOwnedSlice

Transfer ownership from an ArrayList to a raw slice:

```zig
var list = std.ArrayList(u8).empty;
defer list.deinit(allocator);
try list.appendSlice(allocator, "hello");
const owned = try list.toOwnedSlice(allocator);
// Now `owned` is a regular slice — caller must free it
defer allocator.free(owned);
```

## Lifetime & Ownership

Zig has no RAII. Memory must be explicitly freed. Use `defer` and `errdefer` for cleanup:

```zig
fn createThing(allocator: Allocator) !*Thing {
    const ptr = try allocator.create(Thing);
    errdefer allocator.destroy(ptr);

    // If this fails, ptr is freed via errdefer
    ptr.* = try initializeThing();

    return ptr;  // Ownership transferred to caller
}
```

### Ownership Conventions

1. **Caller frees** — if a function returns allocated memory, the caller is responsible for freeing it
2. **`defer` for cleanup** — always pair allocation with a `defer` free
3. **`errdefer` for error paths** — free on error, keep on success
4. **Document ownership** — use doc comments to clarify who owns what

## Recursion

Deep recursion can overflow the stack. For deep recursion, use an explicit stack (heap-allocated):

```zig
// Instead of recursive function, use iterative approach with a stack
var stack = std.ArrayList(Node).empty;
defer stack.deinit(allocator);
```

## Heap Allocation Failure

Allocation can fail. Always handle the error:

```zig
const memory = allocator.alloc(u8, 100) catch {
    // Handle allocation failure
    return error.OutOfMemory;
};
```

## Memory Safety

- **No use-after-free** — use `defer` to ensure cleanup
- **No double-free** — `free` on already-freed memory is illegal behavior
- **No leaks** — use `std.testing.allocator` in tests to detect leaks
- **Bounds checking** — slice indexing is bounds-checked in safe modes

## Memory Operations

### @memcpy
```zig
@memcpy(dest, source)  // dest and source must be same length
```

### @memmove
```zig
@memmove(dest, source)  // handles overlapping regions
```

### @memset
```zig
@memset(dest, value)  // fill with value
```

## std.mem Module

```zig
const mem = std.mem;

// Compare
mem.eql(u8, a, b)           // equality
mem.lessThan(u8, a, b)      // comparison

// Search
mem.indexOf(u8, haystack, needle)
mem.indexOfScalar(u8, slice, value)
mem.lastIndexOf(u8, slice, needle)

// Transform
mem.trim(u8, slice, cutset)
mem.tokenize(u8, slice, delimiters)
mem.split(u8, slice, delimiter)

// Copy
mem.copyForwards(u8, dest, source)
mem.copyBackwards(u8, dest, source)
```
