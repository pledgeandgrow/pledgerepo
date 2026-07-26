# Smart Pointers

**Docs:** https://doc.rust-lang.org/book/ch15-00-smart-pointers.html | https://doc.rust-lang.org/std/boxed/index.html

## Box<T>

Heap-allocated, single-owner smart pointer.

```rust
// Allocate value on the heap
let b = Box::new(5);
println!("{}", b);  // dereferenced automatically

// Box for recursive types
enum List {
    Cons(i32, Box<List>),
    Nil,
}

let list = List::Cons(1, Box::new(List::Cons(2, Box::new(List::Nil))));

// Box for large data that shouldn't be copied
let large = Box::new([0u8; 1_000_000]);  // 1MB on heap

// Box<dyn Trait> — trait object
let shape: Box<dyn Shape> = Box::new(Circle { radius: 5.0 });

// Box::leak — leak memory, get 'static reference
let static_ref: &'static [i32] = Box::leak(Box::new([1, 2, 3]));

// ThinBox<T> — boxed pointer with pointer-sized metadata (nightly)
// Like Box<T> but uses only one word for metadata instead of two
// Useful for boxed DSTs (dynamically sized types) like Box<dyn Trait>
// ThinBox::new(val) / ThinBox::new_unsize(val)
```

## Rc<T> — Reference Counted (Single-threaded)

```rust
use std::rc::Rc;

// Multiple owners of the same data (single-threaded only)
let data = Rc::new(vec![1, 2, 3]);
let data2 = Rc::clone(&data);  // increments ref count, not a copy
let data3 = Rc::clone(&data);

println!("Count: {}", Rc::strong_count(&data));  // 3

// Data dropped when last Rc is dropped
drop(data2);
println!("Count: {}", Rc::strong_count(&data));  // 2

// Rc is immutable — can't modify through Rc
// Use RefCell<Rc<T>> for interior mutability
```

## Arc<T> — Atomic Reference Counted (Thread-safe)

```rust
use std::sync::Arc;

// Thread-safe version of Rc — use across threads
let data = Arc::new(vec![1, 2, 3]);

let data_clone = Arc::clone(&data);
std::thread::spawn(move || {
    println!("{:?}", data_clone);
}).join().unwrap();

println!("{:?}", data);  // still valid

// Arc is slightly slower than Rc due to atomic operations
// Use Arc only when sharing across threads
```

## Cell<T> — Interior Mutability (Copy types)

```rust
use std::cell::Cell;

// Cell provides interior mutability for Copy types
// No borrow checking — get/set/replace directly
let cell = Cell::new(5);
cell.set(10);
let val = cell.get();  // copies the value
cell.replace(20);

// Cell is not Sync — can't share across threads
// Cell is Send if T is Send
```

## RefCell<T> — Interior Mutability (Runtime borrow checking)

```rust
use std::cell::RefCell;

// RefCell moves borrow checking to runtime
let cell = RefCell::new(vec![1, 2, 3]);

// Immutable borrow — panics if already mutably borrowed
{
    let borrowed = cell.borrow();
    println!("{:?}", *borrowed);
}

// Mutable borrow — panics if already borrowed
{
    let mut borrowed = cell.borrow_mut();
    borrowed.push(4);
}

// try_borrow / try_borrow_mut — non-panicking versions
match cell.try_borrow_mut() {
    Ok(ref mut data) => data.push(5),
    Err(_) => println!("already borrowed"),
}
```

### Rc<RefCell<T>> — Multiple Owners with Mutation

```rust
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
    prev: Option<Weak<RefCell<Node>>>,  // Weak to prevent cycles
}

let a = Rc::new(RefCell::new(Node { value: 1, next: None, prev: None }));
let b = Rc::new(RefCell::new(Node { value: 2, next: None, prev: None }));

a.borrow_mut().next = Some(Rc::clone(&b));
b.borrow_mut().prev = Some(Rc::downgrade(&a));  // Weak reference
```

## Weak<T> — Weak References

```rust
use std::rc::{Rc, Weak};

let strong = Rc::new(42);
let weak: Weak<i32> = Rc::downgrade(&strong);

// Upgrade returns Option — None if strong refs are all gone
if let Some(val) = weak.upgrade() {
    println!("{}", val);
}

drop(strong);
assert!(weak.upgrade().is_none());  // strong refs gone

// Weak references don't prevent dropping
// Use to break reference cycles
```

## Cow<T> — Clone on Write

```rust
use std::borrow::Cow;

// Cow can hold either borrowed or owned data
fn process(input: &str) -> Cow<str> {
    if input.contains("bad") {
        Cow::Owned(input.replace("bad", "good"))  // owned
    } else {
        Cow::Borrowed(input)  // borrowed — no allocation
    }
}

let result = process("hello");
let result = process("bad word");

// Cow derefs to the borrowed type
println!("{}", &result);
```

## Deref and DerefMut

```rust
use std::ops::{Deref, DerefMut};

struct MyBox<T>(T);

impl<T> Deref for MyBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MyBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

let b = MyBox(5);
println!("{}", *b);  // deref to 5

// Deref coercion — &MyBox<String> -> &String -> &str
fn takes_str(s: &str) { println!("{}", s); }
let mb = MyBox(String::from("hello"));
takes_str(&mb);  // auto-deref through MyBox -> String -> str
```

## Drop

```rust
struct Resource { name: String }

impl Drop for Resource {
    fn drop(&mut self) {
        println!("Dropping: {}", self.name);
    }
}

let r = Resource { name: "file".into() };
// drop called automatically when r goes out of scope

// Manual drop
let r2 = Resource { name: "conn".into() };
drop(r2);  // dropped now
println!("After drop");

// Drop order: fields dropped in declaration order, then the struct
```

## When to Use What

| Type | Ownership | Thread-safe | Mutability | Use Case |
|------|-----------|-------------|------------|----------|
| `Box<T>` | Single | Yes (if T: Send) | No (by default) | Heap alloc, trait objects, recursive types |
| `Rc<T>` | Shared | No | No | Multiple owners, single-threaded |
| `Arc<T>` | Shared | Yes | No | Multiple owners, multi-threaded |
| `Cell<T>` | Single | No | Yes (Copy types) | Simple interior mutability |
| `RefCell<T>` | Single | No | Yes (runtime check) | Interior mutability, single-threaded |
| `Mutex<T>` | Shared | Yes | Yes (lock) | Interior mutability, multi-threaded |
| `RwLock<T>` | Shared | Yes | Yes (lock) | Read-heavy concurrent access |
| `Weak<T>` | None | Depends | No | Break reference cycles |
| `Cow<T>` | Either | Yes (if B: Sync) | Clone on write | Avoid allocation when possible |

## OnceCell (std::cell::OnceCell)

```rust
use std::cell::OnceCell;

// OnceCell — a cell that can be written to only once
// Single-threaded version (for multi-threaded, use std::sync::OnceLock)
let cell = OnceCell::new();
assert_eq!(cell.get(), None);

cell.set(42).unwrap();  // Ok if first time
assert!(cell.set(99).is_err());  // Already set
assert_eq!(cell.get(), Some(&42));

// get_or_init — lazily initialize:
let cell = OnceCell::new();
let val = cell.get_or_init(|| expensive_computation());
assert_eq!(cell.get(), Some(val));

// get_mut — get &mut T if uniquely borrowed (no one else has &OnceCell)
if let Some(ref_mut) = cell.get_mut() {
    *ref_mut += 1;
}

// into_inner — extract value, consuming the cell
let val = cell.into_inner().unwrap();
```

## LazyCell (std::cell::LazyCell)

```rust
use std::cell::LazyCell;

// LazyCell — lazily initialized, write-once cell (single-threaded)
// Like OnceCell but initialization happens on first access
let cell = LazyCell::new(|| expensive_init());
let val = &*cell;  // initializes on first deref
// For multi-threaded: use std::sync::LazyLock
```

## SyncUnsafeCell (std::cell::SyncUnsafeCell)

```rust
use std::cell::SyncUnsafeCell;

// SyncUnsafeCell — UnsafeCell that is also Sync
// Allows sharing &SyncUnsafeCell<T> across threads
// All access still requires unsafe — no interior mutability safety
// Useful for implementing custom synchronization primitives
let cell = SyncUnsafeCell::new(42);
// unsafe { *cell.get() } — access requires unsafe
```

## RefCell Borrow Errors

```rust
use std::cell::RefCell;

// BorrowError / BorrowMutError — returned by try_borrow / try_borrow_mut
let cell = RefCell::new(42);
let r1 = cell.borrow();

// try_borrow returns Result, not Option:
match cell.try_borrow() {
    Ok(r) => println!("{}", *r),
    Err(e) => println!("already borrowed: {:?}", e),  // BorrowError
}

// try_borrow_mut:
match cell.try_borrow_mut() {
    Ok(r) => println!("{}", *r),
    Err(e) => println!("already borrowed: {:?}", e),  // BorrowMutError
}
```

## UniqueRc and UniqueArc (nightly)

```rust
use std::rc::UniqueRc;
// use std::sync::UniqueArc;

// UniqueRc — Rc with unique (unshared) reference at creation
// Allows mutable access without RefCell when only one Rc exists
// Can be upgraded to Rc<T> once sharing is needed
let rc = UniqueRc::new(42);
let mut rc_mut = rc;  // unique access
*rc_mut = 100;  // direct mutation, no RefCell needed

// UniqueArc — thread-safe version of UniqueRc
// use std::sync::UniqueArc;
// let arc = UniqueArc::new(42);
// *arc = 100;  // direct mutation while unique
// let shared: Arc<T> = arc.into();  // share once done mutating
```

## UnsafePinned (std::pin::UnsafePinned)

```rust
use std::pin::UnsafePinned;

// UnsafePinned — marker type that makes a type !Unpin (nightly)
// Like PhantomPinned but as a wrapper struct instead of a marker field
// UnsafePinned<T> wraps T and prevents moving the inner value
// Requires unsafe to construct: UnsafePinned::new_unchecked(val)
```
