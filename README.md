# nsync-rs

Safe Rust bindings for [Google's nsync](https://github.com/google/nsync) synchronization library. This library (WIP) provides high-performance, cross-platform synchronization primitives that can outperform standard library implementations in high-contention scenarios.

## Features

- **High-Performance Mutexes**: Reader-writer locks that are as efficient as standard mutexes
- **Compact Memory Footprint**: Locks and condition variables occupy only two words each
- **Cross-Platform**: Works on Unix-like systems and Windows
- **Drop-in Replacement**: API designed to be familiar to Rust developers

## Why nsync?

nsync offers several advantages over standard synchronization primitives:

1. **Better Performance**: Especially in high-contention scenarios
2. **Reader-Writer Efficiency**: Reader-writer locks with mutex-like performance
3. **Automatic Condition Checking**: Conditional critical sections eliminate manual while loops
4. **Cancellation Model**: More flexible than thread-based cancellation
5. **Cross-Platform Consistency**: Same performance characteristics across platforms

### Basic Usage

```rust
use nsync_rs::{Mutex, Condvar, Once};
use std::sync::Arc;
use std::thread;

// Basic mutex usage
let data = Arc::new(Mutex::new(0));
let data_clone = Arc::clone(&data);

thread::spawn(move || {
    let mut guard = data_clone.lock().unwrap();
    *guard += 1;
});

// Reader-writer lock
use nsync_rs::RwLock;

let lock = RwLock::new(5);

// Multiple readers
let reader = lock.read().unwrap();
println!("Value: {}", *reader);

// Single writer
let mut writer = lock.write().unwrap();
*writer += 1;
```

## Performance Benchmarks

This crate includes benchmarks comparing nsync with standard library mutexes and spin locks. To run them:

```bash
cd example
cargo run --release counter 8 100000    # Simple counter benchmark
cargo run --release shootout             # LRU cache benchmark
cargo run --release                      # Run both benchmarks
```

### Benchmark Results

The benchmarks test two scenarios:

1. **Simple Counter**: Multiple threads incrementing a shared counter
2. **LRU Cache**: More realistic workload with larger critical sections

Typical results show nsync performing 1.3-1.7x better than std::Mutex in high-contention scenarios, with the advantage increasing as thread count grows.

### Error Handling

The API follows Rust conventions with `LockResult<T>` and `TryLockResult<T>` types that handle poisoning similar to `std::sync`.

## Examples

Check out the `example/` directory for comprehensive usage examples including:

- Mutex performance comparisons
- LRU cache implementations
- Benchmark suites
  
## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## Acknowledgments

- [Google's nsync library](https://github.com/google/nsync) for the underlying implementation
- The Rust community for inspiration on safe API design
- [Justine Tunney's mutex research](https://justine.lol/fastmutex/)

## Related Projects

- [parking_lot](https://crates.io/crates/parking_lot) - Alternative high-performance synchronization
- [spin](https://crates.io/crates/spin) - Spin-based synchronization primitives
- [std::sync](https://doc.rust-lang.org/std/sync/) - Rust standard library synchronization

---

**Note**: This is not an official Google product. nsync-rs is an independent Rust wrapper around the nsync library.
