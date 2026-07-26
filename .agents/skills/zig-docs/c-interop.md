# C Interop (Zig)

Zig acknowledges the importance of interacting with existing C code. No FFI, no bindings — direct interop.

## C Type Primitives

| Zig Type | C Type |
|----------|--------|
| `c_char` | `char` |
| `c_short` | `short` |
| `c_ushort` | `unsigned short` |
| `c_int` | `int` |
| `c_uint` | `unsigned int` |
| `c_long` | `long` |
| `c_ulong` | `unsigned long` |
| `c_longlong` | `long long` |
| `c_ulonglong` | `unsigned long long` |
| `c_longdouble` | `long double` |

## Import from C Header File

`@cImport` imports C headers and makes C declarations available in Zig:

```zig
const c = @cImport({
    @cInclude("stdio.h");
    @cInclude("stdlib.h");
});

// Use C functions
pub fn main() void {
    _ = c.printf("Hello from C!\n");
}
```

### @cInclude

```zig
@cInclude("header.h")  // Include a C header
```

### @cDefine

```zig
@cDefine("_GNU_SOURCE", "1")  // Define a C macro
```

### @cUndef

```zig
@cUndef("UNWANTED_MACRO")  // Undefine a C macro
```

## C Translation CLI

Translate C source to Zig:

```bash
zig translate-c input.c -o output.zig
```

### Command Line Flags

```bash
zig translate-c input.c \
  -target x86_64-linux-gnu \
  -I/include/path \
  -DDEFINE=value \
  -o output.zig
```

### @cImport vs translate-c

| Feature | `@cImport` | `translate-c` |
|---------|-----------|---------------|
| When | Compile time | Before compilation |
| Output | Implicit | Explicit `.zig` file |
| Debugging | Harder | Easier (visible output) |
| Caching | Automatic | Manual |

### C Translation Caching

`@cImport` results are cached. The cache is invalidated when headers change.

### Translation Failures

Some C constructs cannot be automatically translated. The compiler will emit errors with suggestions.

## C Macros

C macros are translated to Zig functions or constants:

```c
// C
#define MAX(a, b) ((a) > (b) ? (a) : (b))
```

```zig
// Zig (auto-translated)
fn MAX(a: anytype, b: anytype) @TypeOf(a, b) {
    return if (a > b) a else b;
}
```

## C Pointers

Zig provides `[*c]T` for C pointer interop — can be null, supports pointer arithmetic without bounds checking:

```zig
extern fn malloc(size: usize) [*c]u8;

const ptr = malloc(100);
if (ptr == null) {
    // handle null
}
```

**Note:** Prefer `?*T` or `?[*]T` over `[*c]T` in Zig code. `[*c]T` is mainly for C interop.

## C Variadic Functions

```zig
extern fn printf(format: [*:0]const u8, ...) c_int;

pub fn main() void {
    _ = printf("Hello, %s!\n", "World");
}
```

## Exporting a C Library

```zig
// Export functions with C ABI
export fn add(a: i32, b: i32) i32 {
    return a + b;
}

// With explicit calling convention
export fn process(data: [*]const u8, len: usize) callconv(.c) i32 {
    // ...
}
```

### Building a C-compatible Library

```bash
zig build-lib mylib.zig -dynamic -O ReleaseFast
```

### Using from C

```c
// C code
#include <stdint.h>

extern int32_t add(int32_t a, int32_t b);

int main() {
    return add(1, 2);
}
```

## Mixing Object Files

Zig can compile C/C++ source files alongside Zig code:

```bash
# Compile C file
zig cc -c example.c -o example.o

# Link with Zig
zig build-exe main.zig example.o
```

### In build.zig

```zig
const exe = b.addExecutable(.{
    .name = "app",
    .root_module = b.createModule(.{
        .root_source_file = b.path("main.zig"),
        .optimize = optimize,
    }),
});
exe.addCSourceFile(.{
    .file = b.path("example.c"),
    .flags = &.{"-std=c11"},
});
exe.linkLibC();
```

## Linking libc

```bash
zig build-exe hello.zig -lc
```

## extern struct

For C ABI-compatible struct layout:

```zig
const CPoint = extern struct {
    x: c_int,
    y: c_int,
};
```

## extern union

```zig
const CUnion = extern union {
    as_int: c_int,
    as_float: c_float,
};
```

## extern enum

```zig
const CEnum = extern enum(c_int) {
    option_a,
    option_b,
};
```

## @export

Export declarations with custom names:

```zig
const internal_name = 42;
@export(internal_name, .{ .name = "public_name" });
```
