# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Purpose

This project translates Agent-Based Model (ABM) research papers into modular Rust simulations using a Discrete Event Simulation (DES) framework. The goal is to recreate economic and social simulation models from academic literature (e.g., Axelrod's cooperation models, Kirman & Vriend's fish market, zero-intelligence traders) as concrete, runnable implementations.

## Quick Start

```bash
# Build the workspace
cargo build

# Run tests
cargo test

# Run example simulation
cargo run -p simple_queue

# Run specific tests
cargo test min_queue
```

## Repository Structure

```
abm/
├── des/              # Core DES framework (generic event loop)
├── simple_queue/     # Example: Bank counter simulation
├── zi_traders/       # Example: Zero-intelligence trader market
├── prior-art/        # Paper summaries for implementation
└── CLAUDE.md         # This file
```

## Prior Art Directory

The `/prior-art` directory contains in-depth summaries of research papers that are candidates for implementation as simulations.

**When to read these summaries:**
- **MUST READ**: Before implementing a simulation based on a paper (read the primary paper summary first, then related papers for context)
- **Example**: Implementing Kirman & Vriend? Read `prior-art/kirman-vriend-2001.md` first, then related market/agent papers for additional context

**Skip these summaries when:**
- Working on the DES framework itself
- Debugging or refactoring existing simulations
- Build/test/tooling issues
- General questions about the codebase

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

## Auto-Approved Commands

> **Note**: This section lists commands that are automatically approved for Claude Code. Human readers can skip this section.

The following commands can be executed without requiring permission prompts. These are safe, commonly-used operations for this project.

### Git Commands (Read-Only and Safe Operations)
- `git status` - Check working tree status
- `git diff` - View changes
- `git log` - View commit history
- `git show` - Show commit details
- `git branch` - List/view branches
- `git ls-files` - List tracked files
- `git rev-parse` - Parse git revision info
- `git describe` - Describe commits
- `git tag` - List tags (read-only)

### GitHub CLI Commands (Read-Only Operations)
- `gh pr view` - View pull request details
- `gh pr list` - List pull requests
- `gh pr status` - Check PR status
- `gh pr checks` - View PR check status
- `gh issue view` - View issue details
- `gh issue list` - List issues
- `gh repo view` - View repository info
- `gh workflow view` - View workflow details
- `gh run list` - List workflow runs
- `gh run view` - View workflow run details
- `gh api` - Make read-only API calls

### Cargo Commands
- `cargo build` - Build the workspace or specific packages
- `cargo build -p <package>` - Build specific package
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast compilation check
- `cargo clippy` - Run linter
- `cargo fmt` - Format code
- `cargo run` - Run simulations
- `cargo run -p <package>` - Run specific package
- `cargo tree` - View dependency tree
- `cargo bench` - Run benchmarks
- `cargo doc` - Generate documentation

## Development Workflows

### Common Tasks

```bash
# Build and test everything
cargo build && cargo test

# Run linter and formatter
cargo clippy && cargo fmt

# Run specific simulation
cargo run -p simple_queue
cargo run -p zi_traders

# Build specific package
cargo build -p des

# Run specific test by name
cargo test min_queue
```

## Git Workflow and Commit Guidelines

### Pre-commit Hooks and Testing

**Important**: Always run pre-commit hooks (`git commit` without `--no-verify`). These catch issues early and maintain code quality.

If hooks are slow, improve the testing strategy rather than bypassing safety checks:

1. **Put long-running tests behind feature flags** to separate quick validation from exhaustive testing
2. **Move expensive checks to CI** rather than pre-commit hooks
3. **Optimize slow tests** to run faster
4. **Use `cargo test --lib`** for quick unit test feedback

**Why pre-commit hooks matter**: They prevent broken code from entering the repository, maintain CI stability, and catch issues before they become expensive to fix. If hooks feel like friction, that's a signal to improve the test suite, not to skip validation.

**Example**: Feature-flag slow tests:

```rust
#[test]
#[cfg(feature = "slow-tests")]
fn expensive_simulation_convergence_test() {
    // Long-running simulation test
}
```

Run quick tests during development:
```bash
cargo test
```

Run comprehensive tests before pushing:
```bash
cargo test --all-features
```

## Working with This Codebase

### Adding New Simulations

1. Create new crate in workspace (add to root `Cargo.toml` members)
2. Add `des` dependency in new crate's `Cargo.toml`
3. Define Event enum and Stats types in `lib.rs`
4. Implement agents by implementing `des::Agent<Event, Stats>`
5. Create `main.rs` with EventLoop initialization
6. Focus on agent behavior design and event scheduling logic

### Agent Design Patterns

**TL;DR**: Use Pattern 1 (all-entity agents) by default. Only use Pattern 2 (coordinator + entities) when the domain has an explicit central mechanism (markets, auctions, games).

When implementing a simulation, choose between two patterns based on your domain model:

#### Pattern 1: All-Entity Agents (Recommended Default)

Use when all agents represent actual entities in the domain that make autonomous decisions.

**Example**: `simple_queue`
- `ConsumerProcess`: Generates consumer arrivals (entity behavior)
- `Resource`: Bank counter managing capacity (entity in the system)

**Characteristics**:
- Every agent represents something in the real system
- Agents interact via events (e.g., ResourceRequested, ResourceReleased)
- No central orchestrator needed
- Emergent behavior from agent interactions

**When to use**:
- Multi-agent systems where entities interact directly
- Simulations modeling emergent phenomena
- When there's no central coordinator in the real system

#### Pattern 2: Coordinator + Entity Agents

Use when the domain has a clear separation between mechanism and participants.

**Example**: `zi_traders`
- `Coordinator`: Market mechanism (orchestration, not a participant)
- `Traders`: Actual market participants making decisions

**Characteristics**:
- Coordinator implements system rules/mechanism
- Coordinator maintains shared state (order book, active participants)
- Entities respond to coordinator events (OrderRequest, Transaction)
- Coordinator handles turn-taking, resource allocation, rule enforcement

**When to use**:
- Simulations of markets, auctions, games (clear mechanism + participants)
- When the paper/model explicitly separates mechanism from agents
- When strict turn-taking or centralized state management is required

**Tradeoffs**:
- ✅ Matches domain model (mechanism vs participants)
- ✅ Easier to enforce system rules centrally
- ✅ Clear separation of concerns
- ⚠️ Broadcast overhead (coordinator receives events it may ignore)
- ⚠️ Shadow state maintenance (coordinator tracks participant state)
- ⚠️ More complex event flow (request → response → notification cycles)

**Implementation guidelines for Coordinator pattern**:
- Coordinator receives ALL events via broadcast; filter for relevant ones
- Use consistent event timing (schedule next iteration at `current_t + 1`)
- Maintain per-participant state in HashMaps for O(1) lookups, not linear searches
- Use `HashSet` for active participant tracking (O(1) add/remove vs Vec)
- Add module documentation explaining why Coordinator is an Agent
- Consider adding timeout detection for non-responding participants

**Key insight**: The pattern choice should reflect the domain model. If the real system has a central mechanism (market, auction house, game referee), use Pattern 2. If the real system is fully decentralized (interacting entities), use Pattern 1.

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
- **Events scheduled in the past are dropped**: The event loop silently ignores events scheduled before `current_t` (see `EventLoop::run()` implementation)
- **Agents are trait objects**: Stored as `Vec<Box<dyn Agent<T, S>>>` for heterogeneous collections
- **BinaryHeap ordering**: Event ordering is reversed in the `Ord` implementation to create min-heap behavior (earlier times = higher priority)
- **Stats collection**: Call `event_loop.stats()` after `run()` to get `Vec<S>` from all agents
- **Broadcast semantics**: All agents receive all events; agents filter by relevance (see Resource/ConsumerProcess pattern)

## Target Papers for Recreation

From README.md research list:
- The Evolution of Cooperation (Axelrod, 1984)
- Kirman and Vriend (2000, 2001) - fish market, loyalty
- Zero-intelligence traders (Gode and Sunder, 1993, 1997)
- K-level cognition models
- Drug addiction models (Agar & Wilson, Hoffer et al., Heard et al.)
- TRANSIMS traffic simulation
- Policy-relevant ABMs (Dawid et al.)
