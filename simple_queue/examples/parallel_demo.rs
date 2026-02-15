//! Demonstration of parallel scenario execution
//!
//! This example shows how to run multiple independent simple_queue scenarios
//! in parallel using the des::parallel module.
//!
//! Run with:
//!   cargo run --example parallel_demo -p simple_queue

use des::parallel::{run_parallel, simple_progress_reporter, ParallelRunner};
use simple_queue::{ConsumerProcess, Event, Resource, Stats};

/// Create an EventLoop for a bank counter simulation
///
/// Note: This example uses non-seeded RNGs, so results will vary between runs.
/// For deterministic parallel execution, agents should use seeded RNGs
/// (e.g., `StdRng::seed_from_u64(seed)`) as shown in CLAUDE.md.
fn create_bank_counter_simulation(
    _scenario_id: usize,
) -> des::EventLoop<simple_queue::Event, simple_queue::Stats> {
    let events = vec![(0, Event::Start)];

    let counter_id = 0;
    let concurrent_customers = 1;

    let agents: Vec<Box<dyn des::Agent<Event, Stats>>> = vec![
        Box::new(ConsumerProcess::new(
            counter_id,
            1.0 / 100.0,
            (120.0, 20.0),
            (20.0, 2.0),
        )),
        Box::new(Resource::new(counter_id, concurrent_customers)),
    ];

    des::EventLoop::new(events, agents)
}

fn main() {
    println!("=== Parallel EventLoop Demo ===\n");

    // Example 1: Simple parallel execution
    println!("Example 1: Running 10 scenarios in parallel (simple API)");
    let start = std::time::Instant::now();

    let results = run_parallel(10, create_bank_counter_simulation, 1000);

    let duration = start.elapsed();

    println!("Completed {} scenarios in {:.2}s", results.len(), duration.as_secs_f64());
    println!("Success rate: {}/{}\n",
        results.iter().filter(|r| r.is_ok()).count(),
        results.len()
    );

    // Example 2: Builder pattern with progress reporting
    println!("Example 2: Running 50 scenarios with progress reporting");
    let start = std::time::Instant::now();

    let results = ParallelRunner::new(50, create_bank_counter_simulation)
        .progress(simple_progress_reporter(10))
        .run(1000);

    let duration = start.elapsed();

    println!("Completed in {:.2}s", duration.as_secs_f64());
    println!("Success rate: {}/{}\n",
        results.iter().filter(|r| r.is_ok()).count(),
        results.len()
    );

    // Example 3: Aggregate statistics across scenarios
    println!("Example 3: Collecting aggregate statistics from 100 scenarios");
    let start = std::time::Instant::now();

    let results = ParallelRunner::new(100, create_bank_counter_simulation)
        .num_threads(4)  // Limit to 4 threads
        .progress(simple_progress_reporter(25))
        .run(1000);

    let duration = start.elapsed();

    // Extract statistics
    let successful_stats: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .collect();

    println!("\n=== Aggregate Results ===");
    println!("Total scenarios: {}", results.len());
    println!("Successful: {}", successful_stats.len());
    println!("Elapsed time: {:.2}s", duration.as_secs_f64());
    println!("Throughput: {:.1} scenarios/sec",
        results.len() as f64 / duration.as_secs_f64()
    );

    // Count Resource and Consumer stats
    let mut resource_count = 0;
    let mut consumer_count = 0;

    for stats in &successful_stats {
        for s in stats.iter() {
            match s {
                Stats::ResourceStats(_) => resource_count += 1,
                Stats::ConsumerStats(_) => consumer_count += 1,
            }
        }
    }

    println!("\nTotal agents across all scenarios:");
    println!("  Resource agents: {}", resource_count);
    println!("  Consumer agents: {}", consumer_count);

    println!("\n=== Note on Determinism ===");
    println!("This example uses non-seeded RNGs, so results vary between runs.");
    println!("For deterministic parallel execution:");
    println!("  1. Agents should use seeded RNGs (e.g., StdRng::seed_from_u64(seed))");
    println!("  2. Builder function should derive seed from scenario_id");
    println!("  3. See CLAUDE.md for full determinism guidelines");
    println!("\nWith proper seeding, running the same scenarios produces");
    println!("identical results regardless of execution order or thread count.");
}
