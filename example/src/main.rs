mod bench;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "shootout" | "lru" => {
                // Run the full LRU cache mutex shootout
                bench::run_mutex_shootout();
            }
            "counter" => {
                // Run simple counter benchmark for quick comparison
                let threads = if args.len() > 2 {
                    args[2].parse().unwrap_or(8)
                } else {
                    8
                };
                let iterations = if args.len() > 3 {
                    args[3].parse().unwrap_or(100_000)
                } else {
                    100_000
                };
                bench::run_simple_counter_benchmark(threads, iterations);
            }
            "help" | "-h" | "--help" => {
                print_help();
            }
            _ => {
                println!("Unknown command: {}", args[1]);
                print_help();
            }
        }
    } else {
        // Default: run both benchmarks
        println!("Running both benchmark suites...\n");

        // First run the simple counter benchmark
        bench::run_simple_counter_benchmark(8, 100_000);

        println!("\n{}\n", "=".repeat(60));

        // Then run the full LRU mutex shootout
        bench::run_mutex_shootout();
    }
}

fn print_help() {
    println!("nsync-rs Mutex Benchmark Suite");
    println!();
    println!("USAGE:");
    println!("    cargo run [COMMAND] [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    shootout, lru              Run the full LRU cache mutex shootout");
    println!("    counter [threads] [iters]  Run simple counter benchmark");
    println!("    help, -h, --help          Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    cargo run                    # Run both benchmarks with defaults");
    println!("    cargo run shootout           # Run only the LRU cache benchmark");
    println!("    cargo run counter            # Run counter benchmark (8 threads, 100k iters)");
    println!("    cargo run counter 16 500000  # Run counter benchmark (16 threads, 500k iters)");
    println!();
    println!("BENCHMARK DESCRIPTIONS:");
    println!();
    println!("1. Simple Counter Benchmark:");
    println!("   - Multiple threads incrementing a shared counter");
    println!("   - Minimal critical section (single integer increment)");
    println!("   - Tests raw mutex performance under high contention");
    println!("   - Quick to run, good for initial performance comparison");
    println!();
    println!("2. LRU Cache Mutex Shootout:");
    println!("   - Based on Mark Waterman's C++ mutex benchmark");
    println!("   - Multiple threads accessing a shared LRU cache");
    println!("   - More realistic workload with larger critical sections");
    println!("   - Tests mutex performance with real data structures");
    println!("   - Includes cache population and lookup operations");
    println!("   - Uses UUID keys and 2KB payloads");
    println!();
    println!("MUTEX IMPLEMENTATIONS TESTED:");
    println!("   - std::Mutex     : Rust standard library mutex");
    println!("   - nsync::Mutex   : Google nsync mutex (from Cosmopolitan)");
    println!("   - SpinLock       : Simple spin lock implementation");
    println!();
    println!("EXPECTED RESULTS:");
    println!("   - nsync should excel in high-contention scenarios");
    println!("   - std::Mutex should be well-optimized for typical Rust workloads");
    println!("   - SpinLock may perform well with very short critical sections");
    println!("   - Performance differences depend heavily on thread count and workload");
}
