# Unsafe Rust and FFI

**Docs:** https://doc.rust-lang.org/stable/reference/unsafe.html | https://doc.rust-lang.org/book/ch19-01-unsafe-rust.html

## Unsafe Block

```rust
// unsafe block — allows 5 operations:
// 1. Dereference raw pointers
// 2. Call unsafe functions
// 3. Implement unsafe traits
// 4. Access static mut
// 5. Access fields of unions

unsafe {
    // raw pointer dereference
    let x: i32 = 42;
    let ptr: *const i32 = &x;
    println!("{}", *ptr);
}
```

## Raw Pointers

```rust
// *const T — immutable raw pointer
// *mut T — mutable raw pointer

let mut x = 10;

let ptr_const: *const i32 = &x;
let ptr_mut: *mut i32 = &mut x;

unsafe {
    println!("{}", *ptr_const);
    *ptr_mut = 20;
    println!("{}", *ptr_mut);
}

// Raw pointers can be null, don't have lifetimes, and can outlive data
// Raw pointers are not Send/Sync by default
// Creating a raw pointer is safe, dereferencing is unsafe
```

### Pointer Arithmetic

```rust
let arr = [1, 2, 3, 4, 5];
let ptr: *const i32 = arr.as_ptr();

unsafe {
    println!("{}", *ptr);          // 1
    println!("{}", *ptr.add(1));   // 2 — pointer arithmetic
    println!("{}", *ptr.offset(2)); // 3
}
```

## Unsafe Functions

```rust
// unsafe fn — caller must uphold invariants
unsafe fn dangerous() -> i32 {
    42
}

// Calling unsafe fn requires unsafe block
unsafe {
    let val = dangerous();
    println!("{}", val);
}

// Marking a function unsafe means the compiler can't verify safety
// The caller is responsible for meeting preconditions
```

## Unsafe Trait

```rust
// unsafe trait — implementor must uphold invariants
unsafe trait SafeToTransfer {}
unsafe impl SafeToTransfer for MyType {}

// Send and Sync are unsafe traits (auto-implemented when safe)
// Manually implementing them requires unsafe
struct MyRawPtr(*mut i32);
unsafe impl Send for MyRawPtr {}
unsafe impl Sync for MyRawPtr {}
```

## Static Mut

```rust
static mut COUNTER: i32 = 0;

fn increment() {
    unsafe {
        COUNTER += 1;
    }
}

fn get_counter() -> i32 {
    unsafe { COUNTER }
}
// Accessing static mut is unsafe due to data races
// Prefer AtomicI32 for concurrent access
```

## Unions

```rust
#[repr(C)]
union Data {
    int_val: i32,
    float_val: f32,
}

let mut d = Data { int_val: 42 };
unsafe {
    println!("int: {}", d.int_val);
    d.float_val = 3.14;
    println!("float: {}", d.float_val);
    // Reading the wrong field is undefined behavior
}
```

## FFI — Calling C Functions

```rust
// Declare external C function
extern "C" {
    fn abs(x: i32) -> i32;
}

fn main() {
    let x = -5;
    let abs_x = unsafe { abs(x) };
    println!("abs({}) = {}", x, abs_x);
}
```

### Linking C Libraries

```rust
// In Cargo.toml:
// [build-dependencies]
// cc = "1.0"

// build.rs:
// fn main() {
//     cc::Build::new()
//         .file("src/c_code.c")
//         .compile("my_c_lib");
// }

// In Rust:
#[link(name = "my_c_lib")]
extern "C" {
    fn my_c_function(x: i32) -> i32;
}
```

## FFI — Exporting Rust Functions

```rust
// Make Rust function callable from C
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a + b
}

// C-compatible struct
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[no_mangle]
pub extern "C" fn make_point(x: f64, y: f64) -> Point {
    Point { x, y }
}
```

## C Strings

```rust
use std::ffi::{CStr, CString};

// CString — owned, null-terminated C string
let c_string = CString::new("hello").unwrap();
let ptr: *const c_char = c_string.as_ptr();  // pass to C

// CStr — borrowed C string (from C to Rust)
extern "C" {
    fn get_message() -> *const std::os::raw::c_char;
}

let msg = unsafe {
    CStr::from_ptr(get_message())
};
let rust_str = msg.to_str().unwrap();  // &str
```

## Calling Conventions

```rust
// Common calling conventions:
// extern "C"      — C calling convention (default for FFI)
// extern "stdcall" — Windows API
// extern "system" — C on Unix, stdcall on Windows (for WinAPI)
// extern "Rust"   — Rust calling convention (default)
// extern "C-unwind" — C with unwinding support
// extern "aapcs"  — ARM
// extern "fastcall" — x86 fastcall
```

## Bindgen

```rust
// Use bindgen to auto-generate FFI bindings from C headers
// Cargo.toml: [build-dependencies] bindgen = "0.69"

// build.rs:
// use bindgen;
// fn main() {
//     let bindings = bindgen::Builder::default()
//         .header("wrapper.h")
//         .generate()
//         .unwrap();
//     bindings.write_to_file("src/bindings.rs").unwrap();
// }

// Then include the generated bindings:
// include!("bindings.rs");
```

## Safe Abstraction Pattern

```rust
// Wrap unsafe code in a safe API
mod safe_vec {
    struct Vec<T> {
        ptr: *mut T,
        len: usize,
        cap: usize,
    }

    impl<T> Vec<T> {
        fn new() -> Self {
            Self { ptr: std::ptr::null_mut(), len: 0, cap: 0 }
        }

        fn push(&mut self, val: T) {
            // unsafe internals, safe interface
            // ...
        }
    }

    impl<T> Drop for Vec<T> {
        fn drop(&mut self) {
            // cleanup
        }
    }
}
```

## Unsafe Guidelines

1. **Minimize unsafe** — wrap in safe abstractions
2. **Document safety invariants** — `// SAFETY: ...` comments
3. **Use tools** — Miri (UB detector), ASAN, valgrind
4. **Prefer safe alternatives** — Atomics over `static mut`, `Vec` over manual allocation
5. **Test thoroughly** — unsafe code can have subtle UB
6. **Use `#[deny(unsafe_op_in_unsafe_fn)]`** — require explicit unsafe blocks inside unsafe fns

## FFI String Types (std::ffi)

### CString and CStr — C Strings

```rust
use std::ffi::{CString, CStr};

// CString — owned, heap-allocated, null-terminated, no interior nulls
let c_string = CString::new("hello").unwrap();
let c_string_with_nul = CString::new("hello\0world");  // Err(NulError)

// Create from bytes (must end with null):
let bytes = b"hello\0";
let cstr = CStr::from_bytes_with_nul(bytes).unwrap();  // &CStr

// Create from bytes until null (no trailing null required):
let cstr = CStr::from_bytes_until_nul(b"hello\0extra").unwrap();

// CString → *const c_char:
let ptr: *const std::os::raw::c_char = c_string.as_ptr();

// CStr → &str:
let rust_str = cstr.to_str().unwrap();  // UTF-8 validation
let rust_bytes = cstr.to_bytes();        // raw bytes (no null)
let rust_bytes_with_nul = cstr.to_bytes_with_nul();

// CString → String:
let owned: String = c_string.into_string().unwrap();

// CString → Vec<u8>:
let bytes: Vec<u8> = c_string.into_bytes();  // without null
let bytes: Vec<u8> = c_string.into_bytes_with_nul();
```

### OsStr and OsString — Platform Strings

```rust
use std::ffi::{OsStr, OsString};

// OsString — owned, platform-native string (UTF-8 on Unix, UTF-16 on Windows)
// OsStr — borrowed slice of OsString

// Conversions:
let os_string = OsString::from("hello");
let os_str: &OsStr = &os_string;

// OsString <-> String (may fail on non-UTF-8):
let s: String = os_string.into_string().unwrap();  // Err if non-UTF-8
let os: OsString = OsString::from("hello");

// OsStr <-> &str:
let s: &str = os_str.to_str().unwrap();  // may fail
let os: &OsStr = OsStr::new("hello");

// From environment:
let path: OsString = std::env::var_os("PATH").unwrap();

// Used by:
// std::env::args_os() → Iterator<OsString>
// std::fs paths accept AsRef<OsStr>
// std::process::Command args accept AsRef<OsStr>
```

### C Type Aliases (std::ffi, std::os::raw)

```rust
use std::ffi;
use std::os::raw;

// These match C types on the target platform:
// c_void, c_char, c_schar, c_uchar, c_short, c_ushort,
// c_int, c_uint, c_long, c_ulong, c_longlong, c_ulonglong,
// c_float, c_double, c_size_t, c_ssize_t, c_ptrdiff_t

// Example FFI with C types:
extern "C" {
    fn malloc(size: raw::c_size_t) -> *mut raw::c_void;
    fn free(ptr: *mut raw::c_void);
    fn printf(fmt: *const raw::c_char, ...) -> raw::c_int;
}

// c_void — opaque void type (like void in C)
// Used for raw memory pointers: *mut c_void, *const c_void
```

### VaList — C Variadic Functions

```rust
use std::ffi::VaList;

// FFI with C variadic functions (printf, etc.)
extern "C" {
    fn printf(fmt: *const std::os::raw::c_char, ...) -> std::os::raw::c_int;
}

// Calling variadic C functions from Rust:
unsafe {
    printf(b"Hello, %s!\n\0".as_ptr().cast(), b"world\0".as_ptr().cast());
}

// Receiving variadic args in a Rust extern fn (rare):
// extern "C" fn my_variadic(fmt: *const c_char, mut args: VaList) -> c_int {
//     let arg: i32 = args.next_arg();
//     ...
// }
```

## Raw Pointer Utilities (std::ptr)

### NonNull — Guaranteed Non-Null Pointer

```rust
use std::ptr::NonNull;

// NonNull<T> is *mut T with guaranteed non-null and covariance
// It's the safe wrapper around raw pointers for FFI and collections

let mut x = 42;
let ptr = NonNull::new(&mut x).unwrap();  // Option<NonNull<T>>
assert_eq!(unsafe { *ptr.as_ref() }, 42);

// NonNull is covariant (unlike *mut T which is invariant)
// NonNull::dangling() — sentinel for empty allocations (non-null, aligned)
let dangling: NonNull<u8> = NonNull::dangling();
// Not dereferenceable — just a valid non-null address

// From raw pointer:
let ptr = NonNull::new(raw_ptr)?;  // None if null
```

### addr_of / addr_of_mut — Field Pointers

```rust
use std::ptr::addr_of;

// Get pointer to a field without creating a temporary reference
// (needed for unaligned or packed fields)
#[repr(packed)]
struct Packed { a: u8, b: u32 }  // b is unaligned!

let packed = Packed { a: 1, b: 42 };
// let b = &packed.b;  // UB! unaligned reference
let b_ptr = addr_of!(packed.b);  // *const u32 — safe
let b_val = unsafe { std::ptr::read_unaligned(b_ptr) };  // 42

// addr_of_mut for mutable field pointers:
let mut packed = Packed { a: 1, b: 0 };
let b_ptr = std::ptr::addr_of_mut!(packed.b);
unsafe { std::ptr::write_unaligned(b_ptr, 99); }
```

### Pointer Operations

```rust
use std::ptr;

// copy / copy_nonoverlapping — memcpy:
let src = [1, 2, 3, 4, 5];
let mut dst = [0i32; 5];
unsafe { ptr::copy_nonoverlapping(src.as_ptr(), dst.as_mut_ptr(), 5); }
// ptr::copy handles overlapping ranges (memmove)

// read / write — transfer ownership via pointer:
let mut place: i32 = 0;
unsafe { ptr::write(&mut place, 42); }      // write value
let val = unsafe { ptr::read(&place) };      // read value

// read_unaligned / write_unaligned — for unaligned access:
unsafe { ptr::write_unaligned(ptr, 42); }
let val = unsafe { ptr::read_unaligned(ptr) };

// read_volatile / write_volatile — volatile access (no optimization):
unsafe { ptr::write_volatile(ptr, 42); }

// swap — swap values at two pointers:
let mut a = 1;
let mut b = 2;
unsafe { ptr::swap(&mut a, &mut b); }  // a=2, b=1

// swap_nonoverlapping — faster swap for non-overlapping:
unsafe { ptr::swap_nonoverlapping(&mut a, &mut b, 1); }

// replace — write new value, return old:
let old = unsafe { ptr::replace(&mut a, 99) };  // old=2, a=99

// drop_in_place — run destructor without moving:
struct DropMe;
impl Drop for DropMe { fn drop(&mut self) { println!("dropped"); } }
let mut x = DropMe;
unsafe { ptr::drop_in_place(&mut x); }  // runs Drop, x is now invalid

// write_bytes — memset equivalent:
let mut buf = [0u8; 10];
unsafe { ptr::write_bytes(buf.as_mut_ptr(), 0xFF, 10); }

// null / null_mut:
let p: *const i32 = ptr::null();
let p: *mut i32 = ptr::null_mut();

// from_ref / from_mut — convert reference to raw pointer (safe):
let x = 42;
let ptr: *const i32 = ptr::from_ref(&x);
let mut y = 42;
let ptr: *mut i32 = ptr::from_mut(&mut y);
```

### Pointer Provenance

```rust
// Provenance — tracking which allocation a pointer came from
// Rust has strict provenance rules to avoid UB

// with_exposed_provenance — create pointer from integer (exposed provenance):
let addr: usize = 0x7fff1234;
let ptr = ptr::with_exposed_provenance::<u8>(addr);  // unsafe

// without_provenance — create pointer with no provenance (for sentinel values):
let ptr: *const u8 = ptr::without_provenance(0x1);  // not dereferenceable

// addr_eq — compare pointer addresses (ignoring provenance):
// ptr::addr_eq(p1, p2) — true if same address, even if different provenance

// Best practice: use strict provenance APIs, avoid integer-to-pointer casts
```

## Memory Allocation (std::alloc)

```rust
use std::alloc::{self, Layout, System, GlobalAlloc, Allocator};

// Layout — describes allocation size and alignment:
let layout = Layout::new::<u32>();  // size=4, align=4
let layout = Layout::from_size_align(16, 8)?;  // LayoutError if invalid
let layout = Layout::array::<u32>(100)?;  // array of 100 u32s

// Global allocator functions (use System by default):
unsafe {
    let ptr = alloc::alloc(layout);           // allocate
    alloc::dealloc(ptr, layout);              // deallocate
    let ptr = alloc::alloc_zeroed(layout);    // allocate zeroed
    let ptr = alloc::realloc(ptr, layout, new_size);  // resize
}

// handle_alloc_error — called on allocation failure (aborts)
// set_alloc_error_hook / take_alloc_error_hook — customize error behavior

// System — default global allocator:
unsafe impl GlobalAlloc for System { ... }

// GlobalAlloc trait — implement for custom allocator:
struct MyAllocator;
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

// #[global_allocator] — set custom global allocator:
#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;
// Only one per crate. All Box/Vec/String allocations go through it.

// Allocator trait — finer-grained than GlobalAlloc:
// Supports grow/shrink, allocate_zeroed, and error handling (Result)
// Global struct implements Allocator, wraps the global allocator
```

## Architecture-Specific Intrinsics (std::arch)

```rust
// std::arch provides SIMD intrinsics and CPU feature detection

// CPU feature detection macros (runtime):
#[cfg(target_arch = "x86_64")]
{
    if std::arch::is_x86_feature_detected!("sse4.1") {
        // use SSE 4.1 intrinsics
    }
    if std::arch::is_x86_feature_detected!("avx2") {
        // use AVX2 intrinsics
    }
}
// Other architectures:
// is_aarch64_feature_detected!("neon")
// is_arm_feature_detected!("neon")
// is_powerpc_feature_detected!("altivec")
// is_riscv_feature_detected!("v")

// #[target_feature] — enable specific CPU features for a function:
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.1")]
unsafe fn hex_encode_sse41(src: &[u8], dst: &mut [u8]) {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;
    unsafe {
        let invec = _mm_loadu_si128(src.as_ptr() as *const _);
        // ... SIMD operations
    }
}

// Pattern: fallback + optimized path
pub fn hex_encode(src: &[u8], dst: &mut [u8]) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if std::arch::is_x86_feature_detected!("sse4.1") {
            return unsafe { hex_encode_sse41(src, dst) };
        }
    }
    hex_encode_fallback(src, dst);
}

fn hex_encode_fallback(src: &[u8], dst: &mut [u8]) {
    for (byte, slots) in src.iter().zip(dst.chunks_mut(2)) {
        slots[0] = b"0123456789abcdef"[(byte >> 4) as usize];
        slots[1] = b"0123456789abcdef"[(byte & 0xf) as usize];
    }
}

// Static feature detection via cfg(target_feature):
#[cfg(target_feature = "avx2")]
fn fast_path() { /* compile-time AVX2 */ }

// std::arch::x86_64 intrinsics: _mm_add_epi8, _mm_loadu_si128, etc.
// std::arch::aarch64 intrinsics: vaddq_u8, vld1q_u8, etc.
// All intrinsics are unsafe — require target_feature or cfg gate
```

## Portable SIMD (std::simd)

```rust
use std::simd::{Simd, Mask, simd_swizzle};
use std::simd::prelude::*;

// Simd<T, N> — portable SIMD vector, like [T; N] with parallel operations
// Compiles to best available SIMD instructions on any target
// N must be a power of 2: 1, 2, 4, 8, 16, 32, 64
let a: Simd<i32, 4> = Simd::from_array([1, 2, 3, 4]);
let b: Simd<i32, 4> = Simd::splat(10);  // [10, 10, 10, 10]

// Element-wise operations:
let sum = a + b;          // [11, 12, 13, 14]
let product = a * b;      // [10, 20, 30, 40]
let neg = -a;             // [-1, -2, -3, -4]

// Mask — boolean vector result of comparisons:
let mask = a.simd_gt(Simd::splat(2));  // Mask<i32, 4>: [false, false, true, true]
let selected = mask.select(a, b);      // [10, 10, 3, 4]

// Horizontal reductions:
let total: i32 = a.reduce_sum();
let max_val: i32 = a.reduce_max();
let min_val: i32 = a.reduce_min();

// Conversions:
let arr: [i32; 4] = a.to_array();
let from_arr = Simd::from_array([5, 6, 7, 8]);
let cast: Simd<f32, 4> = a.cast();  // integer to float

// simd_swizzle! — rearrange lanes:
let shuffled: Simd<i32, 4> = simd_swizzle!(a, [3, 2, 1, 0]);  // reverse

// Traits:
// SimdElement — marker for types usable in Simd (i8..i64, u8..u64, f32, f64, usize, isize)
// SimdFloat / SimdInt / SimdUint — numeric operations on SIMD vectors
// SimdOrd / SimdPartialEq / SimdPartialOrd — comparison operations
// Select — mask.select(true_vals, false_vals)
// Swizzle — trait for lane rearrangement
// ToBytes — convert Simd to/from byte arrays
// MaskElement — marker for types usable in Mask (i8, i16, i32, i64, isize)
// SimdCast — type casting
// StdFloat — math.h functions (sqrt, sin, cos, etc.) on SIMD vectors

// Simd pointer operations (std::simd::ptr):
// SimdConstPtr / SimdMutPtr — SIMD vectors of raw pointers
// gather/scatter operations for strided memory access

// Unlike std::arch, std::simd is safe and portable — no unsafe required
```

## Pointer Utility Functions

```rust
// ptr::from_ref(&T) → *const T — safe conversion from reference to raw pointer
// Stable since 1.76. Does not require unsafe.
use std::ptr;
let x: i32 = 42;
let ptr_const: *const i32 = ptr::from_ref(&x);

// ptr::from_mut(&mut T) → *mut T — safe conversion from mutable reference
let mut y: i32 = 10;
let ptr_mut: *mut i32 = ptr::from_mut(&mut y);

// ptr::without_provenance(addr: usize) → *const T — create pointer from address
// Creates a pointer with no provenance (no associated allocation).
// Useful for storing addresses as integers (e.g., packed pointers).
// The pointer cannot be dereferenced — only used for comparisons/roundtrip.
let dangling: *const u8 = ptr::without_provenance(0x1000);
// Also: ptr::without_provenance_mut(addr) → *mut T

// ptr::addr_eq(a: *const T, b: *const U) → bool — compare pointer addresses
// Returns true if pointers point to the same address.
// Unlike `==`, works across different provenance and types.
let a: *const u8 = &0u8;
let b: *const u8 = &0u8;
// ptr::addr_eq(a, b) compares addresses regardless of provenance

// ptr::fn_addr_eq — compare function pointer addresses (nightly)

// Memory fences — synchronize memory access across threads
use std::sync::atomic;

// fence(ordering) — full memory fence barrier
// Prevents reordering of loads/stores across the fence.
// Stronger than individual atomic operations' orderings.
atomic::fence(Ordering::SeqCst);    // full barrier — all prior ops visible
atomic::fence(Ordering::Acquire);   // acquire fence — prior loads visible
atomic::fence(Ordering::Release);   // release fence — stores visible to others

// compiler_fence(ordering) — compiler-only fence (no CPU barrier)
// Prevents compiler from reordering, but CPU may still reorder.
// Useful when combined with atomic operations that already provide
// hardware ordering (e.g., on strongly-ordered architectures like x86).
atomic::compiler_fence(Ordering::SeqCst);

// Common fence pattern with SeqCst:
use std::sync::atomic::AtomicUsize;
static COUNTER: AtomicUsize = AtomicUsize::new(0);
COUNTER.fetch_add(1, Ordering::Relaxed);
atomic::fence(Ordering::SeqCst);  // ensure all threads see this increment
```
