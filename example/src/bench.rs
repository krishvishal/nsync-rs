use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub const OPS_PER_THREAD: usize = 100_000;

pub const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8, 16];
pub const CONTENTION_LEVELS: &[(&str, usize, usize)] = &[
    ("Low", 100, 1),     // 100μs work, 1μs critical section
    ("Medium", 10, 1),   // 10μs work, 1μs critical section
    ("High", 1, 1),      // 1μs work, 1μs critical section
    ("VeryHigh", 0, 10), // No work, 10μs critical section
];

pub(crate) fn benchmark_std_mutex(
    num_threads: usize,
    work_us: usize,
    critical_us: usize,
) -> Duration {
    use std::sync::Mutex;

    let counter = Arc::new(Mutex::new(0u64));
    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..OPS_PER_THREAD {
                    // Simulate work outside critical section
                    if work_us > 0 {
                        busy_wait_us(work_us);
                    }

                    // Critical section
                    let mut guard = counter.lock().unwrap();
                    *guard += 1;
                    if critical_us > 0 {
                        busy_wait_us(critical_us);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

pub fn benchmark_nsync_mutex(num_threads: usize, work_us: usize, critical_us: usize) -> Duration {
    use nsync_rs::Mutex;

    let counter = Arc::new(Mutex::new(0u64));
    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..OPS_PER_THREAD {
                    // Simulate work outside critical section
                    if work_us > 0 {
                        busy_wait_us(work_us);
                    }

                    // Critical section
                    let mut guard = counter.lock().unwrap();
                    *guard += 1;
                    if critical_us > 0 {
                        busy_wait_us(critical_us);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

pub fn benchmark_std_rwlock(num_threads: usize, work_us: usize, critical_us: usize) -> Duration {
    use std::sync::RwLock;

    let data = Arc::new(RwLock::new(vec![0u64; 100]));
    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let data = Arc::clone(&data);
            thread::spawn(move || {
                for j in 0..OPS_PER_THREAD {
                    if work_us > 0 {
                        busy_wait_us(work_us);
                    }

                    // 90% reads, 10% writes
                    if j % 10 == 0 {
                        let mut guard = data.write().unwrap();
                        guard[i % 100] += 1;
                        if critical_us > 0 {
                            busy_wait_us(critical_us);
                        }
                    } else {
                        let guard = data.read().unwrap();
                        let _ = guard[i % 100];
                        if critical_us > 0 {
                            busy_wait_us(critical_us);
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

pub fn benchmark_nsync_rwlock(num_threads: usize, work_us: usize, critical_us: usize) -> Duration {
    use nsync_rs::RwLock;

    let data = Arc::new(RwLock::new(vec![0u64; 100]));
    let start = Instant::now();

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let data = Arc::clone(&data);
            thread::spawn(move || {
                for j in 0..OPS_PER_THREAD {
                    if work_us > 0 {
                        busy_wait_us(work_us);
                    }

                    // 90% reads, 10% writes
                    if j % 10 == 0 {
                        let mut guard = data.write().unwrap();
                        guard[i % 100] += 1;
                        if critical_us > 0 {
                            busy_wait_us(critical_us);
                        }
                    } else {
                        let guard = data.read().unwrap();
                        let _ = guard[i % 100];
                        if critical_us > 0 {
                            busy_wait_us(critical_us);
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

// Busy wait for approximately the given number of microseconds
fn busy_wait_us(us: usize) {
    let start = Instant::now();
    let duration = Duration::from_micros(us as u64);
    while start.elapsed() < duration {
        std::hint::spin_loop();
    }
}
