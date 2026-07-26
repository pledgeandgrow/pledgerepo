# Concurrency and Async

**Docs:** https://doc.rust-lang.org/book/ch16-00-concurrency.html | https://doc.rust-lang.org/std/thread/index.html

## Threads

```rust
use std::thread;
use std::time::Duration;

// Spawn a thread
let handle = thread::spawn(|| {
    for i in 0..5 {
        println!("spawned thread: {}", i);
        thread::sleep(Duration::from_millis(1));
    }
});

for i in 0..5 {
    println!("main thread: {}", i);
    thread::sleep(Duration::from_millis(1));
}

// Wait for thread to finish
handle.join().unwrap();
```

### move Closures in Threads

```rust
let data = vec![1, 2, 3];

let handle = thread::spawn(move || {
    println!("{:?}", data);  // data moved into thread
});

handle.join().unwrap();
// data is no longer available here
```

### Thread Builder

```rust
let handle = thread::Builder::new()
    .name("worker-1".to_string())
    .stack_size(4 * 1024 * 1024)
    .spawn(|| {
        println!("Custom thread");
    })
    .unwrap();

handle.join().unwrap();
```

## Message Passing (Channels)

```rust
use std::sync::mpsc;
use std::thread;

// Create a channel
let (tx, rx) = mpsc::channel();

// Multiple producers
let tx1 = tx.clone();
thread::spawn(move || {
    tx1.send("from tx1").unwrap();
});

thread::spawn(move || {
    tx.send("from tx").unwrap();
});

// Receive
for received in rx {
    println!("Got: {}", received);
}
```

### Sync vs Async Channels

```rust
// mpsc::channel() — async, unbounded (sender never blocks)
let (tx, rx) = mpsc::channel();

// mpsc::sync_channel(n) — sync, bounded (sender blocks when full)
let (tx, rx) = mpsc::sync_channel(10);  // buffer of 10

// rx is an Iterator
for msg in rx {
    println!("{}", msg);
}
```

### mpsc Channel Types

| Channel | Bounded | Sender Blocks | Multiple Senders |
|---------|---------|---------------|-----------------|
| `mpsc::channel()` | No (unbounded) | Never | Yes (clone tx) |
| `mpsc::sync_channel(n)` | Yes (n items) | When full | Yes (clone tx) |

```rust
use std::sync::mpsc;

// Send methods:
// tx.send(val) — blocks if bounded and full, returns Err if receiver dropped
// tx.try_send(val) — non-blocking, returns Err (Full/Disconnected) if can't send

// Receive methods:
// rx.recv() — blocks until message, returns Err if all senders dropped
// rx.try_recv() — non-blocking, returns Err (Empty/Disconnected)
// rx.recv_timeout(dur) — blocks up to duration
// rx.iter() — iterator that ends when all senders drop
// rx.try_iter() — non-blocking iterator over currently-buffered messages

// Channel error handling:
match tx.send(value) {
    Ok(()) => {},
    Err(mpsc::SendError(v)) => println!("Receiver dropped, value: {:?}", v),
}

// Sync channel specific:
match tx.try_send(value) {
    Ok(()) => {},
    Err(mpsc::TrySendError::Full(v)) => println!("Channel full: {:?}", v),
    Err(mpsc::TrySendError::Disconnected(v)) => println!("Receiver dropped: {:?}", v),
}
```

### mpmc Channels (std::sync::mpmc)

```rust
use std::sync::mpmc;

// mpmc supports multiple producers AND multiple consumers
// (mpsc only supports multiple producers, single consumer)
let (tx, rx) = mpmc::channel();

let rx2 = rx.clone();  // multiple receivers
let tx2 = tx.clone();  // multiple senders

thread::spawn(move || { tx.send("msg1").unwrap(); });
thread::spawn(move || { tx2.send("msg2").unwrap(); });
thread::spawn(move || { println!("rx1: {}", rx.recv().unwrap()); });
thread::spawn(move || { println!("rx2: {}", rx2.recv().unwrap()); });

// mpmc error types:
// SendTimeoutError — for send_timeout(): Timeout(T) / Disconnected(T)
// RecvTimeoutError — for recv_timeout(): Timeout / Disconnected / Message(T)
// TrySendError — Full(T) / Disconnected(T)
// TryRecvError — Empty / Disconnected

// mpmc also supports bounded channels:
let (tx, rx) = mpmc::sync_channel(10);  // bounded, 10 items
// tx.send_timeout(val, duration) — blocks with timeout
// rx.recv_timeout(duration) — receive with timeout
```

### oneshot Channels (std::sync::oneshot)

```rust
use std::sync::oneshot;

// oneshot — single send, single receive (one value, one time)
let (tx, rx) = oneshot::channel();

thread::spawn(move || {
    tx.send("result").unwrap();  // can only send once
});

let result = rx.recv();  // Result<T, RecvError>
match result {
    Ok(val) => println!("Got: {}", val),
    Err(_) => println!("Sender dropped without sending"),
}

// try_recv() also available for non-blocking check
```

## Shared State with Mutex

```rust
use std::sync::{Arc, Mutex};
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter = Arc::clone(&counter);
    handles.push(thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    }));
}

for handle in handles {
    handle.join().unwrap();
}

println!("Counter: {}", *counter.lock().unwrap());  // 10
```

### RwLock

```rust
use std::sync::RwLock;

let lock = RwLock::new(5);

// Multiple readers
{
    let r1 = lock.read().unwrap();
    let r2 = lock.read().unwrap();
    println!("{} {}", *r1, *r2);
}

// Single writer
{
    let mut w = lock.write().unwrap();
    *w += 1;
}
```

## Arc (Atomic Reference Counting)

```rust
use std::sync::Arc;

// Arc — thread-safe reference counting
let data = Arc::new(vec![1, 2, 3]);

let data_clone = Arc::clone(&data);
thread::spawn(move || {
    println!("{:?}", data_clone);
}).join().unwrap();

println!("{:?}", data);  // still valid — Arc::clone increments count

// Arc vs Rc:
// Arc — atomic, thread-safe, for shared state across threads
// Rc — non-atomic, single-threaded only
```

## Send and Sync Traits

```rust
// Send — type can be transferred to another thread
// Sync — &T can be shared between threads

// Most types are Send + Sync automatically
// Types that are NOT Send: Rc<T>, *const T, *mut T
// Types that are NOT Sync: Cell<T>, RefCell<T>, Rc<T>

// Arc<T> is Send + Sync when T is Send + Sync
// Mutex<T> is Send + Sync when T is Send
```

## Atomic Types

```rust
use std::sync::atomic::{AtomicI32, Ordering};

let counter = AtomicI32::new(0);

// Atomic operations
counter.fetch_add(1, Ordering::SeqCst);
counter.fetch_sub(1, Ordering::SeqCst);
let val = counter.load(Ordering::SeqCst);
counter.store(0, Ordering::SeqCst);
let old = counter.swap(10, Ordering::SeqCst);

// Ordering options:
// Relaxed — no ordering constraints, just atomicity
// Acquire/Release — establish happens-before relationship
// SeqCst — sequentially consistent (strongest, most restrictive)

// Generic Atomic<T> (nightly):
// use std::sync::atomic::Atomic;
// let atomic = Atomic::new(42i32);
// atomic.fetch_add(1, Ordering::Relaxed);
// Requires T: Copy (integers, floats, bools, pointers)

// Atomic types available:
// AtomicBool, AtomicI8..AtomicI64, AtomicIsize,
// AtomicU8..AtomicU64, AtomicUsize, AtomicPtr<T>
// Atomic<T> (generic, nightly)
```

## Synchronization Primitives (std::sync)

```rust
use std::sync::{Barrier, Condvar, Mutex, RwLock, Once, OnceLock, LazyLock};

// Barrier — block N threads until all reach the barrier:
let barrier = Barrier::new(10);  // wait for 10 threads
thread::scope(|s| {
    for _ in 0..10 {
        s.spawn(|| {
            // do work...
            barrier.wait();  // blocks until all 10 threads call wait()
            // exactly one thread gets BarrierWaitResult::is_leader() == true
        });
    }
});

// Condvar — condition variable for waiting on a condition:
let pair = Arc::new((Mutex::new(false), Condvar::new()));
let pair2 = Arc::clone(&pair);
thread::spawn(move || {
    let (lock, cvar) = &*pair2;
    let mut started = lock.lock().unwrap();
    *started = true;
    cvar.notify_one();  // wake one waiting thread
});
let (lock, cvar) = &*pair;
let mut started = lock.lock().unwrap();
while !*started {
    started = cvar.wait(started).unwrap();  // releases lock, waits, re-acquires
}
// Also: cvar.notify_all(), cvar.wait_timeout(lock, duration)
// cvar.wait_timeout_ms(lock, ms) (deprecated, use Duration)

// Once — one-time initialization:
static INIT: Once = Once::new();
INIT.call_once(|| {
    // runs exactly once, even if called from multiple threads
});

// OnceLock — thread-safe OnceCell (1.70+):
static CONFIG: OnceLock<String> = OnceLock::new();
let val = CONFIG.get_or_init(|| "default".to_string());
// CONFIG.get() → Option<&String>
// CONFIG.set(val) → Result<(), T> (Err if already set)
// CONFIG.get_mut() → Option<&mut String>
// CONFIG.into_inner() → String

// LazyLock — thread-safe lazy initialization (1.80+):
static DB: LazyLock<Database> = LazyLock::new(|| open_database());
// DB is initialized on first access via deref
let conn = &*DB;  // initializes if needed

// ReentrantLock — reentrant mutex (same thread can lock multiple times):
// use std::sync::ReentrantLock;
// let lock = ReentrantLock::new(42);
// let guard1 = lock.lock();
// let guard2 = lock.lock();  // same thread — OK, doesn't deadlock
// Uses RefCell internally — requires T: Send
// Useful for callbacks that need to re-acquire the same lock

// Mutex poisoning:
// When a thread panics while holding a Mutex, the lock is "poisoned"
// Subsequent .lock() calls return Err(PoisonError<MutexGuard>)
// Use .lock().unwrap() to propagate poison, or recover with PoisonError::into_inner()

// Mapped guards (nightly):
// Mutex::lock() returns MutexGuard
// MutexGuard::map(guard, |t| &mut t.field) → MappedMutexGuard
// RwLockReadGuard::map() → MappedRwLockReadGuard
// RwLockWriteGuard::map() → MappedRwLockWriteGuard
// Allows narrowing the guard to a sub-field

// nonpoison module (std::sync::nonpoison) — locks that don't poison:
// nonpoison::Mutex, nonpoison::RwLock, nonpoison::Condvar
// Same API but no PoisonError on panic — simpler error handling
// MappedMutexGuard, MappedRwLockReadGuard, MappedRwLockWriteGuard
```

## Async/Await

```rust
// async fn returns a Future
async fn fetch_data() -> String {
    // simulate async work
    String::from("data")
}

async fn process() -> i32 {
    let data = fetch_data().await;  // await the future
    data.len() as i32
}

// Futures are lazy — nothing runs until polled
// Need a runtime to drive futures
```

### Tokio Runtime

```rust
// Cargo.toml: tokio = { version = "1", features = ["full"] }

#[tokio::main]
async fn main() {
    let result = fetch_data().await;
    println!("{}", result);
}

// Spawn concurrent tasks
#[tokio::main]
async fn main() {
    let task1 = tokio::spawn(async {
        fetch_data().await
    });

    let task2 = tokio::spawn(async {
        fetch_data().await
    });

    let (r1, r2) = tokio::join!(task1, task2);
    println!("{} {}", r1.unwrap(), r2.unwrap());
}
```

### Async Patterns

```rust
// Concurrent execution with join!
let (a, b, c) = tokio::join!(fetch_a(), fetch_b(), fetch_c());

// Select — first to complete
tokio::select! {
    val = operation1() => println!("op1: {}", val),
    val = operation2() => println!("op2: {}", val),
}

// Spawn background task
let handle = tokio::spawn(async {
    background_work().await
});

// Channels for async communication
use tokio::sync::mpsc;
let (tx, mut rx) = mpsc::channel(100);
tokio::spawn(async move {
    tx.send("hello").await.unwrap();
});
while let Some(msg) = rx.recv().await {
    println!("{}", msg);
}
```

## thread_local

```rust
use std::cell::RefCell;
use std::thread;

thread_local! {
    static COUNTER: RefCell<i32> = RefCell::new(0);
}

COUNTER.with(|c| {
    *c.borrow_mut() += 1;
});

COUNTER.with(|c| {
    println!("Counter: {}", *c.borrow());
});
```

### Thread Extras (std::thread)

```rust
use std::thread;

// available_parallelism — number of usable CPU cores:
let cores = thread::available_parallelism().unwrap().get();

// ThreadId — unique identifier for each thread:
let id = thread::current().id();
println!("Thread: {:?}", id);

// park / park_timeout — block current thread until unparked or timeout:
thread::park_timeout(std::time::Duration::from_secs(1));

// Thread::name — get thread name:
let name = thread::current().name();  // Option<&str>

// Scoped threads (1.63+) — borrow non-'static data:
let mut data = vec![1, 2, 3, 4, 5];
thread::scope(|s| {
    // Threads can borrow local variables (no move needed)
    s.spawn(|| println!("First half: {:?}", &data[..2]));
    s.spawn(|| println!("Second half: {:?}", &data[2..]));
    // All scoped threads are joined at end of scope
});
// data is still accessible here — scoped threads are guaranteed joined

// Thread::spawn returns JoinHandle<T>:
// handle.thread().name() — get thread name
// handle.thread().id() — get ThreadId
// handle.is_finished() — check if thread finished (non-blocking)
```

## Barrier — Synchronization Point

```rust
use std::sync::Barrier;

// Barrier blocks N threads until all N reach the barrier
let barrier = Arc::new(Barrier::new(3));

for i in 0..3 {
    let b = Arc::clone(&barrier);
    thread::spawn(move || {
        println!("Thread {} waiting", i);
        b.wait();  // blocks until all 3 threads call wait()
        println!("Thread {} released", i);
    });
}
```

## Condvar — Condition Variable

```rust
use std::sync::{Arc, Mutex, Condvar};

// Condvar allows threads to wait for a condition to become true
let pair = Arc::new((Mutex::new(false), Condvar::new()));
let pair2 = Arc::clone(&pair);

// Waiting thread
thread::spawn(move || {
    let (lock, cvar) = &*pair2;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();  // releases lock, waits, re-acquires
    }
    println!("Condition met!");
});

// Notifying thread
let (lock, cvar) = &*pair;
let mut started = lock.lock().unwrap();
*started = true;
cvar.notify_one();  // wake waiting thread
drop(started);
```

## Once and OnceLock — One-time Initialization

```rust
use std::sync::{Once, OnceLock};

// Once — run initialization exactly once
static INIT: Once = Once::new();

INIT.call_once(|| {
    // This runs exactly once, even if called from multiple threads
    println!("Initializing...");
});

// OnceLock — thread-safe lazy initialization of a static value
static CONFIG: OnceLock<String> = OnceLock::new();

let val = CONFIG.get_or_init(|| {
    "computed value".to_string()
});
// Subsequent calls return the same value
```

## LazyLock — Lazy Static Initialization

```rust
use std::sync::LazyLock;

// LazyLock initializes the value on first access (thread-safe)
static DB_CONNECTION: LazyLock<String> = LazyLock::new(|| {
    println!("Opening database connection...");
    "connection_string".to_string()
});

// First access initializes:
fn main() {
    println!("Before access");
    let conn = &*DB_CONNECTION;  // prints "Opening database connection..."
    println!("Using: {}", conn);
    let conn2 = &*DB_CONNECTION;  // no re-initialization
}
```

## ReentrantLock — Reentrant Mutex

```rust
use std::sync::ReentrantLock;

// ReentrantLock allows the same thread to lock multiple times without deadlocking
let lock = ReentrantLock::new(42);

let guard = lock.lock().unwrap();
println!("{}", *guard);
{
    let guard2 = lock.lock().unwrap();  // same thread can re-lock
    println!("{}", *guard2);
}
// Note: ReentrantLock is more expensive than Mutex
```

## Poison Awareness

```rust
// When a thread panics while holding a Mutex/RwLock, the lock is "poisoned"
// Subsequent lock attempts return Err(PoisonError)
use std::sync::{Arc, Mutex};

let lock = Arc::new(Mutex::new(42));
let lock2 = Arc::clone(&lock);

let handle = thread::spawn(move || {
    let _guard = lock2.lock().unwrap();
    panic!("oops");  // lock is now poisoned
});

handle.join().unwrap_or(());

// Lock returns Err:
match lock.lock() {
    Ok(guard) => println!("Got: {}", *guard),
    Err(poisoned) => {
        // poisoned.into_inner() gives access to the (possibly inconsistent) data
        let guard = poisoned.into_inner();
        println!("Recovered from poison: {}", *guard);
    }
}
```

## Future and Task Internals (std::future, std::task)

### Future Trait

```rust
use std::future::Future;
use std::task::{Poll, Context};

// The Future trait:
// trait Future {
//     type Output;
//     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
// }

// Poll enum:
// enum Poll<T> { Ready(T), Pending }

// async fn desugars into a state machine implementing Future:
async fn fetch() -> i32 { 42 }
// is equivalent to a type implementing Future<Output = i32>

// IntoFuture — allows .await on types that aren't directly Future
// (e.g., types implementing IntoFuture can be .awaited)

// AsyncDrop — async version of Drop (nightly):
// trait AsyncDrop {
//     fn async_drop(self: Pin<&mut Self>) -> impl Future<Output = ()>;
// }
// Allows async cleanup when a value is dropped
// std::future::async_drop_in_place — async drop in place (nightly)
```

### Constructing Futures

```rust
use std::future::{ready, pending, poll_fn};

// ready — immediately ready future
let f = ready(42);
assert_eq!(f.await, 42);

// pending — never completes
let f: Future<Output = ()> = pending();
// f.await;  // would hang forever

// poll_fn — create a Future from a closure
let mut counter = 0;
let f = poll_fn(move |cx| {
    counter += 1;
    if counter >= 3 {
        Poll::Ready(counter)
    } else {
        cx.waker().wake_by_ref();  // schedule re-poll
        Poll::Pending
    }
});
assert_eq!(f.await, 3);
```

### Waker and Context

```rust
use std::task::{Waker, Context, Poll, Wake};

// Waker — used to wake up a task when it's ready to make progress
// Context — carries the Waker to poll methods

// When implementing Future manually:
struct Delay {
    when: std::time::Instant,
    waker: Option<Waker>,
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if std::time::Instant::now() >= self.when {
            Poll::Ready(())
        } else {
            // Store waker to be called when ready
            // In real code, register with a timer
            cx.waker().wake_by_ref();  // or: cx.waker().wake()
            Poll::Pending
        }
    }
}

// ready! macro — early return on Pending
// fn poll(...) -> Poll<T> {
//     let val = std::task::ready!(some_future.poll(cx));
//     Poll::Ready(val)
// }
```

### join! Macro (std::future)

```rust
// std::future::join! — poll multiple futures concurrently
// (available in std, not just tokio)
use std::future::join;

// All futures are polled, results returned as a tuple
let (a, b, c) = join!(fetch_a(), fetch_b(), fetch_c()).await;
```

## Async Iterators (std::async_iter)

```rust
use std::async_iter::{AsyncIterator, IntoAsyncIterator};
use std::task::{Poll, Context};
use std::pin::Pin;

// AsyncIterator — async version of Iterator:
// trait AsyncIterator {
//     type Item;
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
// }

// Implementing a custom async iterator:
struct Counter { count: usize }

impl AsyncIterator for Counter {
    type Item = usize;
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.count += 1;
        if self.count <= 5 {
            Poll::Ready(Some(self.count))
        } else {
            Poll::Ready(None)
        }
    }
}

// Using async iterators with `for await` (nightly):
// #![feature(async_for_loop)]
// for x in counter {
//     println!("{x}");
// }

// from_iter — create async iterator from sync iterator:
use std::async_iter::from_iter;
let async_iter = from_iter(vec![1, 2, 3]);

// Async iterators are lazy — nothing happens until poll_next is called
```
