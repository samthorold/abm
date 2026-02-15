//! Parallel execution of independent EventLoop scenarios
//!
//! This module provides utilities for running multiple simulation scenarios concurrently,
//! maintaining deterministic results and graceful error handling.
//!
//! # Example: Running 100 parameter sweeps
//!
//! ```rust
//! use des::parallel::{ParallelRunner, simple_progress_reporter};
//! # use des::{Agent, EventLoop, Response};
//! # struct TestAgent;
//! # #[derive(Clone)]
//! # enum TestStats { A }
//! # impl Agent<u8, TestStats> for TestAgent {
//! #     fn stats(&self) -> TestStats { TestStats::A }
//! # }
//!
//! let results = ParallelRunner::new(100, |scenario_id| {
//!     let seed = 42 + scenario_id as u64;
//!     // Create agents with seed for determinism
//!     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
//!     EventLoop::new(vec![(0, 1)], agents)
//! })
//! .progress(simple_progress_reporter(10))
//! .num_threads(8)
//! .run(10000);
//!
//! // Process results
//! for (id, result) in results.iter().enumerate() {
//!     match result {
//!         Ok(stats) => println!("Scenario {} completed with {} agents", id, stats.len()),
//!         Err(e) => eprintln!("Scenario {} failed: {}", id, e),
//!     }
//! }
//! ```
//!
//! # Determinism
//!
//! Results are deterministic when:
//! 1. Builder function uses `scenario_id` to derive unique seeds
//! 2. Agents use seeded RNGs (e.g., `StdRng::seed_from_u64(seed)`)
//! 3. No shared mutable state across scenarios
//!
//! Running the same scenarios twice produces identical results regardless of
//! execution order or thread count.
//!
//! # Error Handling
//!
//! Panics in individual scenarios are caught and returned as `Err(String)`.
//! Other scenarios continue executing normally. This prevents one bad seed
//! from crashing an entire batch of simulations.
//!
//! # Memory Usage
//!
//! Each scenario runs in its own thread and holds its EventLoop in memory
//! during execution. For large batches, consider using `run_batched()` to
//! limit concurrent scenarios and reduce peak memory usage.

use crate::EventLoop;
use rayon::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Executes multiple EventLoop scenarios in parallel
///
/// Generic over:
/// - `T`: Event type
/// - `S`: Stats type
/// - `F`: Builder function type
///
/// # Type Bounds
///
/// The builder function `F` must be:
/// - `Fn(usize) -> EventLoop<T, S>`: Takes scenario_id, returns fresh EventLoop
/// - `Send + Sync`: Safe to call from multiple threads
///
/// # Example
///
/// ```rust
/// use des::parallel::ParallelRunner;
/// # use des::{Agent, EventLoop, Response};
/// # struct TestAgent;
/// # #[derive(Clone)]
/// # enum TestStats { A }
/// # impl Agent<u8, TestStats> for TestAgent {
/// #     fn stats(&self) -> TestStats { TestStats::A }
/// # }
///
/// let results = ParallelRunner::new(50, |scenario_id| {
///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
///     EventLoop::new(vec![(0, 1)], agents)
/// })
/// .num_threads(4)
/// .run(1000);
///
/// assert_eq!(results.len(), 50);
/// ```
pub struct ParallelRunner<T, S, F>
where
    F: Fn(usize) -> EventLoop<T, S> + Send + Sync,
    S: Send,
{
    num_scenarios: usize,
    builder: F,
    num_threads: Option<usize>,
    progress_callback: Option<Arc<dyn Fn(usize, usize) + Send + Sync>>,
}

impl<T, S, F> ParallelRunner<T, S, F>
where
    F: Fn(usize) -> EventLoop<T, S> + Send + Sync,
    S: Send,
{
    /// Create a new parallel runner
    ///
    /// # Arguments
    ///
    /// * `num_scenarios` - Number of independent scenarios to run
    /// * `builder` - Closure that creates a fresh EventLoop for given scenario_id
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::parallel::ParallelRunner;
    /// # use des::{Agent, EventLoop};
    /// # struct TestAgent;
    /// # #[derive(Clone)]
    /// # enum TestStats { A }
    /// # impl Agent<u8, TestStats> for TestAgent {
    /// #     fn stats(&self) -> TestStats { TestStats::A }
    /// # }
    ///
    /// let runner = ParallelRunner::new(10, |scenario_id| {
    ///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
    ///     EventLoop::new(vec![(scenario_id, 1)], agents)
    /// });
    /// ```
    pub fn new(num_scenarios: usize, builder: F) -> Self {
        ParallelRunner {
            num_scenarios,
            builder,
            num_threads: None,
            progress_callback: None,
        }
    }

    /// Set number of threads (defaults to rayon's global pool)
    ///
    /// By default, rayon uses a thread pool sized to the number of CPU cores.
    /// Use this method to override that behavior.
    ///
    /// # Arguments
    ///
    /// * `n` - Number of worker threads to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::parallel::ParallelRunner;
    /// # use des::{Agent, EventLoop};
    /// # struct TestAgent;
    /// # #[derive(Clone)]
    /// # enum TestStats { A }
    /// # impl Agent<u8, TestStats> for TestAgent {
    /// #     fn stats(&self) -> TestStats { TestStats::A }
    /// # }
    ///
    /// let runner = ParallelRunner::new(100, |_| {
    ///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
    ///     EventLoop::new(vec![(0, 1)], agents)
    /// })
    /// .num_threads(4);  // Limit to 4 threads
    /// ```
    pub fn num_threads(mut self, n: usize) -> Self {
        self.num_threads = Some(n);
        self
    }

    /// Set progress callback (called after each scenario completes)
    ///
    /// The callback receives `(completed_count, total_count)` after each scenario.
    /// Use this for progress reporting, logging, or updating UI.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function called with (completed, total) after each scenario
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::parallel::ParallelRunner;
    /// # use des::{Agent, EventLoop};
    /// # struct TestAgent;
    /// # #[derive(Clone)]
    /// # enum TestStats { A }
    /// # impl Agent<u8, TestStats> for TestAgent {
    /// #     fn stats(&self) -> TestStats { TestStats::A }
    /// # }
    ///
    /// let runner = ParallelRunner::new(100, |_| {
    ///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
    ///     EventLoop::new(vec![(0, 1)], agents)
    /// })
    /// .progress(|completed, total| {
    ///     if completed % 10 == 0 {
    ///         println!("Progress: {}/{}", completed, total);
    ///     }
    /// });
    /// ```
    pub fn progress<P>(mut self, callback: P) -> Self
    where
        P: Fn(usize, usize) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Arc::new(callback));
        self
    }

    /// Execute all scenarios and return results in order
    ///
    /// Runs all scenarios in parallel, collecting results in scenario_id order.
    /// Failed scenarios (panics) are captured and returned as `Err(String)`.
    ///
    /// # Arguments
    ///
    /// * `run_until` - Simulation end time to pass to `EventLoop::run()`
    ///
    /// # Returns
    ///
    /// Vector of results in scenario_id order:
    /// - `Ok(Vec<S>)` for successful scenarios (stats from all agents)
    /// - `Err(String)` for panicked scenarios
    ///
    /// # Example
    ///
    /// ```rust
    /// use des::parallel::ParallelRunner;
    /// # use des::{Agent, EventLoop};
    /// # struct TestAgent;
    /// # #[derive(Clone)]
    /// # enum TestStats { A }
    /// # impl Agent<u8, TestStats> for TestAgent {
    /// #     fn stats(&self) -> TestStats { TestStats::A }
    /// # }
    ///
    /// let results = ParallelRunner::new(10, |_| {
    ///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
    ///     EventLoop::new(vec![(0, 1)], agents)
    /// })
    /// .run(1000);
    ///
    /// assert_eq!(results.len(), 10);
    /// assert!(results.iter().all(|r| r.is_ok()));
    /// ```
    pub fn run(self, run_until: usize) -> Vec<Result<Vec<S>, String>> {
        let progress_counter = Arc::new(AtomicUsize::new(0));

        // Build thread pool if custom size requested
        let pool = self.num_threads.map(|n| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .expect("Failed to create thread pool")
        });

        let execute = || {
            (0..self.num_scenarios)
                .into_par_iter()
                .map(|scenario_id| {
                    // Run scenario with panic catching
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut event_loop = (self.builder)(scenario_id);
                        event_loop.run(run_until);
                        event_loop.stats()
                    }));

                    // Update progress
                    let completed = progress_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    if let Some(ref callback) = self.progress_callback {
                        callback(completed, self.num_scenarios);
                    }

                    // Convert panic to Err
                    result.map_err(|panic| {
                        if let Some(s) = panic.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "Unknown panic".to_string()
                        }
                    })
                })
                .collect()
        };

        if let Some(pool) = pool {
            pool.install(execute)
        } else {
            execute()
        }
    }
}

/// Run scenarios in parallel with simple API
///
/// Convenience function for common case: run N scenarios, collect results.
/// For more control (thread count, progress reporting), use `ParallelRunner`.
///
/// # Arguments
///
/// * `num_scenarios` - Number of independent scenarios to run
/// * `builder` - Closure that creates EventLoop for given scenario_id
/// * `run_until` - Simulation end time
///
/// # Returns
///
/// Vector of results in scenario_id order
///
/// # Example
///
/// ```rust
/// use des::parallel::run_parallel;
/// # use des::{Agent, EventLoop};
/// # struct TestAgent;
/// # #[derive(Clone)]
/// # enum TestStats { A }
/// # impl Agent<u8, TestStats> for TestAgent {
/// #     fn stats(&self) -> TestStats { TestStats::A }
/// # }
///
/// let results = run_parallel(100, |scenario_id| {
///     let seed = scenario_id as u64;
///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
///     EventLoop::new(vec![(0, 1)], agents)
/// }, 10000);
///
/// assert_eq!(results.len(), 100);
/// ```
pub fn run_parallel<T, S, F>(
    num_scenarios: usize,
    builder: F,
    run_until: usize,
) -> Vec<Result<Vec<S>, String>>
where
    F: Fn(usize) -> EventLoop<T, S> + Send + Sync,
    S: Send,
{
    ParallelRunner::new(num_scenarios, builder).run(run_until)
}

/// Run scenarios in batches to limit memory usage
///
/// For large-scale experiments (1000+ scenarios), running all scenarios
/// concurrently can consume excessive memory. This function processes
/// scenarios in batches, limiting peak memory usage.
///
/// # Arguments
///
/// * `num_scenarios` - Total number of scenarios to run
/// * `batch_size` - Maximum concurrent scenarios per batch
/// * `builder` - Closure that creates EventLoop for given scenario_id
/// * `run_until` - Simulation end time
///
/// # Returns
///
/// Vector of all results in scenario_id order
///
/// # Example
///
/// ```rust
/// use des::parallel::run_batched;
/// # use des::{Agent, EventLoop};
/// # struct TestAgent;
/// # #[derive(Clone)]
/// # enum TestStats { A }
/// # impl Agent<u8, TestStats> for TestAgent {
/// #     fn stats(&self) -> TestStats { TestStats::A }
/// # }
///
/// // Run 10000 scenarios, but only 100 concurrent at a time
/// let results = run_batched(10000, 100, |scenario_id| {
///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
///     EventLoop::new(vec![(0, 1)], agents)
/// }, 1000);
///
/// assert_eq!(results.len(), 10000);
/// ```
pub fn run_batched<T, S, F>(
    num_scenarios: usize,
    batch_size: usize,
    builder: F,
    run_until: usize,
) -> Vec<Result<Vec<S>, String>>
where
    F: Fn(usize) -> EventLoop<T, S> + Send + Sync,
    S: Send,
{
    let mut all_results = Vec::with_capacity(num_scenarios);

    for batch_start in (0..num_scenarios).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(num_scenarios);
        let batch_results = run_parallel(
            batch_end - batch_start,
            |local_id| builder(batch_start + local_id),
            run_until,
        );
        all_results.extend(batch_results);
    }

    all_results
}

/// Pre-built progress reporter for common use case
///
/// Creates a progress callback that prints updates at regular intervals.
///
/// # Arguments
///
/// * `interval` - Print progress every N completed scenarios
///
/// # Returns
///
/// Closure suitable for `ParallelRunner::progress()`
///
/// # Example
///
/// ```rust
/// use des::parallel::{ParallelRunner, simple_progress_reporter};
/// # use des::{Agent, EventLoop};
/// # struct TestAgent;
/// # #[derive(Clone)]
/// # enum TestStats { A }
/// # impl Agent<u8, TestStats> for TestAgent {
/// #     fn stats(&self) -> TestStats { TestStats::A }
/// # }
///
/// let results = ParallelRunner::new(1000, |_| {
///     let agents: Vec<Box<dyn Agent<u8, TestStats>>> = vec![Box::new(TestAgent)];
///     EventLoop::new(vec![(0, 1)], agents)
/// })
/// .progress(simple_progress_reporter(100))  // Print every 100 scenarios
/// .run(10000);
/// ```
pub fn simple_progress_reporter(interval: usize) -> impl Fn(usize, usize) + Send + Sync {
    move |completed, total| {
        if completed % interval == 0 || completed == total {
            println!("  Completed {}/{} scenarios", completed, total);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Agent;

    // Minimal test agent for unit tests
    struct CounterAgent {
        id: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct CounterStats {
        id: usize,
        count: usize,
    }

    impl Agent<u8, CounterStats> for CounterAgent {
        fn stats(&self) -> CounterStats {
            CounterStats {
                id: self.id,
                count: 10,
            }
        }
    }

    #[test]
    fn test_parallel_basic() {
        let results = run_parallel(
            10,
            |scenario_id| {
                let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                    vec![Box::new(CounterAgent { id: scenario_id })];
                EventLoop::new(vec![(0, 1)], agents)
            },
            100,
        );

        assert_eq!(results.len(), 10);
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok());
            let stats = result.as_ref().unwrap();
            assert_eq!(stats[0].id, i);
        }
    }

    #[test]
    fn test_parallel_determinism() {
        let builder = |scenario_id| {
            let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                vec![Box::new(CounterAgent { id: scenario_id })];
            EventLoop::new(vec![(0, 1)], agents)
        };

        let run1 = run_parallel(20, builder, 100);
        let run2 = run_parallel(20, builder, 100);

        // Results should be identical
        assert_eq!(run1.len(), run2.len());
        for (r1, r2) in run1.iter().zip(run2.iter()) {
            assert_eq!(r1, r2);
        }
    }

    #[test]
    fn test_parallel_panic_isolation() {
        let results = run_parallel(
            10,
            |scenario_id| {
                if scenario_id == 5 {
                    panic!("Test panic");
                }
                let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                    vec![Box::new(CounterAgent { id: scenario_id })];
                EventLoop::new(vec![(0, 1)], agents)
            },
            100,
        );

        assert_eq!(results.len(), 10);
        assert!(results[5].is_err());
        for (i, result) in results.iter().enumerate() {
            if i != 5 {
                assert!(result.is_ok());
            }
        }
    }

    #[test]
    fn test_parallel_progress_callback() {
        use std::sync::Mutex;
        let completed = Arc::new(Mutex::new(0));
        let completed_clone = completed.clone();

        ParallelRunner::new(5, |scenario_id| {
            let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                vec![Box::new(CounterAgent { id: scenario_id })];
            EventLoop::new(vec![(0, 1)], agents)
        })
        .progress(move |count, _total| {
            *completed_clone.lock().unwrap() = count;
        })
        .run(100);

        assert_eq!(*completed.lock().unwrap(), 5);
    }

    #[test]
    fn test_parallel_custom_threads() {
        let results = ParallelRunner::new(8, |scenario_id| {
            let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                vec![Box::new(CounterAgent { id: scenario_id })];
            EventLoop::new(vec![(0, 1)], agents)
        })
        .num_threads(2)
        .run(100);

        assert_eq!(results.len(), 8);
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[test]
    fn test_batched_execution() {
        let results = run_batched(
            50,
            10,
            |scenario_id| {
                let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                    vec![Box::new(CounterAgent { id: scenario_id })];
                EventLoop::new(vec![(0, 1)], agents)
            },
            100,
        );

        assert_eq!(results.len(), 50);
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok());
            let stats = result.as_ref().unwrap();
            assert_eq!(stats[0].id, i);
        }
    }

    #[test]
    fn test_simple_progress_reporter() {
        // Just verify it compiles and runs without panicking
        let reporter = simple_progress_reporter(10);
        reporter(10, 100);
        reporter(100, 100);
    }

    #[test]
    fn test_empty_scenarios() {
        let results = run_parallel(
            0,
            |_scenario_id| {
                let agents: Vec<Box<dyn Agent<u8, CounterStats>>> = vec![];
                EventLoop::new(vec![], agents)
            },
            100,
        );

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_result_ordering_preserved() {
        // Verify that results[i] corresponds to scenario_id=i
        let results = run_parallel(
            100,
            |scenario_id| {
                let agents: Vec<Box<dyn Agent<u8, CounterStats>>> =
                    vec![Box::new(CounterAgent { id: scenario_id })];
                EventLoop::new(vec![(0, 1)], agents)
            },
            100,
        );

        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok());
            let stats = result.as_ref().unwrap();
            assert_eq!(stats[0].id, i, "Result ordering not preserved");
        }
    }
}
