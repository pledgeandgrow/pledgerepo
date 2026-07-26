# Structs (Zig)

## Declaration

```zig
const Point = struct {
    x: f32,
    y: f32,
};

// Instance
const p: Point = .{ .x = 0.12, .y = 0.34 };
```

## Methods

Functions in a struct's namespace can be called with dot syntax. They are not special — just namespaced functions:

```zig
const Vec3 = struct {
    x: f32,
    y: f32,
    z: f32,

    pub fn init(x: f32, y: f32, z: f32) Vec3 {
        return Vec3{ .x = x, .y = y, .z = z };
    }

    pub fn dot(self: Vec3, other: Vec3) f32 {
        return self.x * other.x + self.y * other.y + self.z * other.z;
    }
};

// Usage
const v1 = Vec3.init(1.0, 0.0, 0.0);
const v2 = Vec3.init(0.0, 1.0, 0.0);
const result = v1.dot(v2);       // dot syntax
const result2 = Vec3.dot(v1, v2); // explicit
```

## Namespaced Declarations

Structs can contain constants, types, and other declarations:

```zig
const Empty = struct {
    pub const PI = 3.14;
    pub fn area(r: f32) f32 {
        return PI * r * r;
    }
};

// Empty structs have size 0
// const does_nothing: Empty = .{};
```

## Default Field Values

```zig
const Color = struct {
    r: u8 = 0,
    g: u8 = 0,
    b: u8 = 0,
    a: u8 = 255,
};

const red = Color{ .r = 255 };  // g=0, b=0, a=255 (defaults)
```

## Field Parent Pointer

Compute a base struct pointer from a field pointer:

```zig
fn setYBasedOnX(x: *f32, y: f32) void {
    const point: *Point = @fieldParentPtr("x", x);
    point.y = y;
}
```

## Generic Structs (Functions Returning Types)

Types are first-class values. Functions can return types:

```zig
fn LinkedList(comptime T: type) type {
    return struct {
        pub const Node = struct {
            prev: ?*Node,
            next: ?*Node,
            data: T,
        };
        first: ?*Node,
        last: ?*Node,
        len: usize,
    };
}

// Usage — compile-time functions are memoized
const list = LinkedList(i32){ .first = null, .last = null, .len = 0 };
const ListOfInts = LinkedList(i32);
```

## extern struct

Guaranteed C ABI layout. Field order is preserved:

```zig
const extern_struct = extern struct {
    x: i32,
    y: i32,
};
```

## packed struct

Fields have exactly the specified bit widths, with no padding. In-memory layout is guaranteed:

```zig
const Flags = packed struct {
    is_active: bool,    // 1 bit
    priority: u3,       // 3 bits
    reserved: u4,       // 4 bits — total 1 byte
};

// Bit-cast between packed struct and integer
const flags: Flags = .{ .is_active = true, .priority = 5, .reserved = 0 };
const bits: u8 = @bitCast(flags);
```

### Packed Struct Field Access

Taking addresses of packed struct fields requires special handling — fields may not be byte-aligned:

```zig
const ptr = &flags.is_active;  // *align(1:0:1) bool
```

## Struct Naming

Anonymous structs get names based on context. The compiler generates names for type inference.

## Anonymous Struct Literals

```zig
// Inferred type from context
const p: Point = .{ .x = 1, .y = 2 };

// Anonymous struct
const anon = .{ .x = 1, .y = 2 };
// @TypeOf(anon) is struct{ x: comptime_int, y: comptime_int }
```

## Tuples

Tuples are structs with fields named `@"0"`, `@"1"`, etc. and no explicit field names:

```zig
const tuple = .{ 1, "hello", true };
// Access by index
const first = tuple[0];   // 1
const second = tuple[1];  // "hello"
const len = tuple.len;    // 3

// Destructuring
const { a, b, c } = tuple;
```

## Auto-Deref

When using a pointer to a struct, fields can be accessed directly without explicit dereference:

```zig
var point = Point{ .x = 1, .y = 2 };
const ptr = &point;
ptr.x = 10;  // same as ptr.*.x = 10
```

## Struct Layout

- **Default:** No guaranteed field order or size — compiler optimizes layout
- **extern:** C ABI layout, field order preserved
- **packed:** Exact bit layout, no padding
- Use `@sizeOf` and `@offsetOf` for layout queries
- Use `@bitOffsetOf` for packed struct field offsets
