# Testing Through Stats: Event Sourcing Approach

## Core Insight

In an event-sourced system, **state is derived from events** and should only be observed through public interfaces. For our ABM simulations:

- **Events** = source of truth (what happened)
- **Agent state** = private implementation detail (how it's tracked)
- **Stats** = public observable state (what we can see)

**Key principle**: If something is worth testing, it should be in Stats. If it's not in Stats, it's an implementation detail that doesn't need testing.

---

## Problems with "Make Fields Public" Approach

My initial recommendation to make fields public has several issues:

1. **Breaks encapsulation**: Exposes implementation details
2. **Tight coupling**: Tests depend on internal structure
3. **Fragile**: Refactoring internals breaks tests
4. **Not event-sourcing**: Tests inspect state instead of observing through events

**Better**: Test through the public Stats interface, treating agents as black boxes.

---

## The Stats-First Testing Pattern

### Current Stats Design (Insufficient)

```rust
pub struct ResourceStats {
    arrival_count: usize,      // Cumulative
    acquired_count: usize,     // Cumulative
    expiry_count: usize,       // Cumulative
    consume_sum: usize,        // Cumulative
    wait_sum: usize,           // Cumulative
}
```

**Problem**: Only cumulative metrics, no current state.

**Can't answer**: "Is the resource currently at capacity?" "How many consumers are queued?"

### Redesigned Stats (Complete Observable State)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceStats {
    // Current state (what's happening now)
    pub current_consumer_count: usize,
    pub current_queue_length: usize,
    pub current_active_requests: usize,

    // Cumulative metrics (what's happened overall)
    pub total_arrivals: usize,
    pub total_acquired: usize,
    pub total_expired: usize,
    pub total_denied: usize,

    // Aggregate metrics (derived stats)
    pub total_consume_time: usize,
    pub total_wait_time: usize,
    pub avg_consume_time: f64,
    pub avg_wait_time: f64,

    // Resource configuration (for context)
    pub resource_id: usize,
    pub capacity: usize,
}

impl ResourceStats {
    pub fn is_at_capacity(&self) -> bool {
        self.current_consumer_count >= self.capacity
    }

    pub fn has_queue(&self) -> bool {
        self.current_queue_length > 0
    }

    pub fn utilization(&self) -> f64 {
        self.current_consumer_count as f64 / self.capacity as f64
    }
}
```

**Benefits**:
- ✅ Complete observable state
- ✅ Semantic query methods
- ✅ No private field access needed
- ✅ Stats are self-documenting

---

## Testing Pattern: Given-When-Then via Stats

### Example 1: Immediate Acquisition

```rust
#[test]
fn given_resource_has_capacity_when_consumer_requests_then_immediately_acquired() {
    // GIVEN: Resource with capacity 2
    let mut resource = Resource::new(0, 2);

    // Verify initial state via Stats
    let stats = resource.stats();
    assert_eq!(stats.current_consumer_count, 0);
    assert_eq!(stats.current_queue_length, 0);
    assert!(!stats.is_at_capacity());

    // WHEN: Consumer requests resource at t=10
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    // THEN: Verify response
    assert_eq!(response.events.len(), 1);
    assert!(matches!(
        response.events[0],
        (10, Event::ResourceAcquired(0, 42, 10))
    ));

    // THEN: Verify state change via Stats
    let stats = resource.stats();
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 0);
    assert_eq!(stats.total_arrivals, 1);
    assert_eq!(stats.total_acquired, 1);
    assert!(!stats.is_at_capacity());  // Still has capacity
}
```

**Note**: No field access! Everything through `stats()`.

### Example 2: Queueing Behavior

```rust
#[test]
fn given_full_resource_when_consumer_requests_then_queued() {
    // GIVEN: Resource at capacity
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Verify at capacity via Stats
    let stats = resource.stats();
    assert!(stats.is_at_capacity());
    assert_eq!(stats.current_consumer_count, 1);

    // WHEN: Another consumer requests
    let response = resource.act(15, &Event::ResourceRequested(0, 2));

    // THEN: No immediate acquisition
    assert_eq!(response.events.len(), 0);

    // THEN: Consumer queued (observable via Stats)
    let stats = resource.stats();
    assert_eq!(stats.current_queue_length, 1);
    assert_eq!(stats.current_active_requests, 1);
    assert_eq!(stats.total_arrivals, 2);
    assert_eq!(stats.total_acquired, 1);
}
```

### Example 3: Queue Processing

```rust
#[test]
fn given_queued_consumers_when_resource_released_then_queue_decreases() {
    // GIVEN: Resource with queue
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(17, &Event::ResourceRequested(0, 3));

    let stats = resource.stats();
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 2);

    // WHEN: Consumer releases
    let response = resource.act(25, &Event::ResourceReleased(0, 1, 10));

    // THEN: Next consumer acquires
    assert_eq!(response.events.len(), 1);
    assert!(matches!(
        response.events[0],
        (25, Event::ResourceAcquired(0, 2, 15))
    ));

    // THEN: Queue reduced by one
    let stats = resource.stats();
    assert_eq!(stats.current_consumer_count, 1);
    assert_eq!(stats.current_queue_length, 1);  // Was 2, now 1
    assert_eq!(stats.total_acquired, 2);

    // THEN: Wait time accumulated
    // Consumer 2 waited from t=15 to t=25 = 10 units
    assert_eq!(stats.total_wait_time, 10);
}
```

---

## Implementation: Redesigning Stats

### Step 1: Enrich Stats Structures

**For Resource:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceStats {
    // Identity
    pub resource_id: usize,
    pub capacity: usize,

    // Current state
    pub current_consumer_count: usize,
    pub current_queue_length: usize,
    pub current_active_requests: usize,  // Waiting consumers (before expiry)

    // Cumulative counters
    pub total_arrivals: usize,
    pub total_acquired: usize,
    pub total_expired: usize,
    pub total_released: usize,

    // Time aggregates
    pub total_consume_time: usize,
    pub total_wait_time: usize,
}

impl ResourceStats {
    pub fn new(resource_id: usize, capacity: usize) -> Self {
        ResourceStats {
            resource_id,
            capacity,
            current_consumer_count: 0,
            current_queue_length: 0,
            current_active_requests: 0,
            total_arrivals: 0,
            total_acquired: 0,
            total_expired: 0,
            total_released: 0,
            total_consume_time: 0,
            total_wait_time: 0,
        }
    }

    // Semantic queries
    pub fn is_at_capacity(&self) -> bool {
        self.current_consumer_count >= self.capacity
    }

    pub fn has_queue(&self) -> bool {
        self.current_queue_length > 0
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.current_consumer_count as f64 / self.capacity as f64
    }

    pub fn avg_wait_time(&self) -> Option<f64> {
        if self.total_acquired == 0 { return None; }
        Some(self.total_wait_time as f64 / self.total_acquired as f64)
    }

    pub fn avg_consume_time(&self) -> Option<f64> {
        if self.total_released == 0 { return None; }
        Some(self.total_consume_time as f64 / self.total_released as f64)
    }
}
```

**For ConsumerProcess:**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerStats {
    pub resource_id: usize,
    pub consumers_generated: usize,
    pub next_consumer_id: usize,

    // Could track distributions used, etc.
    pub arrival_interval_mean: f64,
    pub consume_duration_mean: f64,
    pub wait_duration_mean: f64,
}

impl ConsumerStats {
    // Semantic queries as needed
    pub fn generation_rate(&self) -> f64 {
        1.0 / self.arrival_interval_mean
    }
}
```

### Step 2: Update Resource Implementation

```rust
impl Resource {
    pub fn new(resource_id: usize, consumer_total: usize) -> Resource {
        Resource {
            resource_id,
            consumer_total,
            consumer_count: 0,
            consumer_queue: VecDeque::new(),
            consumers_active: HashSet::new(),
            stats: ResourceStats::new(resource_id, consumer_total),
        }
    }
}

impl des::Agent<Event, Stats> for Resource {
    fn stats(&self) -> Stats {
        // Compute current state from private fields
        let mut stats = self.stats.clone();

        // Update current state fields
        stats.current_consumer_count = self.consumer_count;
        stats.current_queue_length = self.consumer_queue.len();
        stats.current_active_requests = self.consumers_active.len();

        Stats::ResourceStats(stats)
    }

    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event, Stats> {
        match data {
            Event::ResourceRequested(rid, cid) => {
                if rid != &self.resource_id {
                    return des::Response::new();
                }

                println!("[{}] Consumer {} requested Resource {}", current_t, cid, rid);

                // Update stats
                self.stats.total_arrivals += 1;

                if self.consumer_total == self.consumer_count {
                    // Queue the consumer
                    self.consumer_queue.push_back((*cid, current_t));
                    self.consumers_active.insert(*cid);
                    des::Response::new()
                } else {
                    // Grant immediate access
                    self.consumer_count += 1;
                    self.stats.total_acquired += 1;
                    des::Response::event(
                        current_t,
                        Event::ResourceAcquired(*rid, *cid, current_t)
                    )
                }
            }

            Event::ResourceReleased(rid, cid, acquired_t) => {
                if rid != &self.resource_id {
                    return des::Response::new();
                }

                println!("[{}] Consumer {} released Resource {}", current_t, cid, rid);

                // Update stats
                self.stats.total_consume_time += current_t - acquired_t;
                self.stats.total_released += 1;

                self.consumer_count -= 1;

                // Process queue
                while let Some((consumer_id, requested_t)) = self.consumer_queue.pop_front() {
                    if self.consumers_active.contains(&consumer_id) {
                        self.consumers_active.remove(&consumer_id);
                        self.consumer_count += 1;

                        // Update stats
                        self.stats.total_wait_time += current_t - requested_t;
                        self.stats.total_acquired += 1;

                        return des::Response::event(
                            current_t,
                            Event::ResourceAcquired(*rid, consumer_id, requested_t),
                        );
                    }
                }

                des::Response::new()
            }

            Event::ResourceRequestExpired(rid, cid, requested_t) => {
                if rid != &self.resource_id {
                    return des::Response::new();
                }

                let removed = self.consumers_active.remove(cid);

                if removed {
                    println!("[{}] Consumer {} request expired", current_t, cid);

                    // Update stats
                    self.stats.total_expired += 1;
                    self.stats.total_wait_time += current_t - requested_t;
                }

                des::Response::new()
            }

            _ => des::Response::new(),
        }
    }
}
```

**Key changes**:
- Stats fields remain private in Resource
- `stats()` computes current state from private fields
- Stats are updated in `act()` as events are processed
- Tests only access Stats, never private fields

---

## Complete Test Examples

### Test 1: State Transitions via Stats

```rust
#[test]
fn resource_state_transitions_observable_via_stats() {
    let mut resource = Resource::new(0, 2);

    // State 1: Empty
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 0);
    assert!(!s.is_at_capacity());

    // Transition: First consumer arrives
    resource.act(10, &Event::ResourceRequested(0, 1));

    // State 2: One consumer
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 1);
    assert_eq!(s.total_arrivals, 1);
    assert_eq!(s.total_acquired, 1);

    // Transition: Second consumer arrives
    resource.act(15, &Event::ResourceRequested(0, 2));

    // State 3: At capacity
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 2);
    assert!(s.is_at_capacity());
    assert_eq!(s.utilization(), 1.0);

    // Transition: Third consumer arrives
    resource.act(20, &Event::ResourceRequested(0, 3));

    // State 4: Queueing
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 2);
    assert_eq!(s.current_queue_length, 1);
    assert!(s.has_queue());

    // Transition: First consumer releases
    resource.act(30, &Event::ResourceReleased(0, 1, 10));

    // State 5: Queue processed
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 2);  // Still at capacity
    assert_eq!(s.current_queue_length, 0);    // Queue cleared
    assert_eq!(s.total_released, 1);
    assert_eq!(s.total_wait_time, 10);  // Consumer 3 waited 20->30
}
```

### Test 2: Metrics Accumulation

```rust
#[test]
fn resource_metrics_accumulate_correctly() {
    let mut resource = Resource::new(0, 1);

    // Consumer 1: arrives at 10, releases at 40 (consumed for 30)
    resource.act(10, &Event::ResourceRequested(0, 1));
    resource.act(40, &Event::ResourceReleased(0, 1, 10));

    let s = resource.stats();
    assert_eq!(s.total_consume_time, 30);
    assert_eq!(s.total_released, 1);
    assert_eq!(s.avg_consume_time(), Some(30.0));

    // Consumer 2: arrives at 45, releases at 95 (consumed for 50)
    resource.act(45, &Event::ResourceRequested(0, 2));
    resource.act(95, &Event::ResourceReleased(0, 2, 45));

    let s = resource.stats();
    assert_eq!(s.total_consume_time, 80);  // 30 + 50
    assert_eq!(s.total_released, 2);
    assert_eq!(s.avg_consume_time(), Some(40.0));  // (30 + 50) / 2
}
```

### Test 3: Event Sequence Verification

```rust
#[test]
fn resource_expiry_observable_in_stats() {
    let mut resource = Resource::new(0, 1);

    // Occupy resource
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Queue two consumers
    resource.act(15, &Event::ResourceRequested(0, 2));
    resource.act(20, &Event::ResourceRequested(0, 3));

    let s = resource.stats();
    assert_eq!(s.current_queue_length, 2);
    assert_eq!(s.total_arrivals, 3);

    // Consumer 2 expires
    resource.act(35, &Event::ResourceRequestExpired(0, 2, 15));

    let s = resource.stats();
    assert_eq!(s.total_expired, 1);
    assert_eq!(s.total_wait_time, 20);  // Consumer 2 waited 15->35

    // Consumer 1 releases - Consumer 3 should acquire (2 expired)
    resource.act(40, &Event::ResourceReleased(0, 1, 10));

    let s = resource.stats();
    assert_eq!(s.total_released, 1);
    assert_eq!(s.total_acquired, 2);  // Consumer 1 and 3 (not 2)
    assert_eq!(s.current_queue_length, 0);  // Consumer 2 skipped
}
```

---

## Benefits of Stats-Based Testing

### 1. **True Encapsulation**
- Agent internals remain private
- Only public interface (Stats) is tested
- Free to refactor internals

### 2. **Event Sourcing Alignment**
- Tests verify observable behavior, not implementation
- Stats = projection of event stream onto observable state
- Matches conceptual model

### 3. **Self-Documenting**
- Stats structure documents what's observable
- Semantic methods (`is_at_capacity()`) clarify meaning
- Tests read like specifications

### 4. **Refactoring Safety**
- Can change internal data structures freely
- As long as Stats stay correct, tests pass
- Example: Switch from VecDeque to Vec - tests don't care

### 5. **Research Alignment**
- Papers describe observable behavior, not implementation
- Stats capture what papers measure
- Tests verify model matches paper

---

## Design Pattern: Stats as Contract

Think of Stats as a **contract** between agent and outside world:

```
┌─────────────────────────────────────────┐
│  Agent (Private Implementation)         │
│  ┌────────────────────────────────────┐ │
│  │  Private Fields:                   │ │
│  │  - consumer_count                  │ │
│  │  - consumer_queue                  │ │
│  │  - consumers_active                │ │
│  └────────────────────────────────────┘ │
│               │                          │
│               │ Encapsulation Boundary   │
│               │                          │
│               ▼                          │
│  ┌────────────────────────────────────┐ │
│  │  Public Interface (Stats):         │ │
│  │  - current_consumer_count          │ │
│  │  - current_queue_length            │ │
│  │  - is_at_capacity()                │ │
│  │  - utilization()                   │ │
│  └────────────────────────────────────┘ │
└─────────────────────────────────────────┘
                │
                │ Tests interact here
                ▼
         ┌─────────────┐
         │   Tests     │
         └─────────────┘
```

**Agent promises**: "I will maintain these observable properties (Stats)"
**Tests verify**: "Does agent keep its promises?"
**Implementation**: "Private, can change freely"

---

## Migration Path

### Phase 1: Enrich Stats (simple_queue)
1. Expand ResourceStats with current state fields
2. Update `Resource::stats()` to populate current state
3. Add semantic query methods
4. Update ConsumerStats similarly

### Phase 2: Rewrite Tests Using Stats
1. Rewrite example tests to use only Stats
2. Remove any field access
3. Use semantic methods for clarity
4. Verify tests still verify same behavior

### Phase 3: Prove Encapsulation
1. Make all Resource fields private (if not already)
2. Tests should still pass
3. Try refactoring internals
4. Tests should remain green

### Phase 4: Apply Pattern
1. Document pattern in des crate
2. Apply to evolving_market
3. Apply to evolution_coop
4. Create Stats design guidelines

---

## Stats Design Guidelines

When designing Stats for an agent:

### 1. Include Current State
Not just cumulative counters - what's happening RIGHT NOW?
```rust
pub current_consumer_count: usize,  // ✅
pub total_arrivals: usize,          // ✅ (both!)
```

### 2. Include Configuration
Tests need context:
```rust
pub resource_id: usize,
pub capacity: usize,
```

### 3. Add Semantic Methods
Don't make tests compute derived values:
```rust
pub fn is_at_capacity(&self) -> bool { ... }
pub fn utilization(&self) -> f64 { ... }
```

### 4. Include All Observable State
If you're tempted to access a field in a test, add it to Stats:
```rust
// Test wants: resource.consumer_queue.len()
// Solution: Add to Stats
pub current_queue_length: usize,
```

### 5. Make Stats Comparable
Derive PartialEq for snapshot comparisons:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceStats { ... }
```

### 6. Consider Time
For time-series analysis:
```rust
pub last_update_time: usize,
pub time_series: Vec<(usize, StateSnapshot)>,  // Optional
```

---

## Advanced: Event Sourcing Test Pattern

For complex scenarios, treat test as event log replay:

```rust
#[test]
fn scenario_resource_lifecycle() {
    let mut resource = Resource::new(0, 1);

    // Event log
    let events = vec![
        (10, Event::ResourceRequested(0, 1)),
        (15, Event::ResourceRequested(0, 2)),
        (20, Event::ResourceRequested(0, 3)),
        (30, Event::ResourceRequestExpired(0, 2, 15)),
        (40, Event::ResourceReleased(0, 1, 10)),
    ];

    // Expected stats at each point
    let expected_stats = vec![
        (10, |s: &ResourceStats| {
            s.current_consumer_count == 1 && s.total_acquired == 1
        }),
        (15, |s: &ResourceStats| {
            s.current_queue_length == 1 && s.current_active_requests == 1
        }),
        (20, |s: &ResourceStats| {
            s.current_queue_length == 2 && s.total_arrivals == 3
        }),
        (30, |s: &ResourceStats| {
            s.total_expired == 1 && s.current_queue_length == 2
        }),
        (40, |s: &ResourceStats| {
            s.total_released == 1 && s.current_queue_length == 0
        }),
    ];

    // Replay and verify
    for ((t, event), (_, predicate)) in events.iter().zip(expected_stats.iter()) {
        resource.act(*t, event);
        let stats = resource.stats();
        assert!(predicate(&stats), "Stats mismatch at t={}", t);
    }
}
```

---

## Comparison: Field Access vs Stats-Based

### Field Access (❌ Original Approach)
```rust
#[test]
fn test_queueing() {
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Accesses private fields
    assert_eq!(resource.consumer_count, 1);
    assert_eq!(resource.consumer_queue.len(), 0);

    resource.act(15, &Event::ResourceRequested(0, 2));

    // Tightly coupled to implementation
    assert!(resource.consumers_active.contains(&2));
}
```

**Problems**:
- Requires public fields
- Coupled to internal structure
- Brittle to refactoring

### Stats-Based (✅ Better Approach)
```rust
#[test]
fn test_queueing() {
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    // Uses public Stats interface
    let s = resource.stats();
    assert_eq!(s.current_consumer_count, 1);
    assert_eq!(s.current_queue_length, 0);

    resource.act(15, &Event::ResourceRequested(0, 2));

    // Tests observable behavior
    let s = resource.stats();
    assert_eq!(s.current_queue_length, 1);
    assert_eq!(s.current_active_requests, 1);
}
```

**Benefits**:
- Maintains encapsulation
- Tests observable behavior
- Resilient to refactoring

---

## Conclusion

**Key insight**: Stats is not just for end-of-simulation metrics - it's the **complete public interface** for observable agent state.

**Design principle**: If something is worth observing, it belongs in Stats. If it's not in Stats, it's an implementation detail.

**Testing philosophy**: Test through Stats only. Never access agent internals.

**Result**:
- ✅ Proper encapsulation
- ✅ Event sourcing alignment
- ✅ Refactoring safety
- ✅ Self-documenting code
- ✅ Research credibility

This is a **much better** approach than making fields public.
