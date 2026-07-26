# Collections and Strings

**Docs:** https://doc.rust-lang.org/std/collections/index.html | https://doc.rust-lang.org/book/ch08-00-common-collections.html

## Vec<T>

```rust
// Creation
let mut v: Vec<i32> = Vec::new();
let v2 = vec![1, 2, 3];
let v3 = vec![0; 5];  // [0, 0, 0, 0, 0]
let v4: Vec<i32> = (0..10).collect();

// Adding elements
v.push(1);
v.push(2);
v.extend([3, 4, 5]);

// Accessing
let first = v[0];          // panics if out of bounds
let first = v.get(0);      // Option<&i32> — safe
let last = v.last();       // Option<&i32>

// Mutable access
if let Some(elem) = v.get_mut(0) {
    *elem = 10;
}

// Removing
let last = v.pop();        // Option<i32>
v.remove(0);               // O(n) — shifts elements
v.swap_remove(0);          // O(1) — swaps with last, unordered
v.truncate(2);             // keep first 2
v.clear();                 // remove all

// Iteration
for val in v.iter() { }        // &i32
for val in v.iter_mut() { }    // &mut i32
for val in v.into_iter() { }   // i32 (consumes v)

// Slicing
let slice: &[i32] = &v[1..3];

// Length and capacity
let len = v.len();
let cap = v.capacity();
v.reserve(100);            // ensure capacity for 100 more
v.shrink_to_fit();         // reduce capacity to length

// Methods
let contains = v.contains(&3);
let pos = v.iter().position(|&x| x == 3);  // Option<usize>
v.sort();
v.sort_by(|a, b| b.cmp(a));  // descending
v.sort_by_key(|x| x.abs());
v.dedup();                 // remove consecutive duplicates
v.reverse();
let chunks = v.chunks(2);  // iterator of slices
let windows = v.windows(2);
```

## String and str

```rust
// String — owned, heap-allocated, mutable
let mut s = String::new();
let s2 = String::from("hello");
let s3 = "world".to_string();
let s4 = String::with_capacity(100);

// Adding content
s.push_str("hello ");
s.push('w');
s += "orld";               // takes ownership of String
let combined = format!("{} {}", s2, s3);
let combined = s2.clone() + " " + &s3;

// str — string slice, immutable, borrowed
let literal: &str = "hello";
let borrowed: &str = &s2;

// UTF-8 — chars are variable width
let emoji = "😀";           // 4 bytes
let len_bytes = emoji.len();  // 4
let len_chars = emoji.chars().count();  // 1

// Iteration
for c in "hello".chars() { }      // char
for b in "hello".bytes() { }      // u8
for (i, c) in "hello".char_indices() { }  // (byte_offset, char)

// Slicing — must be on char boundary
let s = "hello world";
let hello = &s[0..5];     // "hello"
// let bad = &s[0..3];    // may panic if not on char boundary

// Methods
let upper = "hello".to_uppercase();
let lower = "HELLO".to_lowercase();
let trimmed = "  hi  ".trim();
let parts: Vec<&str> = "a,b,c".split(',').collect();
let joined = vec!["a", "b", "c"].join(", ");
let replaced = "hello".replace("l", "L");
let starts = "hello".starts_with("he");
let ends = "hello".ends_with("lo");
let contains = "hello".contains("ell");
let is_empty = "".is_empty();
let bytes = "hello".as_bytes();
```

## HashMap and HashSet

```rust
use std::collections::{HashMap, HashSet};

// HashMap
let mut map: HashMap<String, i32> = HashMap::new();
map.insert("Alice".to_string(), 30);
map.insert("Bob".to_string(), 25);

// Access
let age = map.get("Alice");  // Option<&i32>
let age = map.get(&"Alice".to_string());

// Mutable access
if let Some(age) = map.get_mut("Alice") {
    *age += 1;
}

// Entry API — insert if absent
map.entry("Carol".to_string()).or_insert(20);
map.entry("Alice".to_string()).and_modify(|a| *a += 1).or_insert(1);

// Remove
map.remove("Bob");

// Iterate
for (key, value) in &map {
    println!("{}: {}", key, value);
}

// HashSet
let mut set: HashSet<i32> = HashSet::new();
set.insert(1);
set.insert(2);
set.insert(3);
set.insert(1);  // no effect, already present

let contains = set.contains(&2);
set.remove(&2);

// Set operations
let a: HashSet<i32> = vec![1, 2, 3].into_iter().collect();
let b: HashSet<i32> = vec![2, 3, 4].into_iter().collect();
let union: HashSet<_> = a.union(&b).copied().collect();        // {1, 2, 3, 4}
let intersection: HashSet<_> = a.intersection(&b).copied().collect();  // {2, 3}
let diff: HashSet<_> = a.difference(&b).copied().collect();    // {1}
let sym_diff: HashSet<_> = a.symmetric_difference(&b).copied().collect();  // {1, 4}
```

## Other Collections

```rust
use std::collections::{VecDeque, LinkedList, BTreeMap, BTreeSet, BinaryHeap};

// VecDeque — double-ended queue (ring buffer)
let mut dq: VecDeque<i32> = VecDeque::new();
dq.push_back(1);
dq.push_front(0);
let front = dq.pop_front();
let back = dq.pop_back();

// LinkedList — doubly linked list (rarely needed)
let mut list = LinkedList::new();
list.push_back(1);
list.push_front(0);

// BTreeMap — ordered map (sorted by key)
let mut btree: BTreeMap<String, i32> = BTreeMap::new();
btree.insert("apple".to_string(), 3);
btree.insert("banana".to_string(), 5);
// Iteration is in sorted order

// BTreeSet — ordered set
let mut btset: BTreeSet<i32> = BTreeSet::new();
btset.insert(3);
btset.insert(1);
btset.insert(2);
// Iteration: 1, 2, 3 (sorted)

// BinaryHeap — max-heap
let mut heap: BinaryHeap<i32> = BinaryHeap::new();
heap.push(3);
heap.push(1);
heap.push(5);
let max = heap.pop();  // Some(5) — largest first

// BinaryHeap extras:
// heap.peek() — &Option<&T> (largest without removing)
// heap.into_iter_sorted() — iterate in sorted order (nightly)
// heap.drain_sorted() — drain elements in sorted order (nightly)
// heap.retention() — filter elements (nightly)

// try_reserve / try_reserve_exact — fallible allocation:
let mut v = Vec::<i32>::new();
v.try_reserve(1000).map_err(|e| {
    // TryReserveError — allocation failure
    // TryReserveErrorKind::CapacityOverflow / AllocError / OutOfMemory
    e
})?;
// Also available on HashMap, HashSet, BTreeMap, BTreeSet, VecDeque, String

// BTreeMap/BTreeSet Cursor — navigate with O(1) prev/next:
use std::collections::btree_map::Cursor;
use std::ops::Bound;
let mut map = BTreeMap::from([(1, "a"), (2, "b"), (3, "c")]);
let mut cursor = map.lower_bound(Bound::Included(&2));
assert_eq!(cursor.key(), Some(&2));
cursor.move_next();  // → key 3
cursor.move_prev();  // → key 2
// CursorMut — mutable cursor, can insert/remove at position
// CursorMutKey — mutable cursor with access to key (nightly)

// BTreeMap/BTreeSet set operations:
use std::collections::BTreeSet;
let a: BTreeSet<i32> = [1, 2, 3].into_iter().collect();
let b: BTreeSet<i32> = [2, 3, 4].into_iter().collect();
// All return lazy iterators:
a.union(&b);                   // {1, 2, 3, 4}
a.intersection(&b);            // {2, 3}
a.difference(&b);              // {1}
a.symmetric_difference(&b);    // {1, 4}
a.is_subset(&b);               // false
a.is_superset(&b);             // false
a.is_disjoint(&b);             // false

// ExtractIf — conditionally remove and yield elements (nightly):
// let mut v = vec![1, 2, 3, 4, 5];
// let evens: Vec<_> = v.extract_if(|_, x| *x % 2 == 0).collect();
// Also available on BTreeMap, BTreeSet, HashMap, HashSet, VecDeque, LinkedList

// Entry API — for all map types:
use std::collections::hash_map::Entry;
let mut map = HashMap::new();
match map.entry("key".to_string()) {
    Entry::Occupied(e) => {
        // e.get(), e.get_mut(), e.insert(v), e.remove()
    }
    Entry::Vacant(e) => {
        e.insert(42);
    }
}
// Entry methods: or_insert(v), or_insert_with(|| v), or_insert_with_key(|k| v)
// and_modify(|v| ...) — chainable before or_insert

// BTreeMap Entry also has: OccupiedError, VacantEntry, OccupiedEntry
// BTreeSet Entry: OccupiedEntry, VacantEntry (no value, just key presence)

// UnorderedKeyError — new error for HashMap/HashSet (1.97+):
// Returned when SipHash key is randomized and keys are not hashable
```

## Choosing a Collection

| Need | Use |
|------|-----|
| Ordered, indexable, growable | `Vec<T>` |
| Double-ended queue | `VecDeque<T>` |
| Key-value lookup | `HashMap<K, V>` |
| Key-value, sorted | `BTreeMap<K, V>` |
| Unique values | `HashSet<T>` |
| Unique values, sorted | `BTreeSet<T>` |
| Priority queue (max) | `BinaryHeap<T>` |
| Linked list (rare) | `LinkedList<T>` |

## When to Use Which Collection (from std::collections)

### Use `Vec` when:
- You want to collect items to process or send elsewhere later
- You want a sequence of elements in a particular order, appending to the end
- You want a stack
- You want a resizable array
- You want a heap-allocated array

### Use `VecDeque` when:
- You want a `Vec` that supports efficient insertion at both ends
- You want a queue
- You want a double-ended queue (deque)

### Use `LinkedList` when:
- You want a `Vec` or `VecDeque` of unknown size and can't tolerate amortization
- You want to efficiently split and append lists
- You are absolutely certain you really, truly, want a doubly linked list

### Use `HashMap` when:
- You want to associate arbitrary keys with an arbitrary value
- You want a cache
- You want a map, with no extra functionality

### Use `BTreeMap` when:
- You want a map sorted by its keys
- You want to get a range of entries on-demand
- You're interested in what the smallest or largest key-value pair is
- You want to find the largest or smallest key that is smaller or larger than something

### Use the Set variant of any Map when:
- You just want to remember which keys you've seen
- There is no meaningful value to associate with your keys

### Use `BinaryHeap` when:
- You want to store elements but only ever process the "biggest" or "most important" one
- You want a priority queue

## Performance Notes

- **n** = collection size, **m** = second collection size, **i** = item index
- `*` = amortized cost, `~` = expected cost
- Rust collections never automatically shrink; removal operations aren't amortized
- `HashMap` uses expected costs (probabilistic hashing; very unlikely to degrade)
- Where ties occur: `Vec` > `VecDeque` > `LinkedList` in speed
- For Sets, all operations have the cost of the equivalent Map operation

## Capacity Management

Collections use an **amortized allocation strategy** — they maintain spare capacity to avoid reallocating on every insert.

- `with_capacity(n)` — pre-allocate space for `n` elements (use when you know the size)
- `reserve(additional)` — hint the collection to make room for more items
- `shrink_to_fit()` — shrink backing array to minimum size
- `capacity()` — query current capacity

```rust
// Pre-allocate for known size — avoids reallocations
let mut v: Vec<i32> = Vec::with_capacity(1000);
for i in 0..1000 {
    v.push(i);  // no reallocation happens
}

// Shrink after removing many elements
v.truncate(10);
v.shrink_to_fit();
```

## Entry API (detailed)

The Entry API provides efficient conditional manipulation of map contents — avoids duplicate search when checking then inserting.

```rust
use std::collections::btree_map::BTreeMap;

// Counting character occurrences
let mut count = BTreeMap::new();
let message = "she sells sea shells by the sea shore";
for c in message.chars() {
    *count.entry(c).or_insert(0) += 1;
}
assert_eq!(count.get(&'s'), Some(&8));

// Entry variants:
// - Vacant(entry): key not found → only valid op is insert
// - Occupied(entry): key found → get, insert, remove, or convert to &mut value

// Entry methods:
// - or_insert(val) — inserts val if vacant, returns &mut V
// - or_insert_with(f) — inserts f() if vacant, returns &mut V
// - or_insert_with_key(|k| f(k)) — inserts f(key) if vacant, returns &mut V
// - and_modify(f) — modifies the value if occupied, returns the entry
// - or_default() — inserts Default if vacant, returns &mut V
```

## Iterator Patterns for Collections

```rust
// Three forms of iteration:
// iter()     — yields &T (immutable references)
// iter_mut() — yields &mut T (mutable references)
// into_iter() — yields T (consumes collection)

// Convert between collection types via collect:
let vec = vec![1, 2, 3, 4];
let set: HashSet<_> = vec.into_iter().collect();
let deque: VecDeque<_> = vec![1, 2, 3].into_iter().collect();

// Move contents between collections via extend:
let mut vec1 = vec![1, 2, 3];
let vec2 = vec![10, 20, 30];
vec1.extend(vec2);  // vec1 is now [1, 2, 3, 10, 20, 30]

// Reverse iteration:
for x in vec.iter().rev() {
    println!("{x:?}");
}

// Note: HashSet does NOT provide iter_mut() — mutating keys
// could put the collection into an inconsistent state
```

## String Methods (std::str, std::string)

### FromStr Trait — Parsing Strings

```rust
use std::str::FromStr;

// FromStr — parse a &str into a type
let n: i32 = i32::from_str("42").unwrap();
let n: i32 = "42".parse().unwrap();  // uses FromStr
let n = "42".parse::<i32>().unwrap();  // turbofish

// Implement FromStr for custom types:
struct Point { x: i32, y: i32 }
impl FromStr for Point {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        Ok(Point {
            x: parts[0].parse()?,
            y: parts[1].parse()?,
        })
    }
}
let p: Point = "3,4".parse().unwrap();
```

### String Slicing and Splitting

```rust
// Split methods:
let s = "hello world foo bar";
let words: Vec<&str> = s.split(' ').collect();       // ["hello", "world", "foo", "bar"]
let words: Vec<&str> = s.split_whitespace().collect(); // same, handles multiple spaces
let lines: Vec<&str> = "a\nb\nc".lines().collect();   // ["a", "b", "c"]
let parts: Vec<&str> = "a,b,c".split(',').collect();
let parts: Vec<&str> = "a,b,,c".split_terminator(',').collect(); // no trailing empty
let parts: Vec<&str> = "a,b,c".rsplit(',').collect();  // ["c", "b", "a"]
let parts: Vec<&str> = "a,b,c,d".splitn(2, ',').collect(); // ["a", "b,c,d"]

// Split inclusive (keeps delimiter):
let parts: Vec<&str> = "a,b,c".split_inclusive(',').collect(); // ["a,", "b,", "c"]

// Match methods:
let s = "hello world hello";
let matches: Vec<&str> = s.matches("hello").collect();  // ["hello", "hello"]
let indices: Vec<(usize, &str)> = s.match_indices("hello").collect();
let rmatches: Vec<&str> = s.rmatches("hello").collect();

// Contains, starts/ends with:
assert!(s.contains("world"));
assert!(s.starts_with("hello"));
assert!(s.ends_with("hello"));
assert!(s.get(0..5).is_some());  // safe slice by byte range
```

### String Manipulation

```rust
let mut s = String::from("hello");

// Insert / remove:
s.insert(5, ' ');           // "hello "
s.insert_str(6, "world");   // "hello world"
s.remove(5);                 // removes char at byte index → "helloworld"
s.remove_range(0..5);        // removes range → "world" (1.97+)

// Replace:
let s = "hello world".replace("world", "rust");  // "hello rust"
let s = "hello world".replacen("l", "L", 1);     // "heLlo world"

// Trim:
let s = "  hello  ".trim();          // "hello"
let s = "  hello  ".trim_start();    // "hello  "
let s = "  hello  ".trim_end();      // "  hello"
let s = "hello\n".trim_end_matches('\n');  // "hello"

// Case conversion (returns iterator, not String):
let s: String = "hello".to_uppercase();
let s: String = "WORLD".to_lowercase();
let s: String = "istanbul".to_titlecase();  // "Istanbul"

// Repeat:
let s = "ab".repeat(3);  // "ababab"

// Padding:
let s = format!("{:>10}", "hi");   // "        hi"
let s = format!("{:0>5}", 42);     // "00042"
```

### UTF-8 Validation

```rust
use std::str;

// from_utf8 — validate bytes as UTF-8
let bytes = [0x48, 0x65, 0x6C, 0x6C, 0x6F];  // "Hello"
let s = str::from_utf8(&bytes).unwrap();  // Ok("Hello")

let invalid = [0xFF, 0xFE];
let err = str::from_utf8(&invalid).unwrap_err();
println!("invalid at byte {}", err.valid_up_to());

// from_utf8_unchecked — unsafe, no validation (UB if invalid)
let s = unsafe { str::from_utf8_unchecked(&bytes) };

// UTF-8 chunks — iterate over valid UTF-8 chunks:
for chunk in str::Utf8Chunks::new(&invalid_bytes) {
    println!("valid: {:?}", chunk.valid());
    println!("invalid bytes: {:?}", chunk.invalid());
}

// is_char_boundary — check if byte index is a char boundary
let s = "héllo";
assert!(s.is_char_boundary(0));   // true
assert!(s.is_char_boundary(1));   // true (after 'h')
assert!(!s.is_char_boundary(2));  // false (middle of 'é')
```

### char Methods (std::char)

```rust
// char is a Unicode scalar value (4 bytes, 0x0000..=0x10FFFF)
let c = 'A';

// Conversion:
let u: u32 = c as u32;
let c = char::from_u32(65).unwrap();     // Some('A')
let c = char::from_digit(5, 10).unwrap(); // Some('5')
let c = unsafe { char::from_u32_unchecked(65) };  // 'A'

// Classification:
assert!('A'.is_alphabetic());
assert!('5'.is_numeric());
assert!(' '.is_whitespace());
assert!('A'.is_uppercase());
assert!('a'.is_lowercase());
assert!('_'.is_alphanumeric());

// Case:
let upper: char = 'a'.to_uppercase().next().unwrap();  // 'A'
let lower: char = 'A'.to_lowercase().next().unwrap();  // 'a'

// Escaping:
let escaped = 'A'.escape_default().to_string();  // "A"
let escaped = '\n'.escape_default().to_string();  // "\\n"
let escaped = 'A'.escape_unicode().to_string();  // "\\u{41}"

// UTF-16:
let utf16: Vec<u16> = "hello".encode_utf16().collect();
let chars: Vec<char> = char::decode_utf16(utf16)
    .filter_map(|r| r.ok())
    .collect();

// Constants:
assert_eq!(char::MAX, '\u{10FFFF}');
assert_eq!(char::REPLACEMENT_CHARACTER, '\u{FFFD}');
```

## Slice Methods (std::slice)

```rust
// Chunks — split into fixed-size sub-slices:
let v = [1, 2, 3, 4, 5];
for chunk in v.chunks(2) {  // [1,2], [3,4], [5]
    println!("{:?}", chunk);
}
for chunk in v.chunks_exact(2) {  // [1,2], [3,4] — remainder separate
    println!("{:?}", chunk);
}
for chunk in v.rchunks(2) {  // [4,5], [2,3], [1]
    println!("{:?}", chunk);
}

// Windows — overlapping sliding windows:
for w in v.windows(3) {  // [1,2,3], [2,3,4], [3,4,5]
    println!("{:?}", w);
}

// Array windows — fixed-size array windows:
for w in v.array_windows::<3>() {  // [1,2,3], [2,3,4], [3,4,5] as [i32; 3]
    println!("{:?}", w);
}

// Split:
let v = [1, 2, 3, 4, 5];
let parts: Vec<&[i32]> = v.split(|&x| x == 3).collect();  // [[1,2], [4,5]]
let parts: Vec<&[i32]> = v.split_inclusive(|&x| x == 3).collect();  // [[1,2,3], [4,5]]

// Concat and join:
let parts: [&[i32]; 3] = [&[1], &[2], &[3]];
assert_eq!(parts.concat(), [1, 2, 3]);
assert_eq!(parts.join(&0), [1, 0, 2, 0, 3]);

// from_fn — create array from closure:
let arr: [i32; 5] = std::array::from_fn(|i| i * i);  // [0, 1, 4, 9, 16]
let arr: Result<[i32; 5], _> = std::array::try_from_fn(|i| i.checked_mul(i).ok_or(()));

// repeat — repeat array:
let arr: [i32; 6] = std::array::repeat([1, 2]);  // [1, 2, 1, 2, 1, 2]

// from_ref / from_mut — convert &T to &[T; 1]:
let x = 42;
let arr: &[i32; 1] = std::array::from_ref(&x);

// Sort:
let mut v = vec![3, 1, 4, 1, 5, 9, 2, 6];
v.sort();                          // [1, 1, 2, 3, 4, 5, 6, 9]
v.sort_by(|a, b| b.cmp(a));       // descending
v.sort_by_key(|&x| std::cmp::Reverse(x));  // descending
v.sort_unstable();                 // faster, not stable
v.sort_unstable_by_key(|&x| x.abs());

// Binary search:
let v = vec![1, 3, 5, 7, 9];
assert_eq!(v.binary_search(&5), Ok(2));
assert_eq!(v.binary_search_by(|x| x.cmp(&5)), Ok(2));
assert_eq!(v.binary_search_by_key(&5, |&x| x), Ok(2));

// Rotate:
let mut v = vec![1, 2, 3, 4, 5];
v.rotate_left(2);   // [3, 4, 5, 1, 2]
v.rotate_right(2);  // [1, 2, 3, 4, 5]

// Fill:
let mut v = vec![0; 5];
v.fill(42);           // [42, 42, 42, 42, 42]
v.fill_with(|| rand()); // fill with closure

// Reverse:
let mut v = vec![1, 2, 3];
v.reverse();  // [3, 2, 1]

// get_disjoint_mut — get multiple mutable slices:
let mut v = vec![1, 2, 3, 4, 5];
let [a, b] = v.get_disjoint_mut([0..2, 3..5]).unwrap();
// a = &mut [1, 2], b = &mut [4, 5]
```

## Byte Strings (std::bstr)

```rust
use std::bstr::{ByteStr, ByteString};

// ByteStr — byte string slice type, like &str but for &[u8]
// ByteString — owned byte string, like String but for Vec<u8>

// ByteStr wraps &[u8] and provides str-like methods:
let bytes: &ByteStr = ByteStr::new(b"hello world");
let parts: Vec<&ByteStr> = bytes.split_str(" ").collect();

// ByteString — owned, growable byte string:
let mut bs = ByteString::new();
bs.push_str(b"hello");
bs.push(b' ');
bs.push_str(b"world");

// Conversions:
let owned: ByteString = ByteString::from(b"hello".to_vec());
let slice: &ByteStr = ByteStr::new(&owned);

// Useful when working with non-UTF-8 byte sequences that need
// string-like operations (split, contains, starts_with, etc.)
```

## TryReserveError and TryReserveErrorKind

```rust
// TryReserveError — error returned by try_reserve() / try_reserve_exact()
// Contains capacity information and the kind of error.
use std::collections::TryReserveError;

// TryReserveErrorKind — enum describing the error reason:
// - CapacityOverflow: capacity would overflow isize::MAX
// - AllocError: allocator returned an error
use std::collections::TryReserveErrorKind;

let mut v: Vec<i32> = Vec::new();
match v.try_reserve(usize::MAX) {
    Ok(()) => {},
    Err(e) => match e.kind() {
        TryReserveErrorKind::CapacityOverflow => {
            // requested capacity exceeds isize::MAX
        }
        TryReserveErrorKind::AllocError { layout } => {
            // allocator failed to allocate memory
        }
    }
}

// try_reserve() is available on Vec, HashMap, HashSet, BTreeMap, BTreeSet,
// VecDeque, LinkedList, String, and other collections.
// Unlike reserve(), it returns Result instead of panicking.
```

## str::pattern — Pattern Matching Module

```rust
// std::str::pattern — traits for string pattern matching (nightly)
// Provides the abstraction layer behind str::find, str::contains,
// str::split, str::matches, etc.

// Traits:
// Pattern — implemented by &str, char, &[char], FnMut(char) -> bool, etc.
//   Enables pattern-based search in strings
// Searcher — iterator-like searcher for Pattern
// ReverseSearcher — searcher that works backwards
// DoubleEndedSearcher — searcher that works both directions

// Structs (searcher implementations):
// CharSearcher, CharSliceSearcher, CharArraySearcher, CharArrayRefSearcher,
// CharPredicateSearcher, StrSearcher

// SearchStep enum:
// Match { start, end } — a match was found at [start, end)
// Reject { start, end } — no match in [start, end)
// Done — search is complete

// Example (nightly):
#![feature(pattern)]
use std::str::pattern::Pattern;
// "hello".find('l') uses Pattern impl for char
// "hello".find("ll") uses Pattern impl for &str
// "hello".find(|c| c.is_ascii_digit()) uses Pattern impl for FnMut
```
