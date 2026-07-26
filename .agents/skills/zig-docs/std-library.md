# Standard Library (Zig)

The Zig Standard Library (`@import("std")`) contains commonly used algorithms, data structures, and definitions.

## Accessing the Documentation

```bash
zig std  # Start local std docs web server
```

## Module Overview

### Data Structures

| Module | Types | Description |
|--------|-------|-------------|
| `std.array_list` | `ArrayList(T)`, `Aligned(T)` | Growable contiguous list |
| `std.hash_map` | `HashMap`, `AutoHashMap`, `StringHashMap` | Hash maps |
| `std.array_hash_map` | `ArrayHashMap`, `AutoArrayHashMap`, `StringArrayHashMap` | Ordered hash maps |
| `std.bit_set` | `DynamicBitSet`, `StaticBitSet` | Bit sets |
| `std.SinglyLinkedList` | `SinglyLinkedList` | Singly linked list |
| `std.DoublyLinkedList` | `DoublyLinkedList` | Doubly linked list |
| `std.PriorityQueue` | `PriorityQueue` | Priority queue |
| `std.PriorityDequeue` | `PriorityDequeue` | Priority dequeue |
| `std.Deque` | `Deque` | Double-ended queue |
| `std.BitStack` | `BitStack` | Bit stack |
| `std.MultiArrayList` | `MultiArrayList` | Structure-of-arrays list |
| `std.Treap` | `Treap` | Balanced binary search tree |
| `std.StaticStringMap` | `StaticStringMap` | Comptime string lookup map |
| `std.EnumArray` | `EnumArray` | Array indexed by enum |
| `std.EnumMap` | `EnumMap` | Map keyed by enum |
| `std.EnumSet` | `EnumSet` | Set of enum values |

### ArrayList

```zig
var list = std.ArrayList(u8).empty;
defer list.deinit(allocator);

try list.appendSlice(allocator, "hello");
try list.append(allocator, '!');
// list.items is []u8
```

### HashMap

```zig
var map = std.AutoHashMap(u32, []const u8).init(allocator);
defer map.deinit();

try map.put(1, "one");
if (map.get(1)) |value| {
    // value is "one"
}
```

### StringHashMap

```zig
var map = std.StringHashMap(u32).init(allocator);
defer map.deinit();

try map.put("hello", 42);
```

### StaticStringMap

Comptime-constructed string map for fast lookup:

```zig
const colors = std.StaticStringMap(u32).initComptime(.{
    .{ "red", 0xff0000 },
    .{ "green", 0x00ff00 },
    .{ "blue", 0x0000ff },
});

const color = colors.get("red").?;  // 0xff0000
```

### MultiArrayList

Stores each field in a separate array (structure-of-arrays). Better cache locality for field access:

```zig
var list = std.MultiArrayList(Item){};
defer list.deinit(allocator);
```

## Core Modules

### std.debug

```zig
std.debug.print("format: {d}\n", .{42});  // Print to stderr
std.debug.assert(condition);               // Assert (panics in debug)
std.debug.panic("message: {d}", .{x});    // Panic with message
```

### std.fmt

Formatted output and parsing:

```zig
// Format specifiers
// {d}  — decimal
// {x}  — hex
// {s}  — string
// {c}  — char
// {e}  — scientific notation
// {?}  — optional
// {!}  — error union
// {any} — any type

std.debug.print("{d} {x} {s}\n", .{ 255, 0xff, "hello" });

// Custom formatting
const buf = try std.fmt.bufPrint(&buffer, "x={d}", .{x});
```

### std.mem

Memory utilities:

```zig
const mem = std.mem;

mem.eql(u8, a, b)                    // String/slice equality
mem.indexOf(u8, haystack, needle)    // Search
mem.lastIndexOf(u8, slice, needle)   // Search from end
mem.trim(u8, slice, cutset)          // Trim characters
mem.tokenize(u8, slice, delims)      // Tokenize iterator
mem.split(u8, slice, delim)          // Split iterator
mem.copyForwards(u8, dest, src)      // Copy memory
mem.copyBackwards(u8, dest, src)     // Copy backwards
mem.set(u8, dest, value)             // Fill memory
mem.reverse(u8, slice)               // Reverse in place
```

### std.fs

File system operations:

```zig
const fs = std.fs;

// Read entire file
const content = try fs.cwd().readFileAlloc(allocator, "file.txt", max_size);
defer allocator.free(content);

// Write file
var file = try fs.cwd().createFile("output.txt", .{});
defer file.close();
try file.writeAll("Hello!\n");

// Directory listing
var dir = try fs.cwd().openDir(".", .{ .iterate = true });
defer dir.close();
var iter = dir.iterate();
while (try iter.next()) |entry| {
    // entry.name, entry.kind
}
```

### std.io / std.Io

I/O operations:

```zig
// stdout
const stdout = std.Io.File.stdout();
try stdout.writeAll(init.io, "Hello!\n");

// stderr
const stderr = std.Io.File.stderr();
```

### std.math

```zig
const math = std.math;

math.max(T, a, b)       // Maximum
math.min(T, a, b)       // Minimum
math.abs(x)             // Absolute value
math.sqrt(x)            // Square root
math.pow(T, base, exp)  // Power
math.sin(x)             // Sine
math.cos(x)             // Cosine
math.log(T, x)          // Natural log
math.isNan(x)           // Check NaN
math.isInf(x)           // Check infinity
math.clamp(x, min, max) // Clamp value
```

### std.os

OS-specific operations (prefer `std.fs` and `std.process` for portability):

```zig
const os = std.os;
// Low-level OS syscalls
```

### std.posix

POSIX operations:

```zig
const posix = std.posix;
// POSIX functions
```

### std.process

Process and command-line:

```zig
const process = std.process;

// Get args
const args = try process.argsAlloc(allocator);
defer process.argsFree(allocator, args);

// Environment variables
const env = process.getEnvVarOwned(allocator, "PATH");
```

### std.thread / std.Thread

```zig
const thread = try std.Thread.spawn(.{}, workerFunction, .{ arg });
thread.join();
```

### std.time

```zig
const time = std.time;

const start = time.milliTimestamp();
// ... do work ...
const elapsed = time.milliTimestamp() - start;

// Sleep
std.Thread.sleep(1_000_000_000);  // 1 second in nanoseconds
```

### std.crypto

Cryptographic primitives:

```zig
const crypto = std.crypto;

// Hashing
const hash = crypto.hash.sha256.Hash;
const digest = hash.hash(data, .{});

// Encryption
const Aes256 = crypto.aead.aes_gcm.Aes256Gcm;

// Random
const random = crypto.random;
const value = random.int(u32);
```

### std.json

JSON parsing and serialization:

```zig
const json = std.json;

// Parse
const parsed = try json.parseFromSlice(MyStruct, allocator, json_string, .{});
defer parsed.deinit();
// parsed.value is MyStruct

// Stringify
try json.stringify(my_struct, .{}, writer);
```

### std.zon

Zig Object Notation (Zig's native data format):

```zig
const zon = std.zon;
// Parse ZON files
```

### std.compress

Compression algorithms:

```zig
const compress = std.compress;
// gzip, zlib, zstd, etc.
```

### std.http

HTTP client and server:

```zig
const http = std.http;
// HTTP client for making requests
// HTTP server for handling requests
```

### std.Uri

URI parsing:

```zig
const uri = std.Uri.parse("https://example.com/path?q=1");
```

### std.base64

Base64 encoding/decoding:

```zig
const encoded = try std.base64.standard.Encoder.encode(dest, source);
const decoded = try std.base64.standard.Decoder.decode(dest, source);
```

### std.sort

Sorting algorithms:

```zig
std.sort.pdq(T, slice, {}, lessThanFn);
std.sort.block(T, slice, {}, lessThanFn);
std.sort.insertion(T, slice, {}, lessThanFn);
```

### std.atomic

Atomic operations:

```zig
var counter = std.atomic.Value(u32).init(0);
_ = counter.fetchAdd(1, .seq_cst);
```

### std.enums

Enum utilities:

```zig
std.enums.EnumArray(MyEnum, T)  // Array indexed by enum
std.enums.EnumMap(MyEnum, T)    // Map keyed by enum
std.enums.EnumSet(MyEnum)       // Set of enum values
```

### std.meta

Metaprogramming utilities:

```zig
std.meta.Tag(TaggedUnion)  // Get tag enum type
std.meta.eql(a, b)         // Deep equality
std.meta.deepEqual(a, b)   // Deep equality
```

### std.Target

Target information for cross-compilation:

```zig
const Target = std.Target;
// CPU architectures, OS tags, ABIs
```

### std.Build

Build system API (used in `build.zig`):

```zig
const Build = std.Build;
// Executable, library, test, module configuration
```

### std.SemanticVersion

```zig
const version = std.SemanticVersion.parse("0.16.0") catch unreachable;
```

### std.Progress

Progress bar for terminal output:

```zig
var progress = std.Progress{};
const node = progress.start("Task", total_items);
// ... work ...
node.end();
```

### std.Random

```zig
var prng = std.Random.DefaultPrng.init(seed);
const random = prng.random();
const value = random.int(u32);
const float = random.float(f32);
```

### std.ascii

```zig
std.ascii.isAlphabetic(c)
std.ascii.isDigit(c)
std.ascii.isWhitespace(c)
std.ascii.toUpper(c)
std.ascii.toLower(c)
```

### std.unicode

Unicode utilities:

```zig
std.unicode.utf8Decode(slice)
std.unicode.utf8Encode(codepoint, dest)
```

## Standard Library Options

Override defaults in root file:

```zig
pub const std_options: std.Options = .{
    .log_level = .debug,
    .enable_segfault_handler = true,
    .crypto_fork_safety = true,
    .http_disable_tls = false,
    .fmt_max_depth = 10,
};
```
