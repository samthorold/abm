# Agent Behavior Testing Strategy

## Current State Analysis

### Existing Tests

**des crate (DES framework)**
- 5 infrastructure tests in `des/src/lib.rs:132-231`
- Focus: Event ordering, EventLoop mechanics, agent/event spawning
- No agent behavior testing

**evolving_market crate**
- 5 tests in `evolving_market/src/lib.rs:267-346`
- Focus: Pure functions (stochastic auction, loyalty updates, metrics)
- No agent behavior or integration testing

**simple_queue crate**
- Zero tests
- Only runnable via `main.rs` with manual inspection of output

**evolution_coop crate**
- Not examined, likely similar pattern

### Key Gaps

1. **No Given-When-Then behavioral tests**: Tests don't verify agent state transitions
2. **No integration tests**: No tests verifying multi-agent interactions
3. **Limited observability**: Agent state is private, only exposed via `stats()` at simulation end
4. **Non-deterministic by default**: Agents use `rand::rng()` without seed control
5. **Testing requires full simulation**: Can't easily test individual agent responses in isolation

---

## Proposed Testing Approach: Given-When-Then for Agent Behavior

### Core Concept

Apply BDD-style testing to agent state machines:

```
GIVEN: Initial agent state + simulation context (current_t, event history)
WHEN:  An event is broadcast to the agent
THEN:  Verify response (new events, new agents, internal state changes)
```

### Three Levels of Testing

#### Level 1: Unit - Individual Agent Response Testing
Test a single agent's response to specific events.

**Example: Resource allocation**
```rust
#[test]
fn given_resource_has_capacity_when_consumer_requests_then_immediately_acquired() {
    // GIVEN: Resource with capacity 2, no active consumers
    let mut resource = Resource::new(resource_id: 0, capacity: 2);

    // WHEN: Consumer 42 requests resource at t=10
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    // THEN: Resource immediately grants access
    assert_eq!(response.events.len(), 1);
    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 10);
            assert_eq!(*rid, 0);
            assert_eq!(*cid, 42);
            assert_eq!(*req_t, 10);
        }
        _ => panic!("Expected ResourceAcquired event"),
    }

    // THEN: Resource internal state updated
    assert_eq!(resource.consumer_count, 1);
    assert_eq!(resource.stats.acquired_count, 1);
}

#[test]
fn given_full_resource_when_consumer_requests_then_queued() {
    // GIVEN: Resource at full capacity
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // WHEN: Second consumer requests resource
    let response = resource.act(15, &Event::ResourceRequested(0, 2));

    // THEN: No immediate acquisition (consumer queued)
    assert_eq!(response.events.len(), 0);
    assert_eq!(resource.consumer_queue.len(), 1);
    assert!(resource.consumers_active.contains(&2));
}

#[test]
fn given_queued_consumers_when_resource_released_then_next_consumer_acquires() {
    // GIVEN: Resource at capacity with queue
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(17, &Event::ResourceRequested(0, 3));

    // WHEN: First consumer releases at t=25
    let response = resource.act(25, &Event::ResourceReleased(0, 1, 10));

    // THEN: Next queued consumer (id=2) acquires resource
    assert_eq!(response.events.len(), 1);
    match &response.events[0] {
        (t, Event::ResourceAcquired(rid, cid, req_t)) => {
            assert_eq!(*t, 25);
            assert_eq!(*cid, 2);
            assert_eq!(*req_t, 15);
        }
        _ => panic!("Expected ResourceAcquired"),
    }

    // THEN: Wait time tracked correctly
    // Consumer 2 waited from t=15 to t=25 = 10 time units
    assert_eq!(resource.stats.wait_sum, 10);
}
```

**Requirements for Level 1 testing:**
- Agent fields must be `pub` or provide inspection methods
- Stats tracking must be testable mid-simulation, not just at end
- Need builder pattern or test constructors for complex agent states

#### Level 2: Integration - Multi-Agent Interaction Testing
Test sequences of events involving multiple agents.

**Example: Consumer-Resource interaction**
```rust
#[test]
fn given_consumer_process_and_resource_when_simulation_runs_then_consumers_acquire_and_release() {
    // GIVEN: Deterministic consumer process + resource
    let events = vec![(0, Event::Start)];
    let agents: Vec<Box<dyn Agent<Event, Stats>>> = vec![
        Box::new(ConsumerProcess::new_with_seed(
            resource_id: 0,
            seed: 42,
            // deterministic parameters
        )),
        Box::new(Resource::new(0, 1)),
    ];
    let mut event_loop = EventLoop::new(events, agents);

    // WHEN: Simulation runs for 100 time units
    event_loop.run(100);

    // THEN: Expected number of acquisitions occurred
    let stats = event_loop.stats();
    if let Stats::ResourceStats(resource_stats) = &stats[1] {
        assert!(resource_stats.acquired_count > 0);
        assert!(resource_stats.acquired_count <= 100 / expected_interval);
    }
}
```

**Example: Market price formation**
```rust
#[test]
fn given_buyers_and_sellers_when_session_completes_then_transactions_occur_at_market_clearing_price() {
    // GIVEN: 3 buyers with reservation prices [100, 80, 60]
    //        2 sellers with costs [40, 50]
    let agents = vec![
        Box::new(Buyer::new_deterministic(0, reservation_price: 100)),
        Box::new(Buyer::new_deterministic(1, reservation_price: 80)),
        Box::new(Buyer::new_deterministic(2, reservation_price: 60)),
        Box::new(Seller::new_deterministic(0, cost: 40)),
        Box::new(Seller::new_deterministic(1, cost: 50)),
    ];

    let events = vec![(0, MarketEvent::SessionStart { day: 1, session: Morning })];
    let mut event_loop = EventLoop::new(events, agents);

    // WHEN: Session completes
    event_loop.run(100);

    // THEN: Expected transactions occurred
    // Buyer 0 & 1 should transact (reservation > cost)
    // Buyer 2 should not (reservation 60 < seller cost 40/50 doesn't guarantee match)
    let stats = event_loop.stats();
    let total_transactions: usize = stats.iter()
        .filter_map(|s| match s {
            MarketStats { transactions_completed, .. } => Some(*transactions_completed),
            _ => None,
        })
        .sum();

    assert!(total_transactions >= 2);
}
```

**Requirements for Level 2 testing:**
- Deterministic agent behavior (seeded RNGs)
- Ability to inspect EventLoop state mid-simulation
- Rich assertion helpers for common patterns

#### Level 3: Scenario - End-to-End Behavioral Testing
Test emergent properties and research hypotheses.

**Example: Queue stability under load**
```rust
#[test]
fn scenario_queue_reaches_steady_state_under_constant_load() {
    // GIVEN: Long-running simulation with specific arrival/service rates
    let events = vec![(0, Event::Start)];
    let agents = vec![
        Box::new(ConsumerProcess::new_with_seed(
            resource_id: 0,
            seed: 42,
            arrival_rate: 1.0 / 50.0,  // avg 50 time units between arrivals
            service_rate: (40.0, 5.0), // avg 40 units service time
        )),
        Box::new(Resource::new(0, capacity: 1)),
    ];

    let mut event_loop = EventLoop::new(events, agents);

    // WHEN: Simulation runs for 10,000 time units
    event_loop.run(10_000);

    // THEN: System reaches steady state
    // Average queue length should stabilize (ρ = λ/μ < 1 for stability)
    // In this case: λ = 1/50, μ = 1/40, ρ = 40/50 = 0.8 (stable)
    let stats = event_loop.stats();
    if let Stats::ResourceStats(rs) = &stats[1] {
        let utilization = rs.acquired_count as f64 / (10_000.0 / 40.0);
        assert!(utilization > 0.7 && utilization < 0.9);

        // Average wait time should be bounded
        let avg_wait = rs.wait_sum as f64 / rs.acquired_count as f64;
        assert!(avg_wait > 0.0 && avg_wait < 200.0);
    }
}
```

**Example: Loyalty concentration emerges over time**
```rust
#[test]
fn scenario_loyalty_concentration_increases_with_experience() {
    // GIVEN: Market with buyers using loyalty-based seller selection
    let agents = setup_market_with_loyalty_learning();
    let mut event_loop = EventLoop::new(vec![(0, SessionStart)], agents);

    // WHEN: Market runs for 100 days
    let mut daily_concentrations = Vec::new();
    for day in 1..=100 {
        event_loop.run(day * 100); // each day = 100 time units

        // Measure loyalty concentration
        let stats = event_loop.stats();
        let avg_concentration = calculate_avg_loyalty_concentration(&stats);
        daily_concentrations.push(avg_concentration);
    }

    // THEN: Concentration increases over time (buyers become more loyal)
    let early_avg = daily_concentrations[0..20].iter().sum::<f64>() / 20.0;
    let late_avg = daily_concentrations[80..100].iter().sum::<f64>() / 20.0;

    assert!(late_avg > early_avg);
    assert!(late_avg > 0.5); // Significant concentration emerges
}
```

**Requirements for Level 3 testing:**
- Long-running simulation support
- Snapshot/checkpointing for mid-simulation inspection
- Statistical assertion helpers
- Visualization/data export for debugging failures

---

## Architectural Changes for Testability

### 1. Deterministic Testing Support

**Current problem**: Agents use `rand::rng()` directly, making behavior non-reproducible.

**Solution**: Inject RNG or seed via agent construction.

```rust
// Before (in simple_queue)
fn draw_arrival_interval(&self) -> usize {
    self.arrival_interval.sample(&mut rand::rng()) as usize
}

// After
pub struct ConsumerProcess {
    resource_id: usize,
    rng: Box<dyn RngCore>, // or StdRng for seeded testing
    // ... other fields
}

impl ConsumerProcess {
    pub fn new_with_seed(resource_id: usize, seed: u64, ...) -> Self {
        ConsumerProcess {
            rng: Box::new(StdRng::seed_from_u64(seed)),
            // ...
        }
    }
}
```

### 2. Observable Agent State

**Current problem**: Agent state is private, only `stats()` is public but designed for end-of-simulation.

**Solution Option A**: Make fields pub for testing
```rust
pub struct Resource {
    pub resource_id: usize,
    pub consumer_count: usize,
    pub consumer_queue: VecDeque<(usize, usize)>,
    pub consumers_active: HashSet<usize>,
    pub stats: ResourceStats,
}
```

**Solution Option B**: Add inspection methods
```rust
impl Resource {
    #[cfg(test)]
    pub fn test_state(&self) -> ResourceTestState {
        ResourceTestState {
            consumer_count: self.consumer_count,
            queue_len: self.consumer_queue.len(),
            active_count: self.consumers_active.len(),
        }
    }
}
```

**Solution Option C**: Rich stats that work mid-simulation
```rust
pub struct ResourceStats {
    // Cumulative (current design)
    pub arrival_count: usize,
    pub acquired_count: usize,

    // Current state (new)
    pub current_consumer_count: usize,
    pub current_queue_length: usize,
    pub current_active_requests: usize,
}
```

**Recommendation**: Use Option A (pub fields) initially for simplicity. This is an ABM research codebase, not a library with stability guarantees.

### 3. Test Fixture Builders

Create builder pattern for complex agent setup in tests.

```rust
#[cfg(test)]
pub mod test_fixtures {
    use super::*;

    pub struct ResourceBuilder {
        resource_id: usize,
        capacity: usize,
        initial_consumers: Vec<usize>,
        initial_queue: VecDeque<(usize, usize)>,
    }

    impl ResourceBuilder {
        pub fn new(id: usize) -> Self {
            ResourceBuilder {
                resource_id: id,
                capacity: 1,
                initial_consumers: Vec::new(),
                initial_queue: VecDeque::new(),
            }
        }

        pub fn with_capacity(mut self, n: usize) -> Self {
            self.capacity = n;
            self
        }

        pub fn with_active_consumer(mut self, consumer_id: usize) -> Self {
            self.initial_consumers.push(consumer_id);
            self
        }

        pub fn with_queued_consumer(mut self, consumer_id: usize, requested_t: usize) -> Self {
            self.initial_queue.push_back((consumer_id, requested_t));
            self
        }

        pub fn build(self) -> Resource {
            let mut resource = Resource::new(self.resource_id, self.capacity);
            resource.consumer_count = self.initial_consumers.len();
            resource.consumer_queue = self.initial_queue;
            for cid in self.initial_consumers {
                resource.consumers_active.insert(cid);
            }
            resource
        }
    }
}

// Usage in tests
#[test]
fn test_queue_with_expiry() {
    let mut resource = ResourceBuilder::new(0)
        .with_capacity(1)
        .with_active_consumer(1)
        .with_queued_consumer(2, 10)
        .with_queued_consumer(3, 15)
        .build();

    // ... test logic
}
```

### 4. Simulation Test Harness

Create helper to reduce boilerplate in integration tests.

```rust
#[cfg(test)]
pub struct SimulationTest<T, S> {
    event_loop: EventLoop<T, S>,
    snapshots: Vec<Vec<S>>,
}

impl<T, S: Clone> SimulationTest<T, S> {
    pub fn new(events: Vec<(usize, T)>, agents: Vec<Box<dyn Agent<T, S>>>) -> Self {
        SimulationTest {
            event_loop: EventLoop::new(events, agents),
            snapshots: Vec::new(),
        }
    }

    pub fn run_until(&mut self, t: usize) -> &mut Self {
        self.event_loop.run(t);
        self
    }

    pub fn snapshot(&mut self) -> &mut Self {
        self.snapshots.push(self.event_loop.stats());
        self
    }

    pub fn assert_stats<F>(&self, predicate: F) -> &Self
    where F: Fn(&[S]) -> bool
    {
        assert!(predicate(&self.event_loop.stats()));
        self
    }

    pub fn current_stats(&self) -> Vec<S> {
        self.event_loop.stats()
    }
}

// Usage
#[test]
fn test_with_harness() {
    SimulationTest::new(vec![(0, Event::Start)], vec![...])
        .run_until(100)
        .snapshot()
        .assert_stats(|stats| {
            // assertions on stats
            true
        })
        .run_until(200)
        .assert_stats(|stats| {
            // more assertions
            true
        });
}
```

### 5. Event Sequence Assertions

Helper to verify event sequences in responses.

```rust
#[cfg(test)]
pub mod test_assertions {
    use super::*;

    pub struct ResponseAssertion<T, S> {
        response: Response<T, S>,
    }

    impl<T, S> ResponseAssertion<T, S> {
        pub fn new(response: Response<T, S>) -> Self {
            ResponseAssertion { response }
        }

        pub fn has_events(self, count: usize) -> Self {
            assert_eq!(self.response.events.len(), count);
            self
        }

        pub fn has_no_events(self) -> Self {
            self.has_events(0)
        }

        pub fn has_agents(self, count: usize) -> Self {
            assert_eq!(self.response.agents.len(), count);
            self
        }

        pub fn event_at<F>(self, index: usize, predicate: F) -> Self
        where F: Fn(&(usize, T)) -> bool
        {
            assert!(predicate(&self.response.events[index]));
            self
        }
    }
}

// Usage
#[test]
fn test_with_assertions() {
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    ResponseAssertion::new(response)
        .has_events(1)
        .event_at(0, |(t, event)| {
            *t == 10 && matches!(event, Event::ResourceAcquired(0, 42, 10))
        });
}
```

---

## Implementation Roadmap

### Phase 1: Foundation (simple_queue)
1. Make agent fields public in `simple_queue/src/lib.rs`
2. Add deterministic constructors with RNG seeding
3. Write 5-10 Level 1 tests for Resource and ConsumerProcess
4. Write 2-3 Level 2 integration tests

**Deliverable**: `simple_queue` has comprehensive behavioral tests demonstrating the approach.

### Phase 2: Generalize (des crate)
1. Add test utilities to `des` crate:
   - `SimulationTest` harness
   - Assertion helpers
   - Documentation on testing patterns
2. Update `des` README with testing examples

**Deliverable**: Reusable testing infrastructure for all simulations.

### Phase 3: Apply to Complex Simulations
1. Apply pattern to `evolving_market`
   - Test buyer seller selection logic
   - Test price formation
   - Test loyalty evolution scenarios
2. Apply to `evolution_coop`
3. Document testing approach in main README

**Deliverable**: All simulations have behavioral tests.

### Phase 4: Advanced (Optional)
1. Snapshot/checkpointing for long simulations
2. Property-based testing with `proptest`
3. Statistical test helpers (distribution testing, convergence tests)
4. Visual debugging (export simulation traces for visualization)

---

## Example Test Suite Structure

```
simple_queue/
├── src/
│   ├── lib.rs (agents + events)
│   └── main.rs (runnable simulation)
└── tests/
    ├── resource_tests.rs
    │   ├── test_immediate_acquisition
    │   ├── test_queueing_when_full
    │   ├── test_queue_processing_on_release
    │   ├── test_expiry_removes_from_queue
    │   └── test_stats_tracking
    ├── consumer_process_tests.rs
    │   ├── test_generates_consumers_on_start
    │   ├── test_schedules_release_on_acquisition
    │   └── test_deterministic_with_seed
    ├── integration_tests.rs
    │   ├── test_consumer_resource_lifecycle
    │   ├── test_multiple_consumers_queue_fairness
    │   └── test_expiry_and_release_interaction
    └── scenarios/
        ├── steady_state_test.rs
        └── queue_stability_test.rs
```

---

## Benefits of This Approach

1. **Verifiable behavior**: Can test specific agent logic without running full simulations
2. **Regression prevention**: Changes to agent logic are caught by tests
3. **Documentation**: Tests serve as executable specifications
4. **Faster development**: Quick feedback loop for agent behavior changes
5. **Research validation**: Scenario tests can verify hypotheses from papers
6. **Debugging**: Pinpoint exactly which agent behavior is incorrect
7. **Confidence in refactoring**: Safe to improve code structure

---

## Open Questions

1. **Should stats() be called multiple times?** Currently it clones data. For frequent calls, consider returning references or separating "current state" from "cumulative stats".

2. **Event matching ergonomics**: Pattern matching on events is verbose. Consider event assertion macros?

3. **Async/parallel testing**: If simulations become large, might need parallel test execution strategies.

4. **Fuzzing/property testing**: Should we invest in `proptest` for invariant testing (e.g., "queue length never exceeds capacity")?

5. **Test data size**: Level 3 scenario tests may generate large amounts of data. Need strategy for test data management.
