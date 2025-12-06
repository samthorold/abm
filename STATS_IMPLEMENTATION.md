# Implementing Rich Stats for Testability

This document shows the concrete implementation changes needed to make `simple_queue` testable via Stats without exposing private fields.

---

## Changes Required

### 1. Expand ResourceStats Structure

**File**: `simple_queue/src/lib.rs`

**Current ResourceStats**:
```rust
#[derive(Debug, Clone)]
pub struct ResourceStats {
    arrival_count: usize,
    acquired_count: usize,
    expiry_count: usize,
    consume_sum: usize,
    wait_sum: usize,
}
```

**New ResourceStats**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceStats {
    // Configuration (for context)
    pub resource_id: usize,
    pub capacity: usize,

    // Current state (what's happening now)
    pub current_consumer_count: usize,
    pub current_queue_length: usize,
    pub current_active_requests: usize,

    // Cumulative counters (what's happened overall)
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

    // Semantic query methods
    pub fn is_at_capacity(&self) -> bool {
        self.current_consumer_count >= self.capacity
    }

    pub fn has_queue(&self) -> bool {
        self.current_queue_length > 0
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.current_consumer_count as f64 / self.capacity as f64
    }

    pub fn avg_wait_time(&self) -> Option<f64> {
        if self.total_acquired == 0 {
            return None;
        }
        Some(self.total_wait_time as f64 / self.total_acquired as f64)
    }

    pub fn avg_consume_time(&self) -> Option<f64> {
        if self.total_released == 0 {
            return None;
        }
        Some(self.total_consume_time as f64 / self.total_released as f64)
    }
}
```

**Key additions**:
- Configuration fields (resource_id, capacity)
- Current state fields (current_consumer_count, current_queue_length, current_active_requests)
- Added total_released counter
- Renamed fields for clarity (arrival_count → total_arrivals)
- Semantic query methods
- PartialEq derivation for snapshot comparisons

---

### 2. Update Resource Constructor

**Current**:
```rust
impl Resource {
    pub fn new(resource_id: usize, consumer_total: usize) -> Resource {
        Resource {
            resource_id,
            consumer_total,
            consumer_count: 0,
            consumer_queue: VecDeque::new(),
            consumers_active: HashSet::new(),
            stats: ResourceStats::new(),  // Old signature
        }
    }
}
```

**Updated**:
```rust
impl Resource {
    pub fn new(resource_id: usize, consumer_total: usize) -> Resource {
        Resource {
            resource_id,
            consumer_total,
            consumer_count: 0,
            consumer_queue: VecDeque::new(),
            consumers_active: HashSet::new(),
            stats: ResourceStats::new(resource_id, consumer_total),  // New signature
        }
    }
}
```

---

### 3. Update stats() Method

**Current**:
```rust
impl des::Agent<Event, Stats> for Resource {
    fn stats(&self) -> Stats {
        Stats::ResourceStats(self.stats.clone())
    }
    // ...
}
```

**Updated** (compute current state from private fields):
```rust
impl des::Agent<Event, Stats> for Resource {
    fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();

        // Populate current state from private fields
        stats.current_consumer_count = self.consumer_count;
        stats.current_queue_length = self.consumer_queue.len();
        stats.current_active_requests = self.consumers_active.len();

        Stats::ResourceStats(stats)
    }
    // ...
}
```

**Key insight**: The `stats()` method is the **projection** from private implementation to public observable state.

---

### 4. Update act() Method - ResourceRequested

**Current**:
```rust
Event::ResourceRequested(rid, cid) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }

    println!("[{}] Consumer {} requested Resource {}", current_t, cid, rid);
    self.stats.arrival_count += 1;  // Old field name

    if self.consumer_total == self.consumer_count {
        self.consumer_queue.push_back((*cid, current_t));
        self.consumers_active.insert(*cid);
        des::Response::new()
    } else {
        self.consumer_count += 1;
        self.stats.acquired_count += 1;  // Old field name
        des::Response::event(
            current_t,
            Event::ResourceAcquired(*rid, *cid, current_t)
        )
    }
}
```

**Updated**:
```rust
Event::ResourceRequested(rid, cid) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }

    println!("[{}] Consumer {} requested Resource {}", current_t, cid, rid);
    self.stats.total_arrivals += 1;  // New field name

    if self.consumer_total == self.consumer_count {
        self.consumer_queue.push_back((*cid, current_t));
        self.consumers_active.insert(*cid);
        des::Response::new()
    } else {
        self.consumer_count += 1;
        self.stats.total_acquired += 1;  // New field name
        des::Response::event(
            current_t,
            Event::ResourceAcquired(*rid, *cid, current_t)
        )
    }
}
```

---

### 5. Update act() Method - ResourceReleased

**Current**:
```rust
Event::ResourceReleased(rid, cid, acquired_t) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }
    println!("[{}] Consumer {} released Resource {}", current_t, cid, rid);
    self.stats.consume_sum += current_t - acquired_t;  // Old field name

    self.consumer_count -= 1;

    while let Some((consumer_id, requested_t)) = self.consumer_queue.pop_front() {
        if self.consumers_active.contains(&consumer_id) {
            self.consumers_active.remove(&consumer_id);
            self.consumer_count += 1;
            self.stats.wait_sum += current_t - requested_t;  // Old field name
            self.stats.acquired_count += 1;  // Old field name
            return des::Response::event(
                current_t,
                Event::ResourceAcquired(*rid, consumer_id, requested_t),
            );
        }
    }
    des::Response::new()
}
```

**Updated**:
```rust
Event::ResourceReleased(rid, cid, acquired_t) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }
    println!("[{}] Consumer {} released Resource {}", current_t, cid, rid);

    // Track consume time and release
    self.stats.total_consume_time += current_t - acquired_t;  // New field name
    self.stats.total_released += 1;  // New counter

    self.consumer_count -= 1;

    while let Some((consumer_id, requested_t)) = self.consumer_queue.pop_front() {
        if self.consumers_active.contains(&consumer_id) {
            self.consumers_active.remove(&consumer_id);
            self.consumer_count += 1;
            self.stats.total_wait_time += current_t - requested_t;  // New field name
            self.stats.total_acquired += 1;  // New field name
            return des::Response::event(
                current_t,
                Event::ResourceAcquired(*rid, consumer_id, requested_t),
            );
        }
    }
    des::Response::new()
}
```

---

### 6. Update act() Method - ResourceRequestExpired

**Current**:
```rust
Event::ResourceRequestExpired(rid, cid, requested_t) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }

    let removed = self.consumers_active.remove(cid);

    if removed {
        println!("[{}] Consumer {} request for Resource {} expired",
                 current_t, cid, rid);
        self.stats.expiry_count += 1;  // Old field name
        self.stats.wait_sum += current_t - requested_t;  // Old field name
    }
    des::Response::new()
}
```

**Updated**:
```rust
Event::ResourceRequestExpired(rid, cid, requested_t) => {
    if rid != &self.resource_id {
        return des::Response::new();
    }

    let removed = self.consumers_active.remove(cid);

    if removed {
        println!("[{}] Consumer {} request for Resource {} expired",
                 current_t, cid, rid);
        self.stats.total_expired += 1;  // New field name
        self.stats.total_wait_time += current_t - requested_t;  // New field name
    }
    des::Response::new()
}
```

---

### 7. Update ConsumerStats (Optional Enhancement)

**Current**:
```rust
#[derive(Debug, Clone)]
pub struct ConsumerStats {}

impl Default for ConsumerStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsumerStats {
    pub fn new() -> Self {
        ConsumerStats {}
    }
}
```

**Enhanced**:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerStats {
    pub resource_id: usize,
    pub consumers_generated: usize,
    pub next_consumer_id: usize,

    // Distribution parameters (for documentation)
    pub arrival_interval_mean: f64,
    pub consume_duration_mean: f64,
    pub consume_duration_std: f64,
    pub wait_duration_mean: f64,
    pub wait_duration_std: f64,
}

impl ConsumerStats {
    pub fn new(
        resource_id: usize,
        arrival_interval: f64,
        consume_duration: (f64, f64),
        wait_duration: (f64, f64),
    ) -> Self {
        ConsumerStats {
            resource_id,
            consumers_generated: 0,
            next_consumer_id: 0,
            arrival_interval_mean: 1.0 / arrival_interval,
            consume_duration_mean: consume_duration.0,
            consume_duration_std: consume_duration.1,
            wait_duration_mean: wait_duration.0,
            wait_duration_std: wait_duration.1,
        }
    }

    pub fn generation_rate(&self) -> f64 {
        1.0 / self.arrival_interval_mean
    }
}
```

Update ConsumerProcess constructor:
```rust
impl ConsumerProcess {
    pub fn new(
        resource_id: usize,
        arrival_interval: f64,
        consume_duration: (f64, f64),
        wait_duration: (f64, f64),
    ) -> ConsumerProcess {
        ConsumerProcess {
            resource_id,
            next_consumer_id: 0,
            arrival_interval: Geometric::new(arrival_interval).unwrap(),
            consume_duration: Normal::new(consume_duration.0, consume_duration.1).unwrap(),
            wait_duration: Normal::new(wait_duration.0, wait_duration.1).unwrap(),
            stats: ConsumerStats::new(
                resource_id,
                arrival_interval,
                consume_duration,
                wait_duration,
            ),
        }
    }
}
```

Update ConsumerProcess::act() to track consumers_generated:
```rust
fn new_consumer(&mut self, current_t: usize) -> ((usize, Event), (usize, Event)) {
    let consumer_id = self.next_consumer_id;
    self.next_consumer_id += 1;
    self.stats.consumers_generated += 1;  // Track generation
    // ... rest of method
}
```

---

## Complete Example: Updated Resource Agent

Here's a complete, minimal working version showing all changes:

```rust
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceStats {
    pub resource_id: usize,
    pub capacity: usize,
    pub current_consumer_count: usize,
    pub current_queue_length: usize,
    pub current_active_requests: usize,
    pub total_arrivals: usize,
    pub total_acquired: usize,
    pub total_expired: usize,
    pub total_released: usize,
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

pub struct Resource {
    resource_id: usize,           // Private
    consumer_total: usize,        // Private
    consumer_count: usize,        // Private
    consumer_queue: VecDeque<(usize, usize)>,  // Private
    consumers_active: HashSet<usize>,          // Private
    stats: ResourceStats,         // Private (but exposed via stats() method)
}

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
        let mut stats = self.stats.clone();
        stats.current_consumer_count = self.consumer_count;
        stats.current_queue_length = self.consumer_queue.len();
        stats.current_active_requests = self.consumers_active.len();
        Stats::ResourceStats(stats)
    }

    fn act(&mut self, current_t: usize, data: &Event) -> des::Response<Event, Stats> {
        match data {
            Event::ResourceRequested(rid, cid) => {
                if rid != &self.resource_id { return des::Response::new(); }

                self.stats.total_arrivals += 1;

                if self.consumer_total == self.consumer_count {
                    self.consumer_queue.push_back((*cid, current_t));
                    self.consumers_active.insert(*cid);
                    des::Response::new()
                } else {
                    self.consumer_count += 1;
                    self.stats.total_acquired += 1;
                    des::Response::event(current_t, Event::ResourceAcquired(*rid, *cid, current_t))
                }
            }

            Event::ResourceReleased(rid, cid, acquired_t) => {
                if rid != &self.resource_id { return des::Response::new(); }

                self.stats.total_consume_time += current_t - acquired_t;
                self.stats.total_released += 1;
                self.consumer_count -= 1;

                while let Some((consumer_id, requested_t)) = self.consumer_queue.pop_front() {
                    if self.consumers_active.contains(&consumer_id) {
                        self.consumers_active.remove(&consumer_id);
                        self.consumer_count += 1;
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
                if rid != &self.resource_id { return des::Response::new(); }

                if self.consumers_active.remove(cid) {
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

---

## Testing the Implementation

After making these changes:

```bash
# Run the stats-based tests
cargo test -p simple_queue --test stats_based_tests

# Should see all tests pass
# Example output:
# test given_empty_resource_when_consumer_requests_then_immediately_acquired ... ok
# test given_full_resource_when_consumer_requests_then_queued_not_acquired ... ok
# test resource_state_transitions_from_empty_to_queuing ... ok
# ...
```

---

## Summary of Changes

1. **ResourceStats**: Expanded with current state + semantic methods
2. **Resource constructor**: Pass id and capacity to Stats::new()
3. **stats() method**: Populate current state from private fields
4. **act() methods**: Update field names (arrival_count → total_arrivals)
5. **ConsumerStats** (optional): Enhanced with configuration info

**Key principle**: Private fields → computed → public Stats

**Result**: Complete testability without breaking encapsulation! ✅
