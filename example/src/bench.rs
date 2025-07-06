use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use uuid::Uuid;

// Constants matching the mutex shootout bench
const OBJ_COUNT: usize = 10000;
const LRU_SIZE: usize = 20000;
const OP_COUNT: usize = 50_000_000;

type Payload = [u8; 2048];

/// Simple LRU cache implementation using HashMap + timestamps
/// This is a simplified version that focuses on mutex performance rather than optimal LRU
pub struct SimpleLruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    capacity: usize,
    data: HashMap<K, (V, usize)>, // (value, access_time)
    access_counter: usize,
}

impl<K, V> SimpleLruCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: HashMap::with_capacity(capacity),
            access_counter: 0,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        self.access_counter += 1;

        if self.data.len() >= self.capacity && !self.data.contains_key(&key) {
            // Find and remove the least recently used item
            let oldest_key = self
                .data
                .iter()
                .min_by_key(|(_, (_, access_time))| *access_time)
                .map(|(k, _)| k.clone());

            if let Some(old_key) = oldest_key {
                self.data.remove(&old_key);
            }
        }

        self.data.insert(key, (value, self.access_counter));
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some((value, _)) = self.data.get(key) {
            let value = value.clone();
            self.access_counter += 1;
            self.data
                .insert(key.clone(), (value.clone(), self.access_counter));
            Some(value)
        } else {
            None
        }
    }
}

/// Simple spin lock implementation
pub struct SpinLock<T> {
    locked: AtomicBool,
    data: std::cell::UnsafeCell<T>,
}

unsafe impl<T: Send> Send for SpinLock<T> {}
unsafe impl<T: Send> Sync for SpinLock<T> {}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    _phantom: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> std::ops::Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> std::ops::DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: std::cell::UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // Spin until we can acquire the lock
        while self.locked.swap(true, Ordering::Acquire) {
            // Hint to the processor that we're spinning
            while self.locked.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        }

        SpinLockGuard {
            lock: self,
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Populate the cache with initial data
fn populate_std_cache(
    lru_cache: &Arc<std::sync::Mutex<SimpleLruCache<Uuid, Arc<Payload>>>>,
    keys: &[Uuid],
) {
    for &key in keys {
        let payload = Arc::new([b'x'; 2048]);
        lru_cache.lock().unwrap().set(key, payload);
    }
}

fn populate_nsync_cache(
    lru_cache: &Arc<nsync_rs::Mutex<SimpleLruCache<Uuid, Arc<Payload>>>>,
    keys: &[Uuid],
) {
    for &key in keys {
        let payload = Arc::new([b'x'; 2048]);
        lru_cache.lock().unwrap().set(key, payload);
    }
}

fn populate_spin_cache(
    lru_cache: &Arc<SpinLock<SimpleLruCache<Uuid, Arc<Payload>>>>,
    keys: &[Uuid],
) {
    for &key in keys {
        let payload = Arc::new([b'x'; 2048]);
        lru_cache.lock().set(key, payload);
    }
}

/// Perform get operations on std::Mutex cache
fn do_std_gets(
    keys: &[Uuid],
    lru_cache: &Arc<std::sync::Mutex<SimpleLruCache<Uuid, Arc<Payload>>>>,
    op_count: usize,
) {
    for i in 0..op_count {
        let key = keys[i % keys.len()];
        let result = lru_cache.lock().unwrap().get(&key);
        assert!(result.is_some(), "Key should exist in cache");
    }
}

/// Perform get operations on nsync cache
fn do_nsync_gets(
    keys: &[Uuid],
    lru_cache: &Arc<nsync_rs::Mutex<SimpleLruCache<Uuid, Arc<Payload>>>>,
    op_count: usize,
) {
    for i in 0..op_count {
        let key = keys[i % keys.len()];
        let result = lru_cache.lock().unwrap().get(&key);
        assert!(result.is_some(), "Key should exist in cache");
    }
}

/// Perform get operations on spin lock cache
fn do_spin_gets(
    keys: &[Uuid],
    lru_cache: &Arc<SpinLock<SimpleLruCache<Uuid, Arc<Payload>>>>,
    op_count: usize,
) {
    for i in 0..op_count {
        let key = keys[i % keys.len()];
        let result = lru_cache.lock().get(&key);
        assert!(result.is_some(), "Key should exist in cache");
    }
}

/// Run benchmark with std::Mutex
pub fn run_std_lru_benchmark(
    keys: &[Uuid],
    thread_count: usize,
    total_op_count: usize,
    lru_size: usize,
) -> f64 {
    let lru_cache = Arc::new(std::sync::Mutex::new(SimpleLruCache::new(lru_size)));
    populate_std_cache(&lru_cache, keys);

    let ops_per_thread = total_op_count / thread_count;
    let mut handles = Vec::with_capacity(thread_count);

    let stopwatch = Instant::now();

    for _ in 0..thread_count {
        let keys_clone = keys.to_vec();
        let cache_clone = Arc::clone(&lru_cache);

        let handle = thread::spawn(move || {
            do_std_gets(&keys_clone, &cache_clone, ops_per_thread);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    stopwatch.elapsed().as_secs_f64()
}

/// Run benchmark with nsync::Mutex
pub fn run_nsync_lru_benchmark(
    keys: &[Uuid],
    thread_count: usize,
    total_op_count: usize,
    lru_size: usize,
) -> f64 {
    let lru_cache = Arc::new(nsync_rs::Mutex::new(SimpleLruCache::new(lru_size)));
    populate_nsync_cache(&lru_cache, keys);

    let ops_per_thread = total_op_count / thread_count;
    let mut handles = Vec::with_capacity(thread_count);

    let stopwatch = Instant::now();

    for _ in 0..thread_count {
        let keys_clone = keys.to_vec();
        let cache_clone = Arc::clone(&lru_cache);

        let handle = thread::spawn(move || {
            do_nsync_gets(&keys_clone, &cache_clone, ops_per_thread);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    stopwatch.elapsed().as_secs_f64()
}

/// Run benchmark with SpinLock
pub fn run_spin_lru_benchmark(
    keys: &[Uuid],
    thread_count: usize,
    total_op_count: usize,
    lru_size: usize,
) -> f64 {
    let lru_cache = Arc::new(SpinLock::new(SimpleLruCache::new(lru_size)));
    populate_spin_cache(&lru_cache, keys);

    let ops_per_thread = total_op_count / thread_count;
    let mut handles = Vec::with_capacity(thread_count);

    let stopwatch = Instant::now();

    for _ in 0..thread_count {
        let keys_clone = keys.to_vec();
        let cache_clone = Arc::clone(&lru_cache);

        let handle = thread::spawn(move || {
            do_spin_gets(&keys_clone, &cache_clone, ops_per_thread);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    stopwatch.elapsed().as_secs_f64()
}

pub fn create_keys(count: usize) -> Vec<Uuid> {
    (0..count).map(|_| Uuid::new_v4()).collect()
}

pub fn run_mutex_shootout() {
    println!("=== Rust Mutex Shootout ===");
    println!("Based on Mark Waterman's C++ mutex benchmark");
    println!();
    println!("Configuration:");
    println!("  Objects: {}", OBJ_COUNT);
    println!("  LRU Cache Size: {}", LRU_SIZE);
    println!("  Total Operations: {}", OP_COUNT);
    println!("  Payload Size: 2KB per cache entry");
    println!();

    // Create keys for the benchmark
    println!("Generating {} UUID keys...", OBJ_COUNT);
    let keys = create_keys(OBJ_COUNT);
    println!("Keys generated.\n");

    // Test different thread counts
    let thread_counts = [2, 4, 8, 16];

    for &thread_count in &thread_counts {
        println!(
            "=== {} Thread{} ===",
            thread_count,
            if thread_count == 1 { "" } else { "s" }
        );

        // Warm up
        if thread_count == 1 {
            println!("Warming up...");
            run_std_lru_benchmark(&keys, 1, 1000, LRU_SIZE);
            println!("Warm up complete.\n");
        }

        // Test std::Mutex
        let std_time = run_std_lru_benchmark(&keys, thread_count, OP_COUNT, LRU_SIZE);
        println!("std::Mutex:   {:.6} seconds", std_time);

        // Test nsync::Mutex
        let nsync_time = run_nsync_lru_benchmark(&keys, thread_count, OP_COUNT, LRU_SIZE);
        println!("nsync::Mutex: {:.6} seconds", nsync_time);

        // Test SpinLock
        let spin_time = run_spin_lru_benchmark(&keys, thread_count, OP_COUNT, LRU_SIZE);
        println!("SpinLock:     {:.6} seconds", spin_time);

        // Calculate and display relative performance
        println!();
        let nsync_vs_std = std_time / nsync_time;
        let spin_vs_std = std_time / spin_time;

        println!("Performance comparison (vs std::Mutex):");
        println!(
            "  nsync::Mutex: {:.2}x {}",
            if nsync_vs_std > 1.0 {
                nsync_vs_std
            } else {
                1.0 / nsync_vs_std
            },
            if nsync_vs_std > 1.0 {
                "faster"
            } else {
                "slower"
            }
        );
        println!(
            "  SpinLock:     {:.2}x {}",
            if spin_vs_std > 1.0 {
                spin_vs_std
            } else {
                1.0 / spin_vs_std
            },
            if spin_vs_std > 1.0 {
                "faster"
            } else {
                "slower"
            }
        );

        println!();
    }
}

/// Simple counter benchmark for comparison with the original
pub fn run_simple_counter_benchmark(threads: usize, iterations: usize) {
    println!("=== Cosmopolitan-style Mutex Benchmark ===");
    println!("Replicating: https://justine.lol/fastmutex/");
    println!(
        "Threads: {}, Iterations per thread: {}, Total ops: {}",
        threads,
        iterations,
        threads * iterations
    );
    println!("Critical section: Single integer increment (tiny)");
    println!();

    // std::Mutex - exact replication of the C code pattern
    let start = Instant::now();
    let counter = Arc::new(std::sync::Mutex::new(0i32));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = counter.lock().unwrap();
                    *guard += 1;
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let std_time = start.elapsed();
    let std_micros = std_time.as_micros();

    // Verify correctness
    assert_eq!(*counter.lock().unwrap(), (threads * iterations) as i32);

    // nsync::Mutex - same pattern
    let start = Instant::now();
    let counter = Arc::new(nsync_rs::Mutex::new(0i32));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = counter.lock().unwrap();
                    *guard += 1;
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let nsync_time = start.elapsed();
    let nsync_micros = nsync_time.as_micros();

    // Verify correctness
    assert_eq!(*counter.lock().unwrap(), (threads * iterations) as i32);

    // SpinLock - direct usage
    let start = Instant::now();
    let counter = Arc::new(SpinLock::new(0i32));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = counter.lock();
                    *guard += 1;
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let spin_time = start.elapsed();
    let spin_micros = spin_time.as_micros();

    // Verify correctness
    assert_eq!(*counter.lock(), (threads * iterations) as i32);

    // Results in the same format as the blog post
    println!("=== Results (Wall Time) ===");
    println!("{:<20} {:>15}", "Implementation", "Time (μs)");
    println!("{}", "=".repeat(40));
    println!("{:<20} {:>15}", "std::Mutex", format!("{:},", std_micros));
    println!(
        "{:<20} {:>15}",
        "nsync::Mutex",
        format!("{:},", nsync_micros)
    );
    println!("{:<20} {:>15}", "SpinLock", format!("{:},", spin_micros));

    println!();
    println!("=== Performance Analysis ===");

    let nsync_vs_std = std_micros as f64 / nsync_micros as f64;
    let spin_vs_std = std_micros as f64 / spin_micros as f64;
    let nsync_vs_spin = spin_micros as f64 / nsync_micros as f64;

    println!(
        "nsync::Mutex vs std::Mutex: {:.2}x {}",
        if nsync_vs_std > 1.0 {
            nsync_vs_std
        } else {
            1.0 / nsync_vs_std
        },
        if nsync_vs_std > 1.0 {
            "faster"
        } else {
            "slower"
        }
    );

    println!(
        "SpinLock vs std::Mutex:     {:.2}x {}",
        if spin_vs_std > 1.0 {
            spin_vs_std
        } else {
            1.0 / spin_vs_std
        },
        if spin_vs_std > 1.0 {
            "faster"
        } else {
            "slower"
        }
    );

    println!(
        "nsync::Mutex vs SpinLock:   {:.2}x {}",
        if nsync_vs_spin > 1.0 {
            nsync_vs_spin
        } else {
            1.0 / nsync_vs_spin
        },
        if nsync_vs_spin > 1.0 {
            "faster"
        } else {
            "slower"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::SimpleLruCache;

    #[test]
    fn test_basic_functionality() {
        let mut cache = SimpleLruCache::new(3);

        cache.set("key1", "value1");
        cache.set("key2", "value2");

        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"nonexistent"), None);
    }

    #[test]
    fn test_capacity_limit() {
        let mut cache = SimpleLruCache::new(2);

        cache.set("key1", "value1");
        cache.set("key2", "value2");

        // Should have both items
        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));

        // Add third item - should evict oldest (key1)
        cache.set("key3", "value3");

        // key1 should be evicted, key2 and key3 should remain
        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key2"), Some("value2"));
        assert_eq!(cache.get(&"key3"), Some("value3"));
    }

    #[test]
    fn test_lru_ordering() {
        let mut cache = SimpleLruCache::new(3);

        cache.set("a", 1);
        cache.set("b", 2);
        cache.set("c", 3);

        // Access 'a' to make it most recently used
        cache.get(&"a");

        // Add fourth item - 'b' should be evicted (oldest unused)
        cache.set("d", 4);

        assert_eq!(cache.get(&"a"), Some(1)); // Should still be there
        assert_eq!(cache.get(&"b"), None); // Should be evicted
        assert_eq!(cache.get(&"c"), Some(3)); // Should still be there
        assert_eq!(cache.get(&"d"), Some(4)); // Should be there
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = SimpleLruCache::new(2);

        cache.set("key1", "value1");
        cache.set("key2", "value2");

        // Update existing key - should not evict anything
        cache.set("key1", "new_value1");

        assert_eq!(cache.get(&"key1"), Some("new_value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));
    }

    #[test]
    fn test_single_capacity() {
        let mut cache = SimpleLruCache::new(1);

        cache.set("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        // Add second item - should evict first
        cache.set("key2", "value2");
        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key2"), Some("value2"));
    }

    #[test]
    fn test_get_updates_access_time() {
        let mut cache = SimpleLruCache::new(2);

        cache.set("a", 1);
        cache.set("b", 2);

        // Access 'a' to make it more recently used
        cache.get(&"a");

        // Add new item - 'b' should be evicted, not 'a'
        cache.set("c", 3);

        assert_eq!(cache.get(&"a"), Some(1)); // Should still be there
        assert_eq!(cache.get(&"b"), None); // Should be evicted
        assert_eq!(cache.get(&"c"), Some(3)); // Should be there
    }

    #[test]
    fn test_benchmark_pattern() {
        let mut cache = SimpleLruCache::new(5);

        // Simulate benchmark usage pattern
        let keys = ["key1", "key2", "key3", "key4", "key5"];

        // Initial population
        for (i, &key) in keys.iter().enumerate() {
            cache.set(key, i);
        }

        // Repeated access pattern
        for _round in 0..100 {
            for &key in &keys {
                let result = cache.get(&key);
                assert!(result.is_some(), "Key {} should exist", key);
            }
        }
    }

    // #[test]
    // fn test_overflow_safety() {
    //     let mut cache = SimpleLruCache::new(2);

    //     // Set access counter to near overflow
    //     cache.access_counter = 18446744073709551615 - 5;

    //     cache.set("key1", "value1");
    //     cache.set("key2", "value2");

    //     // These operations should handle overflow gracefully
    //     cache.get(&"key1");
    //     cache.get(&"key2");
    //     cache.set("key3", "value3"); // Should trigger eviction

    //     // Should still work correctly
    //     assert!(cache.get(&"key2").is_some() || cache.get(&"key3").is_some());
    // }
}
