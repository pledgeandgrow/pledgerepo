# Build System & Compilation (Zig)

## Build Modes

| Mode | Flag | Safety | Optimization | Use Case |
|------|------|--------|-------------|----------|
| Debug | (default) | Full | None | Development |
| ReleaseFast | `-O ReleaseFast` | Off | Max | Production, performance |
| ReleaseSafe | `-O ReleaseSafe` | Full | Moderate | Production, safety-critical |
| ReleaseSmall | `-O ReleaseSmall` | Off | Size | Embedded, size-constrained |

```bash
zig build-exe hello.zig                          # Debug mode
zig build-exe hello.zig -O ReleaseFast           # Optimized
zig build-exe hello.zig -O ReleaseSafe           # Optimized + safety
zig build-exe hello.zig -O ReleaseSmall          # Size-optimized
```

## build.zig

The Zig Build System uses a `build.zig` file to declare build logic:

```zig
const std = @import("std");

pub fn build(b: *std.Build) void {
    const optimize = b.standardOptimizeOption(.{});

    const exe = b.addExecutable(.{
        .name = "example",
        .root_module = b.createModule(.{
            .root_source_file = b.path("example.zig"),
            .optimize = optimize,
        }),
    });
    b.default_step.dependOn(&exe.step);

    // Run step
    const run_cmd = b.addRunArtifact(exe);
    const run_step = b.step("run", "Run the app");
    run_step.dependOn(&run_cmd.step);

    // Test step
    const tests = b.addTest(.{
        .root_module = b.createModule(.{
            .root_source_file = b.path("example.zig"),
            .optimize = optimize,
        }),
    });
    const test_step = b.step("test", "Run tests");
    test_step.dependOn(&b.addRunArtifact(tests).step);
}
```

## Build Commands

```bash
zig build              # Run default step
zig build run          # Run the app
zig build test         # Run tests
zig build --help       # Show available steps
zig build -Doptimize=ReleaseFast  # Override options
```

## Compilation Model

### Source File Structs

Each `.zig` file is implicitly a struct. Top-level declarations become struct fields.

### File and Declaration Discovery

The compiler discovers declarations by analyzing the import graph starting from the root source file.

### Special Root Declarations

The root source file can define special declarations:

#### Entry Point

```zig
// Standard main with init
pub fn main(init: std.process.Init) !void {
    try std.Io.File.stdout().writeStreamingAll(init.io, "Hello!\n");
}

// Simple main
pub fn main() void {
    std.debug.print("Hello!\n", .{});
}
```

#### Standard Library Options

```zig
// In root file
pub const std_options: std.Options = .{
    .log_level = .debug,
};
```

#### Panic Handler

```zig
pub fn panic(msg: []const u8, stack_trace: ?*std.builtin.StackTrace) noreturn {
    // Custom panic handler
    @trap();
}
```

## Compile Variables

Import `builtin` for compile-time target information:

```zig
const builtin = @import("builtin");

// OS detection
const separator = if (builtin.os.tag == .windows) '\\' else '/';

// Architecture
const is_64bit = builtin.cpu.arch == .x86_64;

// Build mode
const is_debug = builtin.mode == .Debug;

// Zig version
const version = builtin.zig_version;
```

### Available Compile Variables

| Variable | Type | Description |
|----------|------|-------------|
| `zig_version` | `SemanticVersion` | Zig compiler version |
| `zig_backend` | `CompilerBackend` | Which backend is used |
| `output_mode` | `OutputMode` | `.Exe`, `.Lib`, `.Obj` |
| `link_mode` | `LinkMode` | `.static` or `.dynamic` |
| `is_test` | `bool` | Building in test mode |
| `single_threaded` | `bool` | Single-threaded build |
| `os` | `Target.Os` | OS info (tag, version) |
| `cpu` | `Target.Cpu` | CPU info (arch, model, features) |
| `abi` | `Target.Abi` | ABI info |
| `target` | `Target` | Full target info |
| `object_format` | `ObjectFormat` | `.elf`, `.macho`, `.coff`, `.wasm` |
| `mode` | `OptimizeMode` | Build mode |
| `link_libc` | `bool` | Whether libc is linked |
| `link_libcpp` | `bool` | Whether libc++ is linked |
| `have_error_return_tracing` | `bool` | Error return traces enabled |
| `valgrind_support` | `bool` | Valgrind support enabled |
| `sanitize_thread` | `bool` | Thread sanitizer enabled |
| `fuzz` | `bool` | Fuzzing mode |
| `position_independent_code` | `bool` | PIC enabled |
| `strip_debug_info` | `bool` | Debug info stripped |

## Single Threaded Builds

```bash
zig build-exe hello.zig -fsingle-threaded
```

Reduces binary size and enables certain optimizations.

## Illegal Behavior

In safe modes (Debug, ReleaseSafe), illegal behavior causes a panic. In ReleaseFast, it is undefined behavior.

### Types of Illegal Behavior

- Reaching unreachable code
- Index out of bounds
- Cast negative number to unsigned integer
- Cast truncates data
- Integer overflow
- Division by zero
- Attempt to unwrap null
- Attempt to unwrap error
- Invalid enum cast
- Wrong union field access
- Incorrect pointer alignment

## Cross-Compilation

```bash
zig build-exe hello.zig -target x86_64-linux-gnu
zig build-exe hello.zig -target aarch64-macos
zig build-exe hello.zig -target wasm32-wasi
```

### Available Targets

```bash
zig targets  # List all supported architectures, OSes, ABIs
```
