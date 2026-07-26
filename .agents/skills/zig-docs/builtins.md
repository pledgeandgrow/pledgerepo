# Builtin Functions (Zig)

Builtin functions are provided by the compiler, prefixed with `@`. The `comptime` keyword on a parameter means it must be known at compile time.

## Memory & Pointers

### @as
```zig
@as(comptime T: type, expression) T
```
Type Coercion — preferred way to convert between types.

### @bitCast
```zig
@bitCast(value) DestType
```
Reinterpret bits of one type as another. Types must have same bit size.

### @ptrCast
```zig
@ptrCast(value) DestType
```
Pointer reinterpret cast. No safety checks — use carefully.

### @constCast
```zig
@constCast(value) DestType
```
Remove `const` from a pointer.

### @volatileCast
```zig
@volatileCast(value) DestType
```
Add/remove `volatile` from a pointer.

### @addrSpaceCast
```zig
@addrSpaceCast(ptr: anytype) anytype
```
Convert pointer between address spaces.

### @alignCast
```zig
@alignCast(ptr: anytype) anytype
```
Change pointer alignment. Adds runtime safety check.

### @ptrFromInt
```zig
@ptrFromInt(addr: usize) DestType
```
Convert integer to pointer.

### @intFromPtr
```zig
@intFromPtr(ptr: anytype) usize
```
Convert pointer to integer.

## Integer Operations

### @intCast
```zig
@intCast(value) DestIntType
```
Runtime-checked integer conversion.

### @truncate
```zig
@truncate(value) DestIntType
```
Truncate integer (drop high bits, no safety check).

### @enumFromInt
```zig
@enumFromInt(value) DestEnumType
```
Convert integer to enum.

### @intFromEnum
```zig
@intFromEnum(enum_value) IntType
```
Convert enum to integer.

### @intFromBool
```zig
@intFromBool(value) u1
```
Convert boolean to `u1` (0 or 1).

### @intFromFloat
```zig
@intFromFloat(value) DestIntType
```
Convert float to integer (runtime-checked).

### @floatFromInt
```zig
@floatFromInt(value) DestFloatType
```
Convert integer to float.

### @floatCast
```zig
@floatCast(value) DestFloatType
```
Convert between float types.

## Overflow Arithmetic

### @addWithOverflow
```zig
@addWithOverflow(a, b) struct { @TypeOf(a, b), u1 }
```
Returns result and overflow bit.

### @subWithOverflow
```zig
@subWithOverflow(a, b) struct { @TypeOf(a, b), u1 }
```

### @mulWithOverflow
```zig
@mulWithOverflow(a, b) struct { @TypeOf(a, b), u1 }
```

### @shlWithOverflow
```zig
@shlWithOverflow(a, b) struct { @TypeOf(a, b), u1 }
```
Left shift with overflow check.

## Math Functions

### @min / @max
```zig
@min(a, b) T
@max(a, b) T
```
Minimum/maximum of two values (any number of args).

### @divTrunc / @divFloor / @divExact
```zig
@divTrunc(a, b) T  // truncated toward zero
@divFloor(a, b) T  // floored toward negative infinity
@divExact(a, b) T  // asserts exact division
```

### @rem / @mod
```zig
@rem(a, b) T   // remainder (truncated)
@mod(a, b) T   // modulo (floored)
```

### @abs
```zig
@abs(value) T
```
Absolute value.

### @sqrt / @sin / @cos / @tan
```zig
@sqrt(value) T
@sin(value) T
@cos(value) T
@tan(value) T
```
Floating point math.

### @exp / @exp2 / @log / @log2 / @log10
```zig
@exp(value) T
@exp2(value) T
@log(value) T
@log2(value) T
@log10(value) T
```

### @floor / @ceil / @trunc / @round
```zig
@floor(value) T
@ceil(value) T
@trunc(value) T
@round(value) T
```

### @mulAdd
```zig
@mulAdd(a, b, c) T  // a * b + c (fused multiply-add)
```

## Bit Operations

### @byteSwap
```zig
@byteSwap(value) T
```
Reverse byte order (endianness).

### @bitReverse
```zig
@bitReverse(value) T
```
Reverse bit order.

### @popCount
```zig
@popCount(value) T
```
Count number of set bits.

### @clz / @ctz
```zig
@clz(value) T  // count leading zeros
@ctz(value) T  // count trailing zeros
```

### @shlExact / @shrExact
```zig
@shlExact(value, shift) T  // left shift (asserts no bits lost)
@shrExact(value, shift) T  // right shift (asserts no bits lost)
```

## Type Information

### @TypeOf
```zig
@TypeOf(...) type
```
Get type of expression(s). Multiple args → peer type resolution.

### @typeInfo
```zig
@typeInfo(T) std.builtin.Type
```
Full type information as a tagged union.

### @typeName
```zig
@typeName(T) [:0]const u8
```
String representation of type name.

### @sizeOf
```zig
@sizeOf(comptime T: type) comptime_int
```
Size in bytes.

### @alignOf
```zig
@alignOf(comptime T: type) comptime_int
```
Required alignment in bytes.

### @bitSizeOf
```zig
@bitSizeOf(comptime T: type) comptime_int
```
Size in bits.

### @bitOffsetOf
```zig
@bitOffsetOf(comptime T: type, comptime field: []const u8) comptime_int
```
Bit offset of a struct field.

### @offsetOf
```zig
@offsetOf(comptime T: type, comptime field: []const u8) comptime_int
```
Byte offset of a struct field.

### @FieldType
```zig
@FieldType(comptime T: type, comptime field: []const u8) type
```
Get the type of a struct field.

## Type Construction

### @Type
```zig
@Type(comptime info: std.builtin.Type) type
```
Construct a type from type info.

### @Vector
```zig
@Vector(comptime len: u32, comptime child: type) type
```
Create a vector type.

### @Tuple
```zig
@Tuple(comptime types: []const type) type
```
Create a tuple type from a list of types.

### @Enum
```zig
@Enum(comptime tag_type: type, comptime fields: []) type
```
Create an enum type.

### @Int
```zig
@Int(comptime signedness: Signedness, comptime bits: u32) type
```
Create an integer type.

### @Pointer
```zig
@Pointer(comptime info: std.builtin.Type.Pointer) type
```
Create a pointer type.

### @Struct
```zig
@Struct(comptime fields: []const std.builtin.Type.StructField) type
```
Create a struct type.

### @Union
```zig
@Union(comptime info: std.builtin.Type.Union) type
```
Create a union type.

### @Fn
```zig
@Fn(comptime info: std.builtin.Type.Fn) type
```
Create a function type.

### @EnumLiteral
```zig
@EnumLiteral
```
The type of enum literals (`.foo`).

## Reflection & Introspection

### @hasDecl
```zig
@hasDecl(comptime T: type, comptime name: []const u8) bool
```
Check if a container has a declaration.

### @hasField
```zig
@hasField(comptime T: type, comptime name: []const u8) bool
```
Check if a type has a field.

### @field
```zig
@field(obj, comptime name: []const u8) FieldType
```
Access a field by name.

### @fieldParentPtr
```zig
@fieldParentPtr(comptime field: []const u8, field_ptr: *T) *Parent
```
Get parent struct pointer from a field pointer.

### @tagName
```zig
@tagName(value) [:0]const u8
```
String name of an enum value or union tag.

### @unionInit
```zig
@unionInit(comptime U: type, comptime name: []const u8, value) U
```
Initialize a union by comptime-known tag name.

## Memory Operations

### @memcpy
```zig
@memcpy(dest, source) void
```
Copy memory (slices must be same length).

### @memmove
```zig
@memmove(dest, source) void
```
Copy memory with potential overlap.

### @memset
```zig
@memset(dest, value) void
```
Fill memory with a value.

## Atomics

### @atomicLoad
```zig
@atomicLoad(comptime T: type, ptr: *const T, comptime ordering: AtomicOrder) T
```

### @atomicStore
```zig
@atomicStore(comptime T: type, ptr: *T, value: T, comptime ordering: AtomicOrder) void
```

### @atomicRmw
```zig
@atomicRmw(comptime T: type, ptr: *T, comptime op: AtomicRmwOp, operand: T, comptime ordering: AtomicOrder) T
```

### @cmpxchgStrong / @cmpxchgWeak
```zig
@cmpxchgStrong(comptime T: type, ptr: *T, expected_value: T, new_value: T, success_order: AtomicOrder, fail_order: AtomicOrder) ?T
@cmpxchgWeak(comptime T: type, ptr: *T, expected_value: T, new_value: T, success_order: AtomicOrder, fail_order: AtomicOrder) ?T
```

## Compile-Time Control

### @import
```zig
@import(comptime path: []const u8) type
```
Import a Zig source file.

### @embedFile
```zig
@embedFile(comptime path: []const u8) *const [N:0]u8
```
Embed a file as a comptime string.

### @compileError
```zig
@compileError(comptime msg: []const u8) noreturn
```
Trigger a compile error.

### @compileLog
```zig
@compileLog(args: anytype) void
```
Print values at compile time (for debugging).

### @setEvalBranchQuota
```zig
@setEvalBranchQuota(comptime new_quota: usize) void
```
Increase comptime evaluation branch limit.

### @setRuntimeSafety
```zig
@setRuntimeSafety(comptime safety_on: bool) void
```
Enable/disable runtime safety checks for a scope.

### @setFloatMode
```zig
@setFloatMode(comptime mode: FloatMode) void
```
Set floating point mode (`.strict` or `.optimized`).

### @inComptime
```zig
@inComptime() bool
```
Check if in a comptime context.

## Other Builtins

### @panic
```zig
@panic(comptime message: []const u8) noreturn
```
Trigger a panic with a message.

### @trap
```zig
@trap() noreturn
```
Unconditional trap (like `int 3` on x86).

### @breakpoint
```zig
@breakpoint() void
```
Insert a debugger breakpoint.

### @returnAddress
```zig
@returnAddress() usize
```
Address of the call instruction in the caller.

### @frameAddress
```zig
@frameAddress() usize
```
Address of the current stack frame.

### @src
```zig
@src() std.builtin.SourceLocation
```
Source location of the call site.

### @call
```zig
@call(comptime options: CallOptions, function, args) ReturnType
```
Control function calling behavior.

### @This
```zig
@This() type
```
Get the innermost struct/union/enum type.

### @export
```zig
@export(declaration, comptime options: ExportOptions) void
```
Export a declaration.

### @extern
```zig
@extern(T: type, comptime options: ExternOptions) T
```
Declare an external symbol.

### @select
```zig
@select(comptime T: type, pred: @Vector(n, bool), a: @Vector(n, T), b: @Vector(n, T)) @Vector(n, T)
```
Vector select operation.

### @splat
```zig
@splat(comptime len: u32, value: anytype) @Vector(len, T)
```
Create a vector with all elements set to value.

### @reduce
```zig
@reduce(comptime op: ReduceOp, value: @Vector(n, T)) T
```
Reduce a vector to a scalar.

### @shuffle
```zig
@shuffle(comptime T: type, a: @Vector(n, T), b: @Vector(n, T), comptime mask: @Vector(m, i32)) @Vector(m, T)
```
Vector shuffle operation.

### @prefetch
```zig
@prefetch(ptr: anytype, comptime options: PrefetchOptions) void
```
Prefetch memory into cache.

### @branchHint
```zig
@branchHint(comptime hint: BranchHint) void
```
Hint the optimizer about branch likelihood (`.cold`, `.hot`, etc.).

### @cImport / @cInclude / @cDefine / @cUndef
```zig
@cImport(expression) type  // Import C headers
@cInclude(comptime header: []const u8) void
@cDefine(comptime name: []const u8, value) void
@cUndef(comptime name: []const u8) void
```

### @errorName / @errorReturnTrace / @errorFromInt / @errorCast
```zig
@errorName(value) [:0]const u8
@errorReturnTrace() ?*std.builtin.StackTrace
@errorFromInt(value: u16) anyerror
@errorCast(value) DestErrorSet
```

### @addrSpaceCast
```zig
@addrSpaceCast(ptr: anytype) anytype
```
Convert pointer between address spaces.

### @cVaArg / @cVaCopy / @cVaEnd / @cVaStart
```zig
@cVaArg(operand: *std.builtin.VaList, comptime T: type) T  // C va_arg
@cVaCopy(src: *std.builtin.VaList) std.builtin.VaList      // C va_copy
@cVaEnd(src: *std.builtin.VaList) void                      // C va_end
@cVaStart() std.builtin.VaList                              // C va_start (variadic only)
```

### @wasmMemorySize / @wasmMemoryGrow
```zig
@wasmMemorySize(index: u32) usize    // Wasm memory size in pages (64KB each)
@wasmMemoryGrow(index: u32, delta: usize) isize  // Grow Wasm memory, returns old size or -1
```

### @workGroupId / @workGroupSize / @workItemId
```zig
@workGroupId(comptime dimension: u32) u32   // Work group index in dimension
@workGroupSize(comptime dimension: u32) u32  // Work group size in dimension
@workItemId(comptime dimension: u32) u32     // Work item index (0 to @workGroupSize-1)
```

### @min / @max
```zig
@min(...) T  // 2+ arguments, returns smallest. NaNs: return smallest non-NaN, or NaN if all NaN
@max(...) T  // 2+ arguments, returns largest. Same NaN handling
```
Accepts integers, floats, and vectors of either. Operation is element-wise for vectors.

### @memcpy / @memmove / @memset (detailed)
```zig
@memcpy(noalias dest, noalias source) void  // Regions must NOT overlap
@memmove(dest, source) void                  // Regions MAY overlap
@memset(dest, elem) void                     // Set all elements to elem
```
- `dest` must be mutable slice, mutable pointer to array, or mutable many-item pointer
- `source` must be slice, pointer to array, or many-item pointer
- At least one must provide length; if both provide length, they must be equal
- For secure zeroing, use `std.crypto.secureZero`
