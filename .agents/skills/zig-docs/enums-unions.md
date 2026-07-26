# Enums & Unions (Zig)

## Enums

```zig
const Type = enum {
    ok,
    not_ok,
};

const c = Type.ok;
```

### Tag Type & Ordinal Values

```zig
const Value = enum(u2) {
    zero,  // 0
    one,   // 1
    two,   // 2
};

// Cast between tag type and enum
const ordinal: u2 = @intFromEnum(Value.one);  // 1
const from_ordinal = @enumFromInt(@as(u2, 2));  // Value.two
```

### Overriding Ordinal Values

```zig
const Value2 = enum(u32) {
    hundred = 100,
    thousand = 1000,
    million = 1000000,
};

// Override only some values — others auto-increment
const Value3 = enum(u4) {
    a,      // 0
    b = 8,  // 8
    c,      // 9
    d = 4,  // 4
    e,      // 5
};
```

### Enum Methods

Enum methods are namespaced functions callable with dot syntax:

```zig
const Suit = enum {
    clubs,
    spades,
    diamonds,
    hearts,

    pub fn isClubs(self: Suit) bool {
        return self == Suit.clubs;
    }
};

const p = Suit.spades;
const result = p.isClubs();  // false
```

### Enum Switch

```zig
const Foo = enum { string, number, none };

const p = Foo.number;
const what = switch (p) {
    Foo.string => "this is a string",
    Foo.number => "this is a number",
    Foo.none => "this is a none",
};
```

### Non-exhaustive Enums

```zig
const NonExhaustive = enum(u8) {
    a,
    b,
    _,
};

// Must have else when switching
const val = switch (non_exhaustive) {
    .a => 1,
    .b => 2,
    else => 0,
};
```

### extern enum

Guaranteed C ABI size. Tag type defaults to `c_int`.

### Enum Literals

```zig
// Enum literal — untyped, coerced by context
const x: Foo = .number;

// In switch
const result = switch (p) {
    .string => "string",
    .number => "number",
    .none => "none",
};
```

### Type Info

```zig
const Small = enum { one, two, three, four };

// Tag type
const tag_type = @typeInfo(Small).@"enum".tag_type;  // u2

// Field count and names
const field_count = @typeInfo(Small).@"enum".fields.len;  // 4
const field_name = @typeInfo(Small).@"enum".fields[1].name;  // "two"

// Get string name of enum value
const name = @tagName(Small.three);  // "three"
```

## Unions

A bare union defines possible types. Only one field active at a time.

```zig
const Payload = union {
    int: i64,
    float: f64,
    boolean: bool,
};

var payload = Payload{ .int = 1234 };
```

### Safety

Accessing the non-active field is safety-checked illegal behavior:

```zig
var payload = Payload{ .int = 1234 };
payload.float = 12.34;  // ❌ Panic: access of union field 'float' while field 'int' is active
```

Switch fields by assigning the entire union:

```zig
var payload = Payload{ .int = 1234 };
payload = Payload{ .float = 12.34 };  // ✅ OK
```

## Tagged Unions

Unions with an enum tag. Required for `switch` with payload capture:

```zig
const Item = union(enum) {
    a: u32,
    b: []const u8,
    c: struct { x: u8, y: u8 },
    d,  // void (no payload)
};

var item = Item{ .c = .{ .x = 1, .y = 2 } };

// Switch with payload capture
const result = switch (item) {
    .a => |val| val,
    .b => |str| str.len,
    .c => |*pt| blk: {
        pt.*.x += 1;
        break :blk pt.*.x;
    },
    .d => 0,
};
```

### Explicit Tag Type

```zig
const Tag = enum(u2) { a, b, c };

const Value = union(Tag) {
    a: u32,
    b: f32,
    c: bool,
};
```

### @unionInit

Initialize a union when the tag is a comptime-known name:

```zig
const U = union(enum) { a: u32, b: f32 };
const u = @unionInit(U, "a", 42);
```

## extern union

Guaranteed C ABI layout. In-memory representation is guaranteed:

```zig
const extern_union = extern union {
    int_val: i32,
    float_val: f32,
};
```

## packed union

All fields share the same memory location with guaranteed layout. Only one field can be active.

## Anonymous Union Literals

```zig
const U = union(enum) { a: u32, b: f32 };
const u: U = .{ .a = 42 };  // type inferred from context
```

## Union Methods

Same as structs and enums — namespaced functions callable with dot syntax:

```zig
const Result = union(enum) {
    ok: u32,
    err: []const u8,

    pub fn isOk(self: Result) bool {
        return self == .ok;
    }
};
```
