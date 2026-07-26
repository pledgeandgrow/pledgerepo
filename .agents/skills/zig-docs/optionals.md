# Optionals (Zig)

## Optional Type

The `?` prefix creates an optional type. An optional can hold the value or `null`:

```zig
const normal_int: i32 = 1234;
const optional_int: ?i32 = 5678;
const null_int: ?i32 = null;
```

## Unwrapping

### orelse

Unwrap with a fallback value:

```zig
const value = optional_int orelse 0;
```

### orelse unreachable

Assert non-null:

```zig
const value = optional_int orelse unreachable;
```

### if unwrap

```zig
if (optional_int) |value| {
    // value is i32 (non-optional)
} else {
    // was null
}

// Pointer capture for mutation
var opt: ?i32 = 5;
if (opt) |*value| {
    value.* = 10;
}
```

### .? operator

Shorthand for `orelse unreachable` — unwrap, panic if null:

```zig
const value = optional_int.?;  // panics if null in safe modes
```

## Optional Pointers

Optional pointers compile down to regular pointers (null = address 0). No runtime overhead:

```zig
const ptr: ?*i32 = null;
// In memory, this is just a pointer with value 0
```

### Comparison with C

```c
// C
void *ptr = malloc(1234);
if (!ptr) return NULL;
```

```zig
// Zig — orelse unwraps and guarantees non-null after
const ptr = malloc(1234) orelse return null;
// ptr is [*]u8, not ?[*]u8 — guaranteed non-null
```

### Null Check Pattern

```zig
fn doAThing(optional_foo: ?*Foo) void {
    if (optional_foo) |foo| {
        doSomethingWithFoo(foo);
        // foo is *Foo (non-optional) inside this block
    }
}
```

## Optional in while

```zig
var iterator: ?Node = getFirst();
while (iterator) |node| {
    // node is Node (unwrapped)
    iterator = node.next;
}
```

## Optional Return Type

```zig
fn findItem(items: []const Item, target: i32) ?Item {
    for (items) |item| {
        if (item.value == target) return item;
    }
    return null;
}
```

## Optional Chaining

Zig does not have optional chaining (`?.`). Use explicit unwrapping:

```zig
// Instead of obj?.field?.subfield
if (obj) |o| {
    if (o.field) |f| {
        // use f.subfield
    }
}
```

## Type Coercion with Optionals

- `T` coerces to `?T` (value to optional)
- `?*T` coerces to `?[*]T` (optional pointer to optional many-pointer)
- `null` coerces to any `?T`

```zig
const opt: ?i32 = 5;  // i32 coerces to ?i32
const ptr_opt: ?*u8 = null;
```

## Optional Payload Capture in switch

```zig
const val: ?i32 = 42;
const result = switch (val) {
    null => 0,
    .payload => |v| v * 2,
};
```
