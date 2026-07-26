# Assembly (Zig)

Zig supports inline assembly for direct control over generated machine code.

## Inline Assembly

```zig
const syscall3 = asm volatile ("syscall"
    : [ret] "={rax}" (-> usize),
    : [number] "{rax}" (number),
      [arg1] "{rdi}" (arg1),
      [arg2] "{rsi}" (arg2),
      [arg3] "{rdx}" (arg3),
    : .{ .rcx = true, .r11 = true }
);
```

### Syntax Breakdown

```zig
asm volatile (assembly_string
    : outputs
    : inputs
    : clobbers
)
```

- **`asm`** — keyword begins the expression
- **`volatile`** — optional modifier; tells Zig the assembly has side-effects. Without `volatile`, Zig may delete the assembly if the result is unused
- **assembly_string** — comptime string containing assembly code. Use `\\` multiline string syntax
- **outputs** — output constraints (can be empty)
- **inputs** — input constraints (can be empty)
- **clobbers** — registers that will not be preserved

### Full Example: Hello World on x86_64 Linux

```zig
pub fn main() noreturn {
    const msg = "hello world\n";
    _ = syscall3(SYS_write, STDOUT_FILENO, @intFromPtr(msg), msg.len);
    _ = syscall1(SYS_exit, 0);
    unreachable;
}

pub const SYS_write = 1;
pub const SYS_exit = 60;
pub const STDOUT_FILENO = 1;

pub fn syscall1(number: usize, arg1: usize) usize {
    return asm volatile ("syscall"
        : [ret] "={rax}" (-> usize),
        : [number] "{rax}" (number),
          [arg1] "{rdi}" (arg1),
        : .{ .rcx = true, .r11 = true });
}

pub fn syscall3(number: usize, arg1: usize, arg2: usize, arg3: usize) usize {
    return asm volatile ("syscall"
        : [ret] "={rax}" (-> usize),
        : [number] "{rax}" (number),
          [arg1] "{rdi}" (arg1),
          [arg2] "{rsi}" (arg2),
          [arg3] "{rdx}" (arg3),
        : .{ .rcx = true, .r11 = true });
}
```

## Output Constraints

Output constraints are still considered unstable. Refer to:
- [LLVM documentation](http://releases.llvm.org/10.0.0/docs/LangRef.html#inline-asm-constraint-string)
- [GCC documentation](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html)

Syntax:
```
[name] "constraint" (-> Type)    // return value
[name] "constraint" (value)      // value binding
```

Example: `"={rax}"` means "the result value is whatever is in `$rax`"

## Input Constraints

Same syntax as output constraints. Example: `"{rax}" (number)` means "when the assembly code is executed, `$rax` shall have the value of `number`"

## Clobbers

Clobbers declare registers whose values will not be preserved. These do **not** include output or input registers.

```zig
: .{ .rcx = true, .r11 = true }
```

The special clobber value `"memory"` means the assembly writes to arbitrary undeclared memory locations.

**Failure to declare the full set of clobbers is unchecked Illegal Behavior.**

## Global Assembly

Assembly in a container-level `comptime` block is global assembly. Different rules:
- `volatile` is not valid (all global assembly is unconditionally included)
- No inputs, outputs, or clobbers
- All global assembly is concatenated verbatim and assembled together
- No template substitution rules regarding `%`

```zig
comptime {
    asm (
        \\.global my_func;
        \\.type my_func, @function;
        \\my_func:
        \\ lea (%rdi,%rsi,1),%eax
        \\ retq
    );
}

extern fn my_func(a: i32, b: i32) i32;

test "global assembly" {
    try std.testing.expectEqual(46, my_func(12, 34));
}
```

## AT&T Syntax

For x86 and x86_64 targets, Zig uses AT&T syntax (not Intel syntax). This is due to LLVM constraints. A literal `%` is obtained with `%%`.

## Assembly Limitations

- WebAssembly targets do not support inline assembly
- Syntax depends on target architecture
- Some constraints are still considered unstable
