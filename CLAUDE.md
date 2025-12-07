# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Purpose

This project translates Agent-Based Model (ABM) research papers into modular Rust simulations using a Discrete Event Simulation (DES) framework. The goal is to recreate economic and social simulation models from academic literature (e.g., Axelrod's cooperation models, Kirman & Vriend's fish market, zero-intelligence traders) as concrete, runnable implementations.

## Prior Art Directory

The `/prior-art` directory contains in-depth summaries of research papers that are candidates for implementation as simulations.

**When to read these summaries:**
- At the start of implementing a new simulation module based on a paper
- Read the primary paper summary first, then related summaries for context (e.g., if implementing Kirman & Vriend, also review other market/agent papers)

**When NOT to read these summaries:**
- General questions about the codebase or DES framework
- Debugging or refactoring existing simulations
- Build/test issues

These files are detailed and should be consulted deliberately when translating research into code.

## Core Architecture

### DES Framework (`des` crate)

The core simulation engine is a generic, event-driven framework:

- **`EventLoop<T, S>`**: Main simulation runner that processes events from a priority queue (BinaryHeap)
  - `T`: Event type (defined per simulation)
  - `S`: Statistics type (defined per simulation)
  - Maintains `current_t` (current simulation time)
  - Broadcasts events to all agents
  - Runs until specified time or queue exhaustion

- **`Agent<T, S>` trait**: Core abstraction for simulation entities
  - `act(&mut self, current_t: usize, data: &T) -> Response<T, S>`: Process events and return new events/agents
  - `stats(&self) -> S`: Return agent statistics for analysis
  - Agents can spawn both new events and new agents dynamically

- **`Response<T, S>`**: Return type from agent actions
  - `events`: Vector of `(usize, T)` tuples (time, event data) to schedule
  - `agents`: Vector of new agents to add to the simulation
  - Convenience methods: `Response::event()`, `Response::events()`

- **`Event<T>`**: Internal wrapper that implements Ord for min-heap behavior (earlier times = higher priority)

### Modular Example Pattern (`simple_queue`)

Each simulation example follows this structure:

1. **Define domain-specific Event enum**: All possible events in the simulation
2. **Define domain-specific Stats types**: Data structures for tracking metrics
3. **Implement Agent trait**: Create concrete agents (e.g., `ConsumerProcess`, `Resource`)
4. **Main simulation setup**: Initialize EventLoop with seed events and agents

Example: `simple_queue` implements a bank counter simulation where:
- `ConsumerProcess` generates consumers with random arrival/service/wait times using probability distributions
- `Resource` manages finite resource allocation with queueing and expiry
- Events: `Start`, `ResourceRequested`, `ResourceAcquired`, `ResourceReleased`, `ResourceRequestExpired`

## Development Commands

### Build and Test
```bash
# Build entire workspace
cargo build

# Build specific crate
cargo build -p des
cargo build -p simple_queue

# Run tests (tests are in des/src/lib.rs)
cargo test

# Run clippy
cargo clippy
```

### Run Simulations
```bash
# Run the simple_queue example
cargo run -p simple_queue
```

### Run Single Test
```bash
# Run specific test by name
cargo test it_works
cargo test min_queue
cargo test noddy_run
```

## Working with This Codebase

### Adding New Simulations

1. Create new crate in workspace (add to root `Cargo.toml` members)
2. Add `des` dependency in new crate's `Cargo.toml`
3. Define Event enum and Stats types in `lib.rs`
4. Implement agents by implementing `des::Agent<Event, Stats>`
5. Create `main.rs` with EventLoop initialization
6. Focus on agent behavior design and event scheduling logic

### Modifying the DES Core

The `des` crate is intentionally minimal. Changes should:
- Preserve the generic Event/Agent/Response architecture
- Maintain time-ordering semantics (events in the past are filtered out)
- Keep the dynamic agent spawning capability
- Not break existing examples

### Research Translation Process

When translating research papers:
1. Identify agent types and their state variables
2. Map paper's events to Event enum variants
3. Determine what statistics to track (Stats types)
4. Implement agent decision-making logic in `act()` method
5. Use probability distributions from `rand`/`rand_distr` for stochastic behavior
6. Test with small-scale runs before full simulations

## Key Implementation Details

- **Time is discrete and measured in `usize`**: All temporal values are non-negative integers
- **Events scheduled in the past are dropped**: See des/src/lib.rs:105-110
- **Agents are trait objects**: Stored as `Vec<Box<dyn Agent<T, S>>>` for heterogeneous collections
- **BinaryHeap ordering**: Event ordering is reversed (line 23) to create min-heap behavior
- **Stats collection**: Call `event_loop.stats()` after `run()` to get `Vec<S>` from all agents
- **Broadcast semantics**: All agents receive all events; agents filter by relevance (see Resource/ConsumerProcess pattern)

## Testing Philosophy: Stats as Observable State

### Event Sourcing Paradigm

This codebase follows an event sourcing approach where:
- **Events** are the source of truth (what happened)
- **Agent state** is private implementation detail (how it's tracked internally)
- **Stats** is the public observable interface (what can be measured)

**Core principle**: If something is worth observing or testing, it belongs in Stats. If it's not in Stats, it's an implementation detail.

### Designing Stats for Testability

Stats should capture both **current state** and **cumulative metrics**:

```rust
pub struct ResourceStats {
    // Configuration (for context)
    pub resource_id: usize,
    pub capacity: usize,

    // Current state (what's happening now)
    pub current_consumer_count: usize,
    pub current_queue_length: usize,

    // Cumulative metrics (what's happened overall)
    pub total_arrivals: usize,
    pub total_acquired: usize,
    pub total_expired: usize,

    // Time aggregates
    pub total_wait_time: usize,
    pub total_consume_time: usize,
}

impl ResourceStats {
    // Semantic query methods make tests readable
    pub fn is_at_capacity(&self) -> bool {
        self.current_consumer_count >= self.capacity
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.current_consumer_count as f64 / self.capacity as f64
    }
}
```

The `stats()` method acts as a **projection** from private state to public observable state:

```rust
fn stats(&self) -> Stats {
    let mut stats = self.stats.clone();
    // Populate current state from private fields
    stats.current_consumer_count = self.consumer_count;
    stats.current_queue_length = self.consumer_queue.len();
    Stats::ResourceStats(stats)
}
```

### Testing Through Stats

Tests verify agent behavior using only the Stats interface, maintaining proper encapsulation:

```rust
#[test]
fn given_full_resource_when_consumer_requests_then_queued() {
    // GIVEN: Resource at capacity
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    let s = resource.stats();
    assert!(s.is_at_capacity());

    // WHEN: Another consumer requests
    resource.act(15, &Event::ResourceRequested(0, 2));

    // THEN: Consumer queued (verified via Stats)
    let s = resource.stats();
    assert_eq!(s.current_queue_length, 1);
    assert_eq!(s.total_arrivals, 2);
}
```

This approach:
- Maintains encapsulation (agent internals remain private)
- Aligns with event sourcing (Stats = projection of event stream)
- Enables refactoring (can change internal data structures without breaking tests)
- Documents observable behavior (Stats shows what the simulation measures)

### Why This Matters for Research

Research papers describe **observable behavior**, not implementation details. By testing through Stats:

1. **Validation**: Verify your implementation matches the paper's observable dynamics
2. **Replication**: Others can verify behavior without knowing internals
3. **Refactoring safety**: Free to optimize implementation while preserving behavior
4. **Documentation**: Stats structure documents what the model measures

Example: A paper describes "queue length stabilizes under constant load" - you can test this directly via `stats.current_queue_length` without knowing whether the queue is implemented as a `VecDeque`, `Vec`, or custom structure.

### Deterministic Testing

For reproducible tests, agent constructors should support seeded RNGs:

```rust
impl ConsumerProcess {
    #[cfg(test)]
    pub fn new_with_seed(
        resource_id: usize,
        seed: u64,
        /* ... parameters ... */
    ) -> Self {
        ConsumerProcess {
            rng: StdRng::seed_from_u64(seed),
            /* ... */
        }
    }
}
```

This enables testing stochastic behavior deterministically.

## Target Papers for Recreation

From README.md research list:
- The Evolution of Cooperation (Axelrod, 1984)
- Kirman and Vriend (2000, 2001) - fish market, loyalty
- Zero-intelligence traders (Gode and Sunder, 1993, 1997)
- K-level cognition models
- Drug addiction models (Agar & Wilson, Hoffer et al., Heard et al.)
- TRANSIMS traffic simulation
- Policy-relevant ABMs (Dawid et al.)
