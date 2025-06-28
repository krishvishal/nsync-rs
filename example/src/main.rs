mod bench;
use nsync_rs::{Condvar, Counter, Mutex, Once, RwLock, Time};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::bench::{
    CONTENTION_LEVELS, OPS_PER_THREAD, THREAD_COUNTS, benchmark_nsync_mutex,
    benchmark_nsync_rwlock, benchmark_std_mutex, benchmark_std_rwlock,
};

fn main() {
    println!("=== nsync vs std::sync Benchmark ===");
    println!("Operations per thread: {}", OPS_PER_THREAD);
    println!();

    // Warm up
    println!("Warming up...");
    benchmark_std_mutex(2, 10, 1);
    benchmark_nsync_mutex(2, 10, 1);

    // Mutex benchmarks
    println!("\n--- Mutex Benchmarks ---");
    for &(contention_name, work_us, critical_us) in CONTENTION_LEVELS {
        println!(
            "\nContention: {} ({}μs work, {}μs critical)",
            contention_name, work_us, critical_us
        );
        println!("Threads  std::Mutex   nsync::Mutex  Speedup");
        println!("-------  ----------   ------------  -------");

        for &num_threads in THREAD_COUNTS {
            let std_time = benchmark_std_mutex(num_threads, work_us, critical_us);
            let nsync_time = benchmark_nsync_mutex(num_threads, work_us, critical_us);

            let speedup = std_time.as_secs_f64() / nsync_time.as_secs_f64();
            let faster = if speedup > 1.0 { "faster" } else { "slower" };

            println!(
                "{:7}  {:>10.3}ms  {:>10.3}ms  {:.2}x {}",
                num_threads,
                std_time.as_secs_f64() * 1000.0,
                nsync_time.as_secs_f64() * 1000.0,
                speedup.max(1.0 / speedup),
                faster
            );
        }
    }

    // RwLock benchmarks
    println!("\n--- RwLock Benchmarks (90% reads, 10% writes) ---");
    for &(contention_name, work_us, critical_us) in CONTENTION_LEVELS {
        println!(
            "\nContention: {} ({}μs work, {}μs critical)",
            contention_name, work_us, critical_us
        );
        println!("Threads  std::RwLock  nsync::RwLock  Speedup");
        println!("-------  -----------  -------------  -------");

        for &num_threads in THREAD_COUNTS {
            let std_time = benchmark_std_rwlock(num_threads, work_us, critical_us);
            let nsync_time = benchmark_nsync_rwlock(num_threads, work_us, critical_us);

            let speedup = std_time.as_secs_f64() / nsync_time.as_secs_f64();
            let faster = if speedup > 1.0 { "faster" } else { "slower" };

            println!(
                "{:7}  {:>11.3}ms  {:>11.3}ms  {:.2}x {}",
                num_threads,
                std_time.as_secs_f64() * 1000.0,
                nsync_time.as_secs_f64() * 1000.0,
                speedup.max(1.0 / speedup),
                faster
            );
        }
    }

    println!("\nBenchmark complete!");
}

fn mutex_example() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final count: {}", *counter.lock().unwrap());
}

fn rwlock_example() {
    let lock = Arc::new(RwLock::new(vec![1, 2, 3]));
    let mut handles = vec![];

    // Spawn readers
    for i in 0..3 {
        let lock = Arc::clone(&lock);
        let handle = thread::spawn(move || {
            let data = lock.read().unwrap();
            println!("Reader {} sees: {:?}", i, *data);
        });
        handles.push(handle);
    }

    // Spawn a writer
    let lock_clone = Arc::clone(&lock);
    let writer = thread::spawn(move || {
        let mut data = lock_clone.write().unwrap();
        data.push(4);
        println!("Writer added 4");
    });
    handles.push(writer);

    for handle in handles {
        handle.join().unwrap();
    }
}

fn condvar_example() {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = Arc::clone(&pair);

    thread::spawn(move || {
        let (lock, cvar) = &*pair2;
        let mut started = lock.lock().unwrap();
        *started = true;
        cvar.notify_one();
        println!("Notified!");
    });

    let (lock, cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }
    println!("Received notification!");
}

#[allow(static_mut_refs)]
fn once_example() {
    static INIT: Once = Once::new();
    static mut VAL: usize = 0;

    let handles: Vec<_> = (0..10)
        .map(|_| {
            thread::spawn(|| {
                INIT.call_once(|| {
                    unsafe {
                        VAL = 42;
                    }
                    println!("Initialized once!");
                });
                unsafe {
                    println!("Value: {}", VAL);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

fn counter_example() {
    let counter = Arc::new(Counter::new(5));
    let counter2 = Arc::clone(&counter);

    thread::spawn(move || {
        for _i in 0..5 {
            thread::sleep(Duration::from_millis(100));
            let new_val = counter2.add(-1);
            println!("Decremented to: {}", new_val);
        }
    });

    println!("Waiting for counter to reach zero...");
    counter.wait(Time::no_deadline());
    println!("Counter reached zero!");
}
