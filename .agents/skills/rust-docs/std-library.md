# Standard Library Reference

**Docs:** https://doc.rust-lang.org/std/index.html

## Module Overview

| Module | Description |
|--------|-------------|
| `std::alloc` | Memory allocation |
| `std::any` | Runtime type information (Any, TypeId) |
| `std::array` | Array operations |
| `std::ascii` | ASCII operations |
| `std::borrow` | Borrow traits (Borrow, BorrowMut, ToOwned, Cow) |
| `std::boxed` | Box<T> heap allocation |
| `std::cell` | Interior mutability (Cell, RefCell, UnsafeCell) |
| `std::char` | Character operations |
| `std::clone` | Clone trait |
| `std::cmp` | Comparison traits (Ord, Eq, PartialOrd, PartialEq, Ordering, Reverse) |
| `std::collections` | HashMap, HashSet, BTreeMap, BTreeSet, VecDeque, LinkedList, BinaryHeap |
| `std::convert` | From, Into, TryFrom, TryInto, AsRef, AsMut |
| `std::default` | Default trait |
| `std::env` | Environment variables, args |
| `std::error` | Error trait |
| `std::ffi` | C FFI types (CStr, CString, OsStr, OsString) |
| `std::fmt` | Formatting and Display/Debug traits |
| `std::fs` | File system operations |
| `std::future` | Future trait |
| `std::hash` | Hash trait and hashing (Hash, Hasher, BuildHasher, DefaultHasher, RandomState) |
| `std::hint` | Compiler hints (spin_loop, black_box) |
| `std::io` | I/O traits, BufReader, BufWriter |
| `std::iter` | Iterator trait and adaptors |
| `std::marker` | PhantomData, Send, Sync, Sized, Unpin |
| `std::mem` | Memory operations (size_of, align_of, swap, replace, take, transmute) |
| `std::net` | TCP, UDP, Unix sockets |
| `std::num` | Numeric types, NonZero, Wrapping, Saturating, ParseIntError, FpCategory |
| `std::ops` | Operator traits (Add, Sub, Range, Deref, Index, Fn, FnMut, FnOnce, Try, Drop) |
| `std::option` | Option<T> |
| `std::os` | OS-specific functionality (unix, windows) |
| `std::panic` | Panic configuration |
| `std::path` | Path, PathBuf |
| `std::pin` | Pin<T> — prevents moving of pinned values |
| `std::process` | Process spawning, exit |
| `std::ptr` | Raw pointer operations |
| `std::range` | New range types (Range, RangeFrom, RangeInclusive, RangeToInclusive) |
| `std::rc` | Rc<T>, Weak<T> |
| `std::result` | Result<T, E> |
| `std::slice` | Slice operations |
| `std::str` | String slice operations |
| `std::string` | String, ToString |
| `std::sync` | Arc, Mutex, RwLock, Barrier, Condvar, Once, OnceLock, LazyLock, ReentrantLock, mpsc, mpmc, oneshot |
| `std::task` | Async task types |
| `std::thread` | Thread spawning, LocalKey |
| `std::time` | Instant, Duration, SystemTime |
| `std::vec` | Vec<T> |
| `std::arch` | SIMD intrinsics, CPU feature detection |
| `std::async_iter` | AsyncIterator trait (async iteration) |
| `std::backtrace` | Backtrace capture for error diagnostics |
| `std::bstr` | ByteStr, ByteString (byte string operations) |
| `std::f16` | 16-bit half-precision float constants |
| `std::f128` | 128-bit quad-precision float constants |
| `std::from` | #[derive(From)] macro (unstable) |
| `std::prelude` | Auto-imported items (traits, types, macros) |
| `std::random` | random(), RandomSource, Distribution |
| `std::autodiff` | Compile-time automatic differentiation (nightly) |
| `std::field` | Field reflection/projection (nightly) |
| `std::simd` | Portable SIMD vectors (Simd, Mask) |

## I/O

```rust
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::fs::File;

// Read file
let content = std::fs::read_to_string("file.txt")?;
let bytes = std::fs::read("file.bin")?;

// Write file
std::fs::write("output.txt", "hello world")?;

// Buffered I/O
let file = File::open("large.txt")?;
let mut reader = BufReader::new(file);
let mut content = String::new();
reader.read_to_string(&mut content)?;

let file = File::create("output.bin")?;
let mut writer = BufWriter::new(file);
writer.write_all(b"data")?;
writer.flush()?;

// stdin/stdout
let mut input = String::new();
io::stdin().read_line(&mut input)?;
println!("You said: {}", input.trim());

// Read line by line
use std::io::BufRead;
let file = File::open("file.txt")?;
for line in BufReader::new(file).lines() {
    println!("{}", line?);
}

// Seek — random access to a stream:
use std::io::{Seek, SeekFrom};
let mut file = File::open("data.bin")?;
file.seek(SeekFrom::Start(1024))?;       // absolute position
file.seek(SeekFrom::Current(-10))?;      // relative to current
file.seek(SeekFrom::End(0))?;            // relative to end
let pos = file.stream_position()?;       // current position

// BufRead — buffered reading with line/byte support:
use std::io::BufRead;
let mut reader = BufReader::new(File::open("file.txt")?);
let mut buf = String::new();
reader.read_line(&mut buf)?;   // read one line
reader.read_until(b'\n', &mut buf)?;  // read until byte
// fill_buf() — peek at buffered data without consuming
// consume(n) — skip n bytes from buffer

// IsTerminal — check if a stream is a terminal (1.70+):
use std::io::IsTerminal;
if io::stdin().is_terminal() {
    println!("Running in interactive mode");
}
// Also: io::stdout().is_terminal(), io::stderr().is_terminal()

// io::RawOsError — type alias for raw OS error codes (i32 on most platforms)
// io::Error::raw_os_error() → Option<RawOsError>
```

## File System

```rust
use std::fs;
use std::path::Path;

// Create directory
fs::create_dir("new_dir")?;
fs::create_dir_all("a/b/c")?;

// Remove
fs::remove_file("file.txt")?;
fs::remove_dir("empty_dir")?;
fs::remove_dir_all("dir")?;

// List directory
for entry in fs::read_dir(".")? {
    let entry = entry?;
    println!("{}", entry.path().display());
}

// Metadata
let metadata = fs::metadata("file.txt")?;
let size = metadata.len();
let is_file = metadata.is_file();
let is_dir = metadata.is_dir();

// Copy, rename
fs::copy("src.txt", "dst.txt")?;
fs::rename("old.txt", "new.txt")?;

// Canonical path
let canon = fs::canonicalize("../file.txt")?;

// Symlink (Unix) / soft_link (cross-platform shim):
#[cfg(unix)]
fs::soft_link("target.txt", "link.txt")?;  // symlink on Unix
// Windows: use std::os::windows::fs::symlink_file / symlink_dir

// Hard link:
fs::hard_link("original.txt", "link.txt")?;

// Read symlink target:
let target = fs::read_link("link.txt")?;

// Symlink metadata (doesn't follow symlinks):
let meta = fs::symlink_metadata("link.txt")?;

// File exists:
let exists = fs::exists("file.txt")?;  // bool (1.97+)

// TOCTOU: avoid check-then-act patterns — use File::create_new instead:
let file = fs::OpenOptions::new()
    .write(true)
    .create_new(true)  // atomic: fails if file exists
    .open("file.txt")?;
```

### OpenOptions — Fine-Grained File Control

```rust
use std::fs::OpenOptions;

let file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)        // create if doesn't exist
    .create_new(true)    // create only if doesn't exist (atomic, avoids TOCTOU)
    .truncate(true)      // truncate to 0 if exists
    .append(true)        // append mode
    .open("file.txt")?;

// DirBuilder — create directories with options:
let mut builder = fs::DirBuilder::new();
builder.recursive(true);  // like create_dir_all
#[cfg(unix)]
{
    use std::os::unix::fs::DirBuilderExt;
    builder.mode(0o755);  // Unix permissions
}
builder.create("a/b/c")?;

// FileTimes — set file access/modify times:
use std::fs::FileTimes;
let times = FileTimes::new()
    .set_accessed(SystemTime::now())
    .set_modified(SystemTime::now());
fs::set_times("file.txt", times)?;
fs::set_times_nofollow("link.txt", times)?;  // don't follow symlinks

// File locking (Unix-only):
use std::fs::File;
let file = File::open("file.txt")?;
// file.try_lock()?;       // exclusive lock
// file.try_lock_shared()?; // shared lock
// file.unlock()?;
```

## Path

```rust
use std::path::{Path, PathBuf};

let path = Path::new("src/main.rs");
let parent = path.parent();           // Some("src")
let file_name = path.file_name();     // Some("main.rs")
let extension = path.extension();     // Some("rs")
let stem = path.file_stem();          // Some("main")

// Join
let full = Path::new("/home").join("user").join("file.txt");

// PathBuf — owned, mutable path
let mut pb = PathBuf::from("/tmp");
pb.push("file.txt");
pb.set_extension("log");

// Path components iteration:
use std::path::Component;
for component in Path::new("/home/user/file.txt").components() {
    match component {
        Component::RootDir => print!("/"),
        Component::Normal(name) => print!("{}/", name),
        Component::CurDir => print!("./"),
        Component::ParentDir => print!("../"),
        Component::Prefix(prefix) => print!("{:?}", prefix),  // Windows: C:\
    }
}

// Ancestors — iterate from path up to root:
for ancestor in Path::new("/a/b/c").ancestors() {
    println!("{}", ancestor.display());
    // /a/b/c, /a/b, /a, /, then None
}

// Strip prefix:
let path = Path::new("/a/b/c");
let stripped = path.strip_prefix("/a")?;  // "b/c" — StripPrefixError if not prefix

// Normalize lexically (1.97+):
let path = Path::new("a/../b/./c");
let normalized = path.normalize_lexical()?;  // "b/c"

// Starts/ends with:
assert!(Path::new("/a/b/c").starts_with("/a"));
assert!(Path::new("/a/b/c").ends_with("b/c"));

// Join multiple components:
let path = Path::new("/home").join("user").join("docs").join("file.txt");

// Path to string (may fail for non-UTF-8):
let s = path.to_str()?;  // Option<&str>
let s = path.to_string_lossy();  // Cow<str> (replaces invalid with U+FFFD)

// Constants:
// std::path::MAIN_SEPARATOR — '/' on Unix, '\\' on Windows
// std::path::MAIN_SEPARATOR_STR — same as &str
// std::path::SEPARATORS — &['/'] on Unix, &['\\', '/'] on Windows
```

## Process

```rust
use std::process::Command;

// Run command
let output = Command::new("echo")
    .arg("hello")
    .output()?;

println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
println!("exit code: {}", output.status.code().unwrap());

// Spawn and interact
let mut child = Command::new("cat")
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()?;

// Write to stdin
use std::io::Write;
if let Some(stdin) = child.stdin.as_mut() {
    stdin.write_all(b"hello")?;
}

// Wait for completion
let status = child.wait()?;
```

## Environment

```rust
use std::env;

// Environment variables
let path = env::var("PATH")?;
let opt = env::var_os("HOME");

// Set environment variable
env::set_var("MY_VAR", "value");

// Arguments
let args: Vec<String> = env::args().collect();
let args_os: Vec<std::ffi::OsString> = env::args_os().collect();

// Current directory
let cwd = env::current_dir()?;
env::set_current_dir("/tmp")?;

// Current executable
let exe = env::current_exe()?;

// Home directory (1.97+):
let home = env::home_dir()?;  // Option<PathBuf>

// Temp directory:
let tmp = env::temp_dir();  // PathBuf

// Iterate all env vars:
for (key, value) in env::vars() {
    println!("{}={}", key, value);
}
// OsString version (handles non-UTF-8):
for (key, value) in env::vars_os() {
    println!("{:?}", key);
}

// Remove env var:
env::remove_var("MY_VAR");

// PATH manipulation:
let paths = env::split_paths(&env::var_os("PATH").unwrap());
let joined = env::join_paths(paths)?;  // JoinPathsError on failure

// Constants:
// env::consts::OS — "linux", "windows", "macos"
// env::consts::ARCH — "x86_64", "aarch64"
// env::consts::FAMILY — "unix", "windows"
// env::consts::DLL_PREFIX — "lib" on Unix, "" on Windows
// env::consts::DLL_SUFFIX — ".so" / ".dll" / ".dylib"
// env::consts::EXE_SUFFIX — "" on Unix, ".exe" on Windows
```

## Time

```rust
use std::time::{Duration, Instant, SystemTime};

// Duration
let five_secs = Duration::from_secs(5);
let half_sec = Duration::from_millis(500);
let one_nano = Duration::from_nanos(1);

// Instant — monotonic clock for measuring elapsed time
let start = Instant::now();
// ... do work ...
let elapsed = start.elapsed();
println!("Took: {:?}", elapsed);

// SystemTime — wall clock
let now = SystemTime::now();
let unix_epoch = SystemTime::UNIX_EPOCH;
let since_epoch = now.duration_since(unix_epoch)?;
println!("Seconds since epoch: {}", since_epoch.as_secs());

// Sleep
std::thread::sleep(Duration::from_millis(100));

// Duration arithmetic:
let d1 = Duration::from_secs(10);
let d2 = Duration::from_millis(500);
let total = d1 + d2;              // 10.5s
let diff = d1 - d2;               // 9.5s
let doubled = d1 * 2;            // 20s
let halved = d1 / 2;             // 5s
let checked = d1.checked_mul(3); // Option<Duration>
let checked = d1.checked_sub(d2);// Option<Duration>

// Duration from float (1.97+):
let d = Duration::try_from_secs_f32(2.5)?;  // 2.5s — TryFromFloatSecsError
let d = Duration::try_from_secs_f64(0.001)?;

// Duration accessors:
d.as_secs();        // u64
d.as_millis();      // u128
d.as_micros();      // u128
d.as_nanos();       // u128
d.as_secs_f32();    // f32
d.as_secs_f64();    // f64
d.subsec_nanos();   // u32 — nanoseconds part
d.subsec_millis();  // u32
d.subsec_micros();  // u32

// Duration::ZERO, Duration::MAX, Duration::MIN (1.97+)
// Duration::is_zero(), Duration::is_infinite()

// SystemTime arithmetic:
let now = SystemTime::now();
let future = now + Duration::from_secs(3600);
let diff = now.duration_since(now)?;  // SystemTimeError if now is "before"
let elapsed = now.elapsed()?;         // duration since now

// UNIX_EPOCH constant:
let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
```

## Net

```rust
use std::net::{TcpListener, TcpStream, UdpSocket};

// TCP server
let listener = TcpListener::bind("127.0.0.1:8080")?;
for stream in listener.incoming() {
    let mut stream = stream?;
    std::io::Write::write_all(&mut stream, b"hello\n")?;
}

// TCP client
let mut stream = TcpStream::connect("127.0.0.1:8080")?;
std::io::Read::read_to_end(&mut stream, &mut Vec::new())?;

// UDP
let socket = UdpSocket::bind("127.0.0.1:8080")?;
socket.send_to(b"hello", "127.0.0.1:9090")?;
let mut buf = [0u8; 1024];
let (amt, addr) = socket.recv_from(&mut buf)?;

// Address types:
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

let v4: Ipv4Addr = "127.0.0.1".parse().unwrap();
let v6: Ipv6Addr = "::1".parse().unwrap();
let ip: IpAddr = v4.into();  // enum: V4(Ipv4Addr) | V6(Ipv6Addr)

let sa4 = SocketAddr::from((v4, 8080));  // or SocketAddrV4::new(v4, 8080)
let sa6 = SocketAddr::from((v6, 8080));

// ToSocketAddrs — DNS resolution trait:
use std::net::ToSocketAddrs;
let addrs: Vec<SocketAddr> = "example.com:80".to_socket_addrs()?.collect();

// Shutdown — half-close a TCP stream:
use std::net::Shutdown;
stream.shutdown(Shutdown::Write)?;  // half-close write side
stream.shutdown(Shutdown::Read)?;   // half-close read side
stream.shutdown(Shutdown::Both)?;   // full close

// Socket addresses are non-inheritable by child processes (CLOEXEC on Unix)

// hostname() — get system hostname (1.97+):
let name = std::net::hostname().unwrap();
```

## Memory Operations (std::mem)

```rust
use std::mem;

let size = mem::size_of::<i32>();        // 4
let align = mem::align_of::<i32>();      // 4

// Swap two values
let mut a = 1;
let mut b = 2;
mem::swap(&mut a, &mut b);  // a=2, b=1

// Replace
let mut x = 5;
let old = mem::replace(&mut x, 10);  // old=5, x=10

// Take — replace with Default
let mut opt = Some(42);
let val = mem::take(&mut opt);  // val=Some(42), opt=None

// Drop
mem::drop(x);  // drops immediately

// Transmute — reinterpret bits (unsafe)
let bits = unsafe { mem::transmute::<f32, u32>(3.14) };
```

## Marker Types (std::marker)

```rust
use std::marker::PhantomData;

// PhantomData — zero-sized type to indicate logical ownership
struct MyMap<K, V> {
    _phantom: PhantomData<(K, V)>,
    data: Vec<u8>,  // actual storage
}

// Send, Sync — auto traits for thread safety
// Sized — known at compile time
// Unpin — can be safely moved after pinning
```

## Pin (std::pin)

`Pin<P>` is a pointer wrapper that prevents the pointed-to value from being moved. Critical for self-referential types and async/await.

```rust
use std::pin::Pin;
use std::marker::Unpin;

// Most types implement Unpin — they can be moved even when pinned
// Self-referential types and async generators are !Unpin

// Pinning a value:
let mut boxed = Box::new(42);
let pinned: Pin<&mut i32> = Pin::new(&mut boxed);  // i32 is Unpin, so this is safe

// For !Unpin types, must use unsafe or pin on the heap:
// let pinned = Box::pin(my_future);  // safe pinning for !Unpin types

// Key points:
// - Pin<&mut T> where T: Unpin — can get &mut T freely (get_mut)
// - Pin<&mut T> where T: !Unpin — cannot safely get &mut T (would allow moving)
// - Box::pin / Box::into_pin — safe way to pin !Unpin types on the heap
// - Pin is essential for async/await — Futures are often !Unpin
```

## Range Types (std::range, std::ops)

```rust
// Legacy range types (in std::ops, also re-exported in std::range::legacy):
use std::ops::{Range, RangeInclusive, RangeFrom, RangeTo, RangeToInclusive, RangeFull};

let r: Range<i32> = 1..5;           // start..end (exclusive)
let ri: RangeInclusive<i32> = 1..=5; // start..=end (inclusive)
let rf: RangeFrom<i32> = 1..;       // start.. (to infinity)
let rt: RangeTo<i32> = ..5;         // ..end (exclusive)
let rti: RangeToInclusive<i32> = ..=5; // ..=end (inclusive)
let rfull: RangeFull = ..;          // .. (full range)

// New range types (std::range — meant to replace legacy in a future edition):
use std::range::{Range as NewRange, RangeFrom as NewRangeFrom,
                 RangeInclusive as NewRangeInclusive, RangeToInclusive as NewRangeToInclusive};

// Ranges are iterators:
let v: Vec<i32> = (1..5).collect();       // [1, 2, 3, 4]
let v: Vec<i32> = (1..=5).collect();      // [1, 2, 3, 4, 5]

// Ranges implement RangeBounds for slicing:
let arr = [0, 1, 2, 3, 4];
assert_eq!(&arr[1..3], &[1, 2]);
assert_eq!(&arr[..=2], &[0, 1, 2]);
assert_eq!(&arr[2..], &[2, 3, 4]);

// RangeBounds trait methods:
use std::ops::RangeBounds;
fn print_range<R: RangeBounds<usize>>(r: &R, arr: &[i32]) {
    let start = r.start().unwrap_or(&0);
    let end = r.end().unwrap_or(&arr.len());
    println!("{:?}", &arr[*start..*end]);
}
```

## UnsafeCell (std::cell)

```rust
use std::cell::UnsafeCell;

// UnsafeCell is the core primitive for interior mutability
// Cell and RefCell are built on top of it
// Using it directly is unsafe — no borrow checking at all

let cell = UnsafeCell::new(5);
unsafe {
    let val = cell.get();
    *val += 1;  // must ensure no aliasing violations
}
```

## Type Layout (std::mem)

```rust
use std::mem;

// Size and alignment:
assert_eq!(mem::size_of::<i32>(), 4);
assert_eq!(mem::align_of::<i32>(), 4);
assert_eq!(mem::size_of::<u8>(), 1);
assert_eq!(mem::align_of::<u8>(), 1);

// size_of_val / align_of_val for DSTs:
let s: &str = "hello";
assert_eq!(mem::size_of_val(s), 5);

// repr attributes control layout:
// #[repr(Rust)] — default, compiler may reorder fields
// #[repr(C)] — C-compatible layout, fields in declaration order
// #[repr(packed)] — no padding between fields
// #[repr(align(N))] — force alignment to N bytes
// #[repr(transparent)] — same layout as single field
```

## I/O Details (std::io)

### io::Error and ErrorKind

```rust
use std::io::{self, ErrorKind};

// ErrorKind variants (common):
// NotFound, PermissionDenied, ConnectionRefused, ConnectionReset,
// ConnectionAborted, NotConnected, AddrInUse, AddrNotAvailable,
// BrokenPipe, AlreadyExists, WouldBlock, InvalidInput, InvalidData,
// TimedOut, WriteZero, Interrupted, Unsupported, UnexpectedEof,
// OutOfMemory, Other, Uncategorized

// Create errors:
let err = io::Error::new(ErrorKind::NotFound, "file missing");
let err = io::Error::from(ErrorKind::PermissionDenied);
let err = io::Error::from_raw_os_error(2);  // errno 2 = ENOENT
let err = io::Error::last_os_error();

// Inspect errors:
match err.kind() {
    ErrorKind::NotFound => println!("not found"),
    ErrorKind::PermissionDenied => println!("permission denied"),
    _ => println!("other: {}", err),
}

// Get raw OS error number:
if let Some(code) = err.raw_os_error() {
    println!("errno: {}", code);
}
```

### Cursor — In-Memory Reader/Writer

```rust
use std::io::{Cursor, Read, Write, Seek, SeekFrom};

// Cursor wraps an in-memory buffer and implements Read + Write + Seek
let mut cursor = Cursor::new(vec![1u8, 2, 3, 4, 5]);

// Read:
let mut buf = [0u8; 3];
cursor.read_exact(&mut buf).unwrap();  // [1, 2, 3]

// Seek:
cursor.seek(SeekFrom::Start(0)).unwrap();
cursor.seek(SeekFrom::End(-2)).unwrap();  // 2 bytes from end
cursor.seek(SeekFrom::Current(1)).unwrap();  // forward 1

// Write:
let mut cursor = Cursor::new(Vec::new());
cursor.write_all(b"hello").unwrap();
let data = cursor.into_inner();  // vec![104, 101, 108, 108, 111]

// Cursor over a slice:
let mut cursor = Cursor::new("hello world");
let mut s = String::new();
cursor.read_to_string(&mut s).unwrap();
```

### Chain — Combining Readers

```rust
use std::io::{Chain, Read};

// Chain two readers into one
let part1 = b"hello ".as_slice();
let part2 = b"world".as_slice();
let mut chain = part1.chain(part2);

let mut s = String::new();
chain.read_to_string(&mut s).unwrap();
assert_eq!(s, "hello world");
```

### Other io Types

```rust
use std::io;

// Empty — reader that returns EOF immediately
let mut empty = io::empty();
let mut buf = [0u8; 10];
assert_eq!(empty.read(&mut buf).unwrap(), 0);

// Repeat — reader that repeats a byte forever
let mut repeat = io::repeat(0xAB);
repeat.read_exact(&mut buf[..3]).unwrap();  // [0xAB, 0xAB, 0xAB]

// Sink — writer that discards all data
let mut sink = io::sink();
sink.write_all(b"discarded").unwrap();  // no-op

// Take — limit number of bytes read
let mut reader = io::repeat(0xFF).take(5);  // only read 5 bytes
reader.read_exact(&mut buf[..5]).unwrap();

// io::copy — copy from reader to writer
let mut reader = b"data".as_slice();
let mut writer = Vec::new();
io::copy(&mut reader, &mut writer).unwrap();

// io::pipe — create a pipe (PipeReader + PipeWriter)
let (mut reader, mut writer) = io::pipe();
writer.write_all(b"hello").unwrap();
drop(writer);  // close write end
let mut s = String::new();
reader.read_to_string(&mut s).unwrap();

// IsTerminal — check if a stream is a terminal
use std::io::IsTerminal;
if io::stdin().is_terminal() {
    println!("Running in a terminal");
}
```

### I/O Safety

```rust
// Rust enforces I/O safety similar to memory safety:
// - OwnedFd / OwnedHandle — exclusively owned file descriptor
// - BorrowedFd / BorrowedHandle — borrowed file descriptor
// - Safe code cannot act on or close FDs it doesn't own/borrow

// On Unix:
use std::os::fd::{AsRawFd, AsFd, BorrowedFd, OwnedFd};
let file = std::fs::File::open("foo.txt")?;
let fd: i32 = file.as_raw_fd();  // raw fd (unsafe to use directly)
let borrowed: BorrowedFd<'_> = file.as_fd();  // safe borrowed fd

// On Windows:
use std::os::windows::io::{AsRawHandle, AsHandle, BorrowedHandle, OwnedHandle};
```

### Stdin/Stdout Locking

```rust
use std::io::{self, BufRead};

// Lock stdin for faster buffered reading (avoids mutex per read)
let stdin = io::stdin();
let locked = stdin.lock();
for line in locked.lines() {
    println!("{}", line?);
}

// Lock stdout for faster writing
let stdout = io::stdout();
let mut lock = stdout.lock();
writeln!(lock, "fast output")?;
```

## Process Details (std::process)

```rust
use std::process::{self, Command, Stdio, ExitStatus, ExitCode};

// exit — terminate immediately with exit code
process::exit(0);   // success
process::exit(1);   // failure
// No destructors are run — use with caution

// abort — terminate with SIGABRT (creates core dump on Unix)
process::abort();

// Termination trait — return type of main()
// fn main() -> impl Termination
// ExitCode::SUCCESS, ExitCode::FAILURE
// i32 (via ExitCode) also implements Termination

// Command methods:
let output = Command::new("ls")
    .arg("-la")
    .args(["--color", "always"])  // multiple args
    .current_dir("/tmp")           // set working directory
    .env("MY_VAR", "value")        // set env var
    .env_clear()                   // clear all env vars
    .stdin(Stdio::null())          // /dev/null stdin
    .stdout(Stdio::piped())        // capture stdout
    .stderr(Stdio::piped())        // capture stderr
    .output()?;                    // run and capture

// Check exit status:
if output.status.success() {
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
} else {
    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}

// ExitStatus methods:
let code = output.status.code();     // Option<i32> (None on Unix signal)
let success = output.status.success();

// Windows: raw_arg for custom command line escaping
// Command::new("cmd").raw_arg("/c echo hello");
```

## Compiler Hints (std::hint)

```rust
use std::hint;

// black_box — prevent compiler optimizations (for benchmarking)
let x = hint::black_box(42);  // compiler won't optimize away x

// spin_loop — hint to CPU that we're in a spin-wait loop
// (uses PAUSE on x86, YIELD on ARM)
while !ready.load(std::sync::atomic::Ordering::Relaxed) {
    std::hint::spin_loop();
}

// likely / unlikely — branch prediction hints
if hint::likely(condition) {
    // hot path
} else {
    // cold path
}

// cold_path — mark a branch as cold (for code layout)
if hint::cold_path(condition) { /* rarely taken */ }

// assert_unchecked — assert without runtime check (unsafe)
// Compiler assumes condition is true for optimization
unsafe { hint::assert_unchecked(x >= 0); }

// unreachable_unchecked — mark code as unreachable (unsafe, UB if reached)
unsafe { hint::unreachable_unchecked(); }

// select_unpredictable — branchless select (hint to avoid branch prediction)
let val = hint::select_unpredictable(condition, true_val, false_val);

// prefetch — prefetch data into cache (x86/ARM specific)
hint::prefetch_read(ptr, std::hint::Locality::Low);
```

## OS-Specific Modules (std::os)

```rust
// std::os::unix — Unix-specific functionality
//   std::os::unix::fs — symlink, permissions, ext attributes
//   std::os::unix::io — AsRawFd, FromRawFd, OwnedFd, BorrowedFd
//   std::os::unix::net — UnixStream, UnixListener, UnixDatagram
//   std::os::unix::process — CommandExt (before_exec, exec)

use std::os::unix::fs::PermissionsExt;
let perms = std::fs::metadata("file")?.permissions();
println!("mode: {:o}", perms.mode());  // e.g., 644
std::fs::set_permissions("file", std::fs::Permissions::from_mode(0o755))?;

// Unix domain sockets:
use std::os::unix::net::UnixListener;
let listener = UnixListener::bind("/tmp/socket")?;

// std::os::windows — Windows-specific functionality
//   std::os::windows::fs — symlink_file, symlink_dir
//   std::os::windows::io — AsRawHandle, OwnedHandle, BorrowedHandle
//   std::os::windows::process — CommandExt (creation_flags)

use std::os::windows::process::CommandExt;
Command::new("cmd")
    .creation_flags(0x00000008)  // DETACHED_PROCESS
    .spawn()?;

// std::os::fd — cross-platform file descriptor (Unix + WASI)
//   OwnedFd, BorrowedFd, AsFd, AsRawFd
// std::os::raw — C type aliases (c_int, c_long, c_char, etc.)
// std::os::linux, std::os::darwin, std::os::wasi, std::os::wasip2
```

## Prelude (std::prelude)

The prelude is automatically imported into every Rust program. It includes:

```rust
// Traits:
//   Clone, Copy, PartialEq, PartialOrd, Eq, Ord
//   AsRef, AsMut, Into, From
//   Default, ToOwned, ToString
//   Iterator, Extend, IntoIterator, DoubleEndedIterator, ExactSizeIterator
//   Drop, Fn, FnMut, FnOnce
//   Send, Sync, Unpin, Sized

// Types:
//   Option::{self, Some, None}
//   Result::{self, Ok, Err}
//   Box, String, Vec
//   &'static str

// Functions:
//   drop, size_of, size_of_val, align_of, align_of_val

// Macros:
//   assert, assert_eq, assert_ne, cfg, concat, dbg, debug_assert, ...
//   println, eprintln, print, eprint, format, format_args, ...
//   panic, todo, unimplemented, unreachable, vec, write, writeln
//   include, include_bytes, include_str, env, option_env
//   file, line, column, module_path, stringify, matches

// Edition preludes:
//   prelude::v1 — Rust 2021 and earlier
//   prelude::rust_2024 — Rust 2024 edition (adds Future, IntoFuture)
```

## Random (std::random)

```rust
use std::random::{random, RandomSource, Distribution, DefaultRandomSource};

// random() — generate a random value (uses DefaultRandomSource):
let n: i32 = random();
let b: bool = random();
let f: f64 = random();

// RandomSource — trait for custom random sources:
struct MyRng;
impl RandomSource for MyRng {
    fn random_bytes(&mut self, buf: &mut [u8]) {
        // fill buf with random bytes
    }
}

// Distribution — trait for types that can be randomly sampled:
// fn sample<R: RandomSource>(&self, rng: &mut R) -> Self;

// DefaultRandomSource — system entropy source
// (uses getrandom on Unix, RtlGenRandom on Windows)
```

## Marker Types Extras (std::marker)

```rust
use std::marker::{PhantomPinned, Tuple, ConstParamTy};

// PhantomPinned — marker to make a type !Unpin:
struct MyFuture {
    _pin: PhantomPinned,  // prevents moving after pinning
}
// Now Pin<&mut MyFuture> cannot be safely converted to &mut MyFuture

// Tuple — marker trait implemented for all tuples
// Used for trait bounds on tuple types: fn f<T: Tuple>(t: T)

// ConstParamTy — allows a type to be used as a const generic parameter
#[derive(ConstParamTy, PartialEq, Eq)]
struct MyId(u32);
// Now: fn foo<const ID: MyId>() is possible

// CoercePointee — derive macro for smart pointer coercion (unsafe, nightly)

// Additional marker traits (mostly auto-implemented, nightly):
// Freeze — &T is immutable (no interior mutability via UnsafeCell)
// Unsize — compiler-implemented: allows unsizing coercion (&[T; N] → &[T])
// StructuralPartialEq — marker for pattern-match-based equality (derive)
// UnsafeUnpin — unsafe manual impl of Unpin (for custom Pin types)
// Destruct — marker for types that can be dropped (replaces Drop bound)
// FnPtr — marker for function pointer types (fn() → impl FnPtr)
// DiscriminantKind — compiler-implemented: provides enum discriminant info
// MetaSized / PointeeSized — sized metadata for fat pointers (nightly)
// Variance — compiler-implemented: controls subtyping relationships
// Reborrow — trait for reborrowing &mut &T → &T (nightly)
// CoerceShared — compiler trait for shared reference coercion (nightly)
```

## Memory Extras (std::mem)

```rust
use std::mem;

// TransmuteFrom — safe transmute trait (nightly):
// trait TransmuteFrom<S> {} — auto-implemented when safe
// If T: TransmuteFrom<S>, then transmute is safe (no unsafe needed)

// mem::type_info — type information at runtime (nightly):
// TypeKind enum: Abi (ABI-stable), Generic (generic type)
// mem::type_info::Type::of::<T>() — get type info

// ZeroablePrimitive (std::num) — marker for primitives that can be zero:
// Implemented for all numeric primitives
// Used by NonZero<T> to guarantee non-zero-ness
// trait ZeroablePrimitive: Sized + Copy {}
// All int/float types implement this; NonZero<T> requires it

// Additional mem functions:
// mem::forget(t) — take ownership without running Drop (leaks)
// mem::drop(t) — same as drop(t), runs Drop
// mem::replace(&mut dest, src) — replace value, return old
// mem::take(&mut dest) — replace with Default::default(), return old
// mem::swap(&mut a, &mut b) — swap two values
// mem::transmute_copy(src) — bitwise copy between same-sized types
// mem::size_of_val_val(v) — size of value (not just type)
// mem::align_of_val_val(v) — alignment of value
// mem::needs_drop::<T>() — whether T needs Drop (false for Copy types)
// mem::variant_count::<T>() — number of enum variants (nightly)
// mem::variant_index::<T>() — index of active variant (nightly)
// mem::assumed_init — hint that value is initialized (nightly)
```

## Standard Macros Reference

```rust
// Assertion macros:
assert!(condition);
assert_eq!(a, b);              // equality
assert_ne!(a, b);              // inequality
assert_matches!(val, pattern); // pattern match (1.97+)
debug_assert!(condition);      // only in debug builds
debug_assert_eq!(a, b);
debug_assert_ne!(a, b);
debug_assert_matches!(val, pattern); // debug-only pattern match

// Debugging:
dbg!(expr);             // prints "expr = value" to stderr
// e.g. dbg!(vec![1, 2, 3]) → "[src/main.rs:1] vec![1, 2, 3] = [1, 2, 3]"

// Compilation:
compile_error!("message");     // produce a compile error
concat!("a", "b", "c");        // "abc" (const string concatenation)
concat_bytes!(b"a", b"b");     // b"ab" (const byte concatenation, 1.97+)
env!("VAR");                   // compile-time env var (panics if unset)
option_env!("VAR");            // compile-time env var (Option)
include!("file.rs");           // include file contents as code
include_str!("file.txt");      // include file as &str
include_bytes!("file.bin");    // include file as &[u8]

// Source info:
file!();        // "src/main.rs"
line!();        // 42
column!();      // 8
module_path!(); // "my_crate::my_module"
stringify!(expr); // "expr" (token stringification)

// Utility:
todo!();              // unimplemented!() — panics with "not yet implemented"
unimplemented!();     // same as todo!()
unreachable!();       // panics with "internal error: entered unreachable code"
matches!(val, pat);   // true if val matches pattern

// Collection constructors:
vec![1, 2, 3];                    // Vec
hash_map!{"key" => "value"};     // HashMap (1.97+)

// Formatting:
format!("Hello {}", "world");     // String
format_args!("Hello {}", "world"); // Arguments (zero-alloc)
write!(buf, "text");              // write to fmt::Write
writeln!(buf, "text");            // write with newline
print!("text");                   // stdout
println!("text");                 // stdout with newline
eprint!("text");                  // stderr
eprintln!("text");                // stderr with newline

// Threading:
thread_local! { static FOO: Cell<i32> = Cell::new(0); }

// Memory:
std::mem::offset_of!(Struct, field); // byte offset of field (1.77+)

// Pinning:
std::pin::pin!(future); // pin a future on the stack (1.68+)

// Configuration:
cfg!(target_os = "linux");   // true/false at compile time
cfg_select! {                // multi-cfg match (nightly)
    target_os = "linux" => { /* linux */ },
    target_os = "windows" => { /* windows */ },
    _ => { /* other */ },
}

// Macro debugging:
trace_macros!(true);  // log macro expansions (nightly)
log_syntax!(tokens);  // print tokens to stdout (nightly)
```

## Automatic Differentiation (std::autodiff)

```rust
// std::autodiff provides compile-time automatic differentiation (nightly)
// Generates derivative functions from regular Rust functions

// #[autodiff_reverse] — reverse-mode (backpropagation) differentiation:
#[autodiff_reverse(df, Const, Active, Active)]
fn f(x: f64, y: f64) -> f64 {
    x * y + x.sin()
}
// df is auto-generated: fn df(x: f64, dx: f64, y: f64, dy: f64) -> (f64, f64, f64)
// Const = treat as constant, Active = differentiate w.r.t. this arg
// Duplicated = gradient accumulation mode

// #[autodiff_forward] — forward-mode differentiation:
#[autodiff_forward(df, Dual, Dual)]
fn g(x: f64, y: f64) -> f64 {
    x * x + y
}
// Dual = dual number (value + derivative)

// Higher-order derivatives: apply forward over reverse
// Works with trait methods and impl blocks
// Limitations: no dyn Trait, requires lto="fat", debug mode may fail
```

## Field Reflection (std::field)

```rust
// std::field provides field projection for structs, unions, enums (nightly)
// Allows accessing fields by type-level descriptors

use std::field::{Field, FieldRepresentingType, field_of};

// Field trait — represents a field of a struct/union/enum
// field_of! macro — get a field projector
// FieldRepresentingType — type that represents a field's identity
// Used for compile-time field access and reflection
```

## Process Termination (std::process::Termination)

```rust
use std::process::Termination;

// Termination trait — allows custom types as main() return value:
// trait Termination {
//     fn report(self) -> ExitCode;
// }
// Implemented for: (), ExitCode, Result<T, E> where E: Debug
// Custom impl allows any type to be returned from main():

struct MyExit { code: i32 }
impl Termination for MyExit {
    fn report(self) -> std::process::ExitCode {
        if self.code == 0 {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::from(self.code as u8)
        }
    }
}

fn main() -> MyExit {
    MyExit { code: 0 }
}
```

## Pointer Extras (std::ptr::Pointee)

```rust
// ptr::Pointee — trait for types with metadata (fat pointers)
// All DSTs implement Pointee, which defines the metadata type
// For [T]: metadata is usize (length)
// For dyn Trait: metadata is DynMetadata (vtable pointer)
// For sized types: metadata is () (thin pointer)

// ptr::metadata(ptr) — extract metadata from a fat pointer
// ptr::from_raw_parts(data_ptr, metadata) — reconstruct fat pointer (nightly)
```

## Marker Variance Types (std::marker)

```rust
use std::marker::{
    PhantomData, PhantomCovariant, PhantomContravariant, PhantomInvariant,
    PhantomCovariantLifetime, PhantomContravariantLifetime, PhantomInvariantLifetime,
};

// PhantomCovariant<T> — expresses covariance: &'a T <: &'b T if 'a: 'b
// PhantomContravariant<T> — expresses contravariance (function parameters)
// PhantomInvariant<T> — expresses invariance (neither covariant nor contravariant)
// PhantomCovariantLifetime<'a> — covariance over a lifetime only
// PhantomContravariantLifetime<'a> — contravariance over a lifetime
// PhantomInvariantLifetime<'a> — invariance over a lifetime

// These replace PhantomData for more precise variance control (nightly):
struct MyCovariant<T> { _marker: PhantomCovariant<T> }
struct MyInvariant<T> { _marker: PhantomInvariant<T> }
```

## Memory Extras — Additional Types (std::mem)

```rust
use std::mem;

// Alignment — represents alignment in bytes (nightly):
// let align = Alignment::of::<u64>();  // Alignment(8)
// align.as_usize() → usize
// Alignment::new(n) → Option<Alignment> (must be power of 2)

// Assume — assumptions for safe transmute (nightly):
// mem::Assume { alignment: true, lifetimes: true, validity: true, safety: true }
// Used with mem::transmute_assume::<Src, Dst>(val, assume)

// Discriminant — opaque token identifying an enum variant:
let d1 = mem::discriminant(&Some(1));
let d2 = mem::discriminant(&Some(2));
assert_eq!(d1, d2);  // same variant — Some
let d3 = mem::discriminant(&None::<i32>);
assert_ne!(d1, d3);  // different variant

// DropGuard — RAII guard that runs a closure on drop (nightly):
// let guard = mem::DropGuard::new(|| println!("cleaned up"));

// MaybeDangling — wrapper indicating a value may be dangling (nightly):
// Used for self-referential types where the pointer may temporarily dangle
// mem::MaybeDangling<T> — transparent wrapper, no runtime cost
```

## I/O Adapters and Utilities (std::io)

```rust
use std::io::{self, Read, Write};

// Cursor — in-memory reader/writer over a buffer:
use std::io::Cursor;
let mut cursor = Cursor::new(vec![1, 2, 3, 4, 5]);
let mut buf = [0u8; 3];
cursor.read_exact(&mut buf)?;  // reads [1, 2, 3]
cursor.seek(SeekFrom::Start(0))?;  // reset position
// Also: Cursor::new(&[u8]) for reading from a slice

// Chain — combine two readers sequentially:
let r1 = &[1, 2, 3][..];
let r2 = &[4, 5, 6][..];
let mut chained = r1.chain(r2);
// reads 1,2,3,4,5,6 in order

// Take — limit number of bytes read:
let mut limited = (&[1, 2, 3, 4, 5][..]).take(3);
// only reads first 3 bytes

// Empty / Sink — null reader/writer:
let mut input = io::empty();  // always returns 0 bytes on read
let mut output = io::sink();  // discards all writes

// Repeat — infinite reader of a single byte:
let mut r = io::repeat(0xAB);  // reads 0xAB forever

// Bytes — iterate over bytes of a reader:
for byte in (&[1, 2, 3][..]).bytes() {
    println!("{}", byte.unwrap());
}

// Split — split reader on a byte delimiter:
let mut split = (&b"a,b,c"[..]).split(b',');
// yields: "a", "b", "c"

// Lines — iterate over lines of a BufRead:
// (already covered in I/O section above)

// BorrowedBuf / BorrowedCursor — borrowed buffer for vectored I/O (nightly):
// io::BorrowedBuf::new(&mut [u8]) — wraps a borrowed buffer
// io::BorrowedCursor — cursor into a BorrowedBuf for writing

// IoSlice / IoSliceMut — vectored I/O scatter/gather:
use std::io::{IoSlice, IoSliceMut};
let bufs = &mut [IoSliceMut::new(&mut [0; 10]), IoSliceMut::new(&mut [0; 20])];
// reader.read_vectored(bufs) — reads into multiple buffers at once

// PipeReader / PipeWriter — anonymous pipe I/O (1.69+):
use std::io::{PipeReader, PipeWriter};
// let (reader, writer) = pipe()?;  // create anonymous pipe
// writer.write_all(b"hello")?;
// reader reads from the other end

// WriterPanicked — error from a writer that panicked (nightly):
// Returned when a BufWriter's inner writer panicked during flush

// StdinLock / StdoutLock / StderrLock — explicit lock handles:
let stdin = io::stdin();
let lock = stdin.lock();  // holds lock for duration of lock
// More efficient than locking per read_line call
```

## File System Extras (std::fs)

```rust
use std::fs::{self, FileTimes, Dir};

// FileTimes — set file access/modification times (1.75+):
use std::time::SystemTime;
let times = FileTimes::new()
    .set_accessed(SystemTime::now())
    .set_modified(SystemTime::now())
    .set_created(SystemTime::now());  // Windows/macOS only
fs::set_times("file.txt", times)?;

// Dir — directory iterator over raw file descriptors (nightly):
// More efficient than ReadDir for bulk directory scanning
// Dir::open(path)? → Dir
// dir.next() → Option<io::Result<Entry>>

// Additional fs functions:
// fs::exists(path) → bool (1.86+, previously nightly)
// fs::try_exists(path) → io::Result<bool> (1.85+)
// fs::set_permissions_nofollow(path, perms) — set perms without following symlinks
// fs::set_times_nofollow(path, times) — set times without following symlinks
// fs::soft_link(src, dst) — alias for symlink on Windows, hard_link on Unix
// fs::symlink_metadata(path) — get metadata without following symlinks
```

## Path Extras (std::path)

```rust
use std::path::{Path, PathBuf};

// Ancestors — iterator over parent directories:
let path = Path::new("/a/b/c/d.txt");
for ancestor in path.ancestors() {
    println!("{}", ancestor.display());
}
// /a/b/c/d.txt, /a/b/c, /a/b, /a, /

// path.normalize() — normalize path (resolve . and ..) (nightly):
// let normalized = Path::new("/a/./b/../c").normalize();
// → "/a/c"
// Returns Result<PathBuf, NormalizeError> if path can't be normalized

// path.strip_prefix(prefix) → Result<&Path, StripPrefixError>
// Path::new("/a/b/c").strip_prefix("/a") → Ok("/b/c")
```

// ─────────────────────────────────────────────────────────
// std::random — Random Value Generation (nightly, #130703)
// ─────────────────────────────────────────────────────────

// Generates a random value using the default random source.
// Available since Rust 1.97.0 (nightly only).
#![feature(random)]
use std::random::random;

// random() works with any type that implements Distribution<T>
// RangeFull (..) implements Distribution for all integer/bool types:
let bits: u128 = random(..);       // random u128
let n: u32 = random(..);           // random u32
let b: bool = random(..);          // random bool

// Can also use ranges:
// let small: u8 = random(0..10);  // random u8 in 0..10 (nightly)

// Trait: Distribution<T> — samples a random value from a distribution
// pub trait Distribution<T> {
//     fn sample(&self, source: &mut (impl RandomSource + ?Sized)) -> T;
// }
// Implementors: RangeFull (..) for all numeric types and bool,
//               &DT where DT: Distribution<T>

// Trait: RandomSource — a source of randomness
// pub trait RandomSource {
//     fn fill_bytes(&mut self, bytes: &mut [u8]);
// }
// Note: calling fill_bytes multiple times is NOT equivalent to
// calling it once with a larger buffer.

// Struct: DefaultRandomSource — the default OS-backed random source
// Uses getrandom (Linux), /dev/urandom, ProcessPrng (Windows),
// arc4random_buf (BSD/macOS), or platform-specific entropy sources.
// Implements: RandomSource, Clone, Copy, Default, Debug
// Send + Sync — safe to share across threads

// Example: generating a v4 UUID
#![feature(random)]
use std::random::random;
let bits: u128 = random(..);
let g1 = (bits >> 96) as u32;
let g2 = (bits >> 80) as u16;
let g3 = (0x4000 | (bits >> 64) & 0x0fff) as u16;
let g4 = (0x8000 | (bits >> 48) & 0x3fff) as u16;
let g5 = (bits & 0xffffffffffff) as u64;
let uuid = format!("{g1:08x}-{g2:04x}-{g3:04x}-{g4:04x}-{g5:012x}");

// ─────────────────────────────────────────────────────────
// std::hint — Compiler Hints for Optimization
// ─────────────────────────────────────────────────────────

use std::hint;

// black_box(value) — prevents compiler optimizations across the call
// Essential for benchmarking. Forces the compiler to treat the value
// as unpredictable, preventing dead-code elimination and constant folding.
use std::hint::black_box;

fn increment(x: u8) -> u8 { x + 1 }

// Without black_box, compiler may eliminate the pure function call:
let _ = increment(5);

// With black_box, compiler is forced to compute the result:
let _ = black_box(increment(black_box(5)));

// Benchmarking pattern:
pub fn benchmark() {
    let haystack = vec!["abc", "def", "ghi", "jkl", "mno"];
    let needle = "ghi";
    for _ in 0..10 {
        black_box(contains(
            black_box(&haystack),
            black_box(needle),
        ));
    }
}

// spin_loop() — hint to the CPU that we're in a busy-wait loop
// Lowers power consumption during spin-waiting. Used in lock-free code.
use std::sync::atomic::{AtomicBool, Ordering};
let ready = AtomicBool::new(false);
// In a spin-wait loop:
while !ready.load(Ordering::Acquire) {
    std::hint::spin_loop();
}

// assert_unchecked(cond) — assert without runtime checking (unsafe)
// Tells the compiler that cond is always true. Enables optimizations.
// If cond is false at runtime → undefined behavior.
// # Safety: cond MUST always be true.
unsafe {
    let x: i32 = 5;
    hint::assert_unchecked(x > 0);  // compiler can assume x > 0
}

// unreachable_unchecked() — marks code as unreachable (unsafe)
// If reached → undefined behavior. More efficient than unreachable!().
// # Safety: This code path must never be executed.
unsafe {
    let x: u8 = 255;
    match x {
        0..=200 => {},
        201..=255 => {},
        _ => unsafe { hint::unreachable_unchecked() },
    }
}

// likely(cond) / unlikely(cond) — branch prediction hints (nightly)
// Tells the compiler which branch is more likely to be taken.
// These are no-ops at runtime but may influence code generation.
#![feature(likely_unlikely)]
if hint::likely(data.is_empty()) {
    // fast path — compiler places this first in instruction cache
}

if hint::unlikely(error_condition) {
    // cold path — compiler places this far away
}

// cold_path() — marks the current code path as cold (nightly)
// Influences inlining and code layout decisions.

// select_unpredictable(condition, true_val, false_val) (nightly)
// Tells the compiler that the condition is unpredictable,
// preventing branch prediction optimizations.

// must_use(expr) — suppresses unused_must_use warnings (nightly)
// Useful when you intentionally discard a #[must_use] result.
let _ = hint::must_use(must_use_function());

// prefetch functions (nightly) — CPU prefetch hints
// hint::prefetch_read(ptr, locality)      — prefetch data for reading
// hint::prefetch_write(ptr, locality)     — prefetch data for writing
// hint::prefetch_read_instruction(ptr)    — prefetch instruction for reading
// hint::prefetch_read_non_temporal(ptr)   — non-temporal read (no cache pollution)
// hint::prefetch_write_non_temporal(ptr)  — non-temporal write
// Locality enum: specifies temporal locality hint (0 = no temporal locality, 3 = high)

// ─────────────────────────────────────────────────────────
// std::range — Replacement Range Types (nightly)
// ─────────────────────────────────────────────────────────
// New range types meant to replace legacy Range, RangeInclusive,
// RangeToInclusive, and RangeFrom in a future edition.
// Key difference: these types implement From for range syntax.
#![feature(range)]
use core::range::{Range, RangeFrom, RangeInclusive, RangeToInclusive};

let arr = [0, 1, 2, 3, 4];
assert_eq!(arr[Range::from(1..3)], [1, 2]);
assert_eq!(arr[RangeFrom::from(1..)], [1, 2, 3, 4]);
assert_eq!(arr[RangeInclusive::from(1..=3)], [1, 2, 3]);
assert_eq!(arr[RangeToInclusive::from(..=3)], [0, 1, 2, 3]);

// Additional structs: RangeIter, RangeInclusiveIter, RangeFromIter
// — separate iterator types for each range type

// ─────────────────────────────────────────────────────────
// Macros — Additional Standard Library Macros
// ─────────────────────────────────────────────────────────

// assert_matches! — assert a value matches a pattern (nightly):
#![feature(assert_matches)]
use std::assert_matches::assert_matches;
let x = Some(42);
assert_matches!(x, Some(n) if n > 0);
// Also: debug_assert_matches! (nightly)

// concat_bytes! — concatenate byte string literals (nightly):
#![feature(concat_bytes)]
let bytes: [u8; 6] = concat_bytes!(b"hello", b"!");
// → [104, 101, 108, 108, 111, 33]

// mem::offset_of! — get byte offset of a field (stable since 1.77):
use std::mem::offset_of;
#[repr(C)]
struct Foo { a: u8, b: u32, c: u8 }
assert_eq!(offset_of!(Foo, a), 0);
assert_eq!(offset_of!(Foo, b), 4);  // aligned to 4
assert_eq!(offset_of!(Foo, c), 8);

// iter::iter! — create iterator from expressions (nightly):
#![feature(iter_macro)]
// let it = iter!(1, 2, 3, 4);  // equivalent to [1,2,3,4].into_iter()

// pat::pattern_type! — pattern type macro (nightly):
// Used for pattern types in type ascription

// unsafe_binder::wrap_binder! / unwrap_binder! (nightly):
// Macros for working with unsafe binder types
// wrap_binder!(type_expr) — wraps a type in an unsafe binder
// unwrap_binder!(type_expr) — unwraps an unsafe binder type

// ─────────────────────────────────────────────────────────
// fmt::from_fn — Closure-based Debug/Display (stable since 1.97)
// ─────────────────────────────────────────────────────────
// Creates a type whose Debug AND Display impls are forwarded to a closure.
// Useful for ad-hoc formatting without defining a wrapper struct.
use std::fmt;

let value = 'a';
assert_eq!(format!("{}", value), "a");
assert_eq!(format!("{:?}", value), "'a'");

let wrapped = fmt::from_fn(|f| write!(f, "{value:?}"));
assert_eq!(format!("{}", wrapped), "'a'");   // Display → closure
assert_eq!(format!("{:?}", wrapped), "'a'"); // Debug → same closure

// ─────────────────────────────────────────────────────────
// process — Additional Process Functions
// ─────────────────────────────────────────────────────────

// set_exit_code(code) — set exit code without exiting immediately
// The process will exit with this code when main() returns normally.
// Unlike exit(), destructors will still run.
use std::process;
process::set_exit_code(42);  // exit with 42 when main returns

// abort_immediate() — abort without running destructors (nightly)
// Even more immediate than abort() — no cleanup at all.

// ─────────────────────────────────────────────────────────
// thread — Additional Thread Functions
// ─────────────────────────────────────────────────────────

// available_parallelism() → io::Result<NonZero<usize>>
// Returns the number of usable CPUs for parallelism.
// Recomputed each call (not cached) — don't call from hot loops.
// May undercount/overcount on Windows (>64 CPUs), Linux (cgroup quotas),
// VMs with CPU limits, etc.
use std::thread;
let count = thread::available_parallelism().unwrap().get();
assert!(count >= 1);

// sleep_until(deadline: Instant) — sleep until a specific time (nightly)
// More precise than sleep(duration) for periodic tasks.
#![feature(thread_sleep_until)]
use std::time::{Duration, Instant};

// Game loop at 60 FPS:
let max_fps = 60.0;
let frame_time = Duration::from_secs_f32(1.0 / max_fps);
let mut next_frame = Instant::now();
loop {
    thread::sleep_until(next_frame);
    next_frame += frame_time;
    // update(); render();
}

// Rate-limited retry pattern:
let deadline = Instant::now() + Duration::from_secs(30);
let delay = Duration::from_millis(250);
let mut next_attempt = Instant::now();
loop {
    if Instant::now() > deadline { break Err(()); }
    // if api_call().is_ready() { break Ok(data); }
    next_attempt = deadline.min(next_attempt + delay);
    thread::sleep_until(next_attempt);
}

// ─────────────────────────────────────────────────────────
// ascii::Char — ASCII-only Character Type (nightly)
// ─────────────────────────────────────────────────────────
// A type for ASCII-only characters (0x00–0x7F).
// More efficient than char for ASCII-only operations.
// Implements Into<char>, From<u8>, Display, Debug, etc.
#![feature(ascii_char)]
use std::ascii::Char;

let c: Char = Char::from_u8(b'A').unwrap();
let as_char: char = c.into();  // 'A'
let as_byte: u8 = c.to_byte(); // 65
// Char::from_u8(byte) → Option<Char> (None if > 127)

// ─────────────────────────────────────────────────────────
// slice::range / slice::try_range — Safe Range Extraction (nightly)
// ─────────────────────────────────────────────────────────
// Extract a range from a slice with bounds checking.
// slice::range(range, slice) → Range<usize>
// Returns the resolved range within the slice bounds.
#![feature(slice_range)]
use std::slice;

let data = [1, 2, 3, 4, 5];
let r = slice::range(1..4, &data);  // 1..4
assert_eq!(&data[r], [2, 3, 4]);

// try_range — returns Result instead of panicking:
#![feature(slice_try_range)]
let r2 = slice::try_range(1..10, &data);
// → Err(TrySliceRangeError) — out of bounds
let r3 = slice::try_range(1..4, &data);
// → Ok(1..4)

// ─────────────────────────────────────────────────────────
// path — Additional Path Functions
// ─────────────────────────────────────────────────────────

// path::absolute(path) → io::Result<PathBuf>
// Converts a relative path to absolute by prepending current_dir.
// Resolves . components but NOT .. (use canonicalize for that).
use std::path;
let abs = path::absolute("foo/bar").unwrap();
// → "/current/dir/foo/bar" (on Unix) or "C:\\current\\dir\\foo\\bar" (on Windows)

// Path::is_separator() — check if a byte is a path separator (nightly)
#![feature(path_is_separator)]
// Platform-specific: '/' on Unix, '/' or '\\' on Windows
// let is_sep = Path::is_separator(b'/');  // true on all platforms
// let is_sep = Path::is_separator(b'\\'); // true on Windows only

// ─────────────────────────────────────────────────────────
// net::hostname — Get System Hostname (nightly)
// ─────────────────────────────────────────────────────────
// Returns the system's hostname as a String.
#![feature(hostname)]
use std::net;
let name = net::hostname().unwrap();
// → "my-computer" or similar

// ─────────────────────────────────────────────────────────
// std::unsafe_binder — Unsafe Binder Types (nightly)
// ─────────────────────────────────────────────────────────
// Experimental feature for type-safe unsafe abstractions.
// Provides wrap_binder! and unwrap_binder! macros.
// Types wrapped in an unsafe binder can only be accessed via unsafe,
// providing a way to enforce safety invariants at the type level.
#![feature(unsafe_binder)]
// let binder = unsafe { wrap_binder!(MyType { ... }) };
// let inner: MyType = unsafe { unwrap_binder!(binder) };
