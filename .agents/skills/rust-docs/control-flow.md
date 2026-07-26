# Control Flow

**Docs:** https://doc.rust-lang.org/stable/reference/expressions.html | https://doc.rust-lang.org/book/ch03-05-control-flow.html

## if Expressions

```rust
let number = 6;

if number % 4 == 0 {
    println!("divisible by 4");
} else if number % 3 == 0 {
    println!("divisible by 3");
} else {
    println!("not divisible by 4 or 3");
}

// if is an expression — returns a value
let condition = true;
let x = if condition { 5 } else { 10 };
// Both branches must return the same type
```

## match Expressions

```rust
let value = 1;

match value {
    1 => println!("one"),
    2 | 3 => println!("two or three"),
    4..=6 => println!("four through six"),
    _ => println!("anything else"),  // _ is the wildcard pattern
}

// match is exhaustive — all cases must be handled
// match is an expression
let result = match value {
    1..=3 => "small",
    4..=9 => "medium",
    _ => "large",
};
```

### Match Guards

```rust
let num = 4;
match num {
    n if n % 2 == 0 => println!("even"),
    _ => println!("odd"),
}

// Bind values
match age {
    n @ 0..=12 => println!("child: {}", n),
    n @ 13..=19 => println!("teen: {}", n),
    n => println!("adult: {}", n),
}
```

## if let

```rust
let some_value = Some(3);

if let Some(n) = some_value {
    println!("got: {}", n);
} else {
    println!("was None");
}

// Useful for pattern matching without full match
```

## let else (Rust 1.65+)

```rust
fn get_value(opt: Option<i32>) -> i32 {
    let Some(val) = opt else {
        return -1;  // diverge if pattern doesn't match
    };
    val
}
```

## while Loops

```rust
let mut n = 0;
while n < 5 {
    println!("{}", n);
    n += 1;
}

// while let
let mut stack = Vec::new();
stack.push(1);
stack.push(2);
while let Some(top) = stack.pop() {
    println!("{}", top);
}
```

## loop

```rust
let mut count = 0;
loop {
    count += 1;
    if count == 5 {
        break;  // exit loop
    }
}

// loop returns a value from break
let mut counter = 0;
let result = loop {
    counter += 1;
    if counter == 10 {
        break counter * 2;  // returns 20
    }
};

// Labeled loops
let mut count = 0;
'outer: loop {
    let mut remaining = 10;
    loop {
        if remaining == 9 {
            break;  // breaks inner loop
        }
        if count == 2 {
            break 'outer;  // breaks outer loop
        }
        remaining -= 1;
    }
    count += 1;
}
```

## for Loops

```rust
for i in 0..5 {
    println!("{}", i);  // 0, 1, 2, 3, 4
}

for i in 0..=5 {
    println!("{}", i);  // 0, 1, 2, 3, 4, 5 (inclusive)
}

let arr = [10, 20, 30];
for val in arr.iter() {
    println!("{}", val);
}

for (index, val) in arr.iter().enumerate() {
    println!("{}: {}", index, val);
}

// Mutable iteration
let mut nums = vec![1, 2, 3];
for n in nums.iter_mut() {
    *n *= 2;
}
```

## break and continue

```rust
for i in 0..10 {
    if i == 3 {
        continue;  // skip to next iteration
    }
    if i == 7 {
        break;     // exit loop
    }
    println!("{}", i);
}

// Labeled break/continue
'outer: for i in 0..3 {
    for j in 0..3 {
        if i == 1 && j == 1 {
            break 'outer;  // break outer loop
        }
    }
}
```

## Blocks and Expressions

```rust
// Blocks are expressions — return the last expression (no semicolon)
let x = {
    let y = 5;
    let z = 10;
    y + z  // no semicolon — this is the return value
};
// x == 15

// Statements end with semicolon and return ()
let y = 5;  // statement
```

## return

```rust
fn foo() -> i32 {
    return 42;  // explicit return

    // or implicit — last expression without semicolon
    // 42
}

// return from any point in a function
fn bar(x: i32) -> i32 {
    if x < 0 {
        return -1;
    }
    x * 2
}
```

## Range Expressions

```rust
// Exclusive range: start..end
for i in 1..5 { }  // 1, 2, 3, 4

// Inclusive range: start..=end
for i in 1..=5 { }  // 1, 2, 3, 4, 5

// Open-ended ranges
let r1 = 1..;       // 1 to infinity
let r2 = ..5;       // up to 5 (exclusive)
let r3 = ..=5;      // up to 5 (inclusive)

// Range as a type
use std::ops::Range;
let r: Range<i32> = 0..10;
let contains = r.contains(&5);
```
