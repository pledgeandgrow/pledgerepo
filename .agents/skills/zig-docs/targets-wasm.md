# Targets & WebAssembly (Zig)

## Targets

A target is the computer that will run the executable. It includes:
- CPU architecture
- Enabled CPU features
- Operating system
- OS version range
- ABI
- ABI version

## Listing Targets

```bash
zig targets
```

Shows all supported architectures, OSes, ABIs, and object formats.

## Cross-Compilation

Zig has first-class cross-compilation. No additional toolchain needed:

```bash
# Linux x86_64
zig build-exe hello.zig -target x86_64-linux-gnu

# macOS ARM64
zig build-exe hello.zig -target aarch64-macos

# Windows x86_64
zig build-exe hello.zig -target x86_64-windows-msvc

# WebAssembly
zig build-exe hello.zig -target wasm32-wasi

# Bare metal (no OS)
zig build-exe hello.zig -target riscv64-linux-none
```

## Target Triple Format

```
<arch><sub>-<os>-<abi>
```

### Common Architectures

| Architecture | Target String |
|-------------|---------------|
| x86_64 | `x86_64` |
| ARM 64-bit | `aarch64` |
| ARM 32-bit | `arm` |
| RISC-V 64-bit | `riscv64` |
| RISC-V 32-bit | `riscv32` |
| WebAssembly 32-bit | `wasm32` |
| WebAssembly 64-bit | `wasm64` |

### Common OSes

| OS | Target String |
|----|---------------|
| Linux | `linux` |
| macOS | `macos` |
| Windows | `windows` |
| FreeBSD | `freebsd` |
| NetBSD | `netbsd` |
| WASI | `wasi` |
| None (bare metal) | `none` |

### Common ABIs

| ABI | Target String |
|-----|---------------|
| GNU | `gnu` |
| musl | `musl` |
| MSVC | `msvc` |
| Android | `android` |
| None | `none` |

## CPU Features

Enable/disable specific CPU features:

```bash
zig build-exe hello.zig -target x86_64-linux-gnu -mcpu=znver4
zig build-exe hello.zig -target x86_64-linux-gnu -mcpu=baseline
zig build-exe hello.zig -target x86_64-linux-gnu -mcpu=native
```

### In build.zig

```zig
const exe = b.addExecutable(.{
    .name = "app",
    .root_module = b.createModule(.{
        .root_source_file = b.path("main.zig"),
        .target = b.standardTargetOptions(.{}),
        .optimize = optimize,
    }),
});
```

## Runtime Target Detection

```zig
const builtin = @import("builtin");

// Check architecture
if (builtin.cpu.arch == .x86_64) { ... }

// Check OS
if (builtin.os.tag == .linux) { ... }

// Check ABI
if (builtin.abi == .gnu) { ... }

// Check features
if (builtin.cpu.arch == .x86_64 and std.Target.x86.featureSetHas(builtin.cpu.features, .avx2)) {
    // Use AVX2 optimized path
}
```

## WebAssembly

### Freestanding (no WASI)

For raw WebAssembly without an OS abstraction:

```bash
zig build-exe hello.zig -target wasm32-freestanding
```

### WASI (WebAssembly System Interface)

Full OS-like environment with file access, environment variables:

```bash
zig build-exe hello.zig -target wasm32-wasi
```

Run with a WASI runtime:

```bash
wasmtime hello.wasm
wasmer run hello.wasm
```

### Exporting Functions to JavaScript

```zig
export fn add(a: i32, b: i32) i32 {
    return a + b;
}
```

```javascript
// JavaScript
const wasm = await WebAssembly.instantiate(wasmBytes, {});
const result = wasm.instance.exports.add(1, 2);  // 3
```

### WebAssembly Limitations

- No threads (unless using `--target=wasm32-wasi-threads`)
- No inline assembly
- Limited floating point support (f16, f80, f128 may not be available)
- `@asyncCall` not supported

## Object Formats

| Format | Target | Extension |
|--------|--------|-----------|
| ELF | Linux, FreeBSD | `.o`, `.so` |
| Mach-O | macOS | `.o`, `.dylib` |
| COFF | Windows | `.obj`, `.dll` |
| WASM | WebAssembly | `.wasm` |
| SPIR-V | GPU shaders | `.spv` |

## Dynamic Linking

```bash
zig build-exe hello.zig -target x86_64-linux-gnu -dynamic
zig build-lib mylib.zig -target x86_64-linux-gnu -dynamic
```
