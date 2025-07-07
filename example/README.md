## Benchmark

### Rust Mutex Shootout

This basic benchmark shows nsync::Mutex is faster as contention increases. In normal contention scenarios its as efficient as std::Mutex.

```
=== Rust Mutex Shootout ===
Based on Mark Waterman's C++ mutex benchmark

Configuration:
  Objects: 10000
  LRU Cache Size: 20000
  Total Operations: 50000000
  Payload Size: 2KB per cache entry

Generating 10000 UUID keys...
Keys generated.

=== 2 Threads ===
std::Mutex:   12.617422 seconds
nsync::Mutex: 7.887267 seconds
SpinLock:     3.591567 seconds

Performance comparison (vs std::Mutex):
  nsync::Mutex: 1.60x faster
  SpinLock:     3.51x faster

=== 4 Threads ===
std::Mutex:   7.101643 seconds
nsync::Mutex: 7.703925 seconds
SpinLock:     3.870337 seconds

Performance comparison (vs std::Mutex):
  nsync::Mutex: 1.08x slower
  SpinLock:     1.83x faster

=== 8 Threads ===
std::Mutex:   11.042134 seconds
nsync::Mutex: 8.161319 seconds
SpinLock:     10.034476 seconds

Performance comparison (vs std::Mutex):
  nsync::Mutex: 1.35x faster <- FASTER!
  SpinLock:     1.10x faster

=== 16 Threads ===
std::Mutex:   15.624889 seconds
nsync::Mutex: 8.741397 seconds
SpinLock:     29.189532 seconds

Performance comparison (vs std::Mutex):
  nsync::Mutex: 1.79x faster <- FASTER!
  SpinLock:     1.87x slower
```
