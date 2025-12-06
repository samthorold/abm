# Testing Implementation Guide

## Making simple_queue Testable

The example tests in `simple_queue/tests/` demonstrate the desired testing approach but won't compile with the current codebase. This guide outlines the specific changes needed to make the tests work.

---

## Required Changes to simple_queue/src/lib.rs

### 1. Make Resource Fields Public

**Current (private fields):**
```rust
pub struct Resource {
    resource_id: usize,
    consumer_total: usize,
    consumer_count: usize,
    consumer_queue: VecDeque<(usize, usize)>,
    consumers_active: HashSet<usize>,
    stats: ResourceStats,
}
```

**Required (public fields):**
```rust
pub struct Resource {
    pub resource_id: usize,
    pub consumer_total: usize,
    pub consumer_count: usize,
    pub consumer_queue: VecDeque<(usize, usize)>,
    pub consumers_active: HashSet<usize>,
    pub stats: ResourceStats,
}
```

**Rationale**: Tests need to inspect agent state to verify behavior. For a research codebase, public fields are acceptable.

---

### 2. Make ResourceStats Fields Public

**Current:**
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

**Required:**
```rust
#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub arrival_count: usize,
    pub acquired_count: usize,
    pub expiry_count: usize,
    pub consume_sum: usize,
    pub wait_sum: usize,
}
```

---

### 3. Add Deterministic Constructor to ConsumerProcess

**Current:** No way to seed the RNG for deterministic testing.

**Add this method:**
```rust
use rand::SeedableRng;
use rand_distr::{Distribution, Geometric, Normal};

impl ConsumerProcess {
    // Keep existing constructor
    pub fn new(
        resource_id: usize,
        arrival_interval: f64,
        consume_duration: (f64, f64),
        wait_duration: (f64, f64),
    ) -> ConsumerProcess {
        // ... existing implementation
    }

    // Add new constructor for testing with seeded RNG
    #[cfg(test)]
    pub fn new_with_seed(
        resource_id: usize,
        seed: u64,
        arrival_interval: f64,
        consume_duration: (f64, f64),
        wait_duration: (f64, f64),
    ) -> ConsumerProcess {
        ConsumerProcess {
            resource_id,
            next_consumer_id: 0,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            arrival_interval: Geometric::new(arrival_interval).unwrap(),
            consume_duration: Normal::new(consume_duration.0, consume_duration.1).unwrap(),
            wait_duration: Normal::new(wait_duration.0, wait_duration.1).unwrap(),
            stats: ConsumerStats::new(),
        }
    }
}
```

**Required change to ConsumerProcess struct:**
```rust
use rand::rngs::StdRng;

pub struct ConsumerProcess {
    resource_id: usize,
    next_consumer_id: usize,
    rng: StdRng,  // Changed from using rand::rng() directly
    arrival_interval: Geometric,
    consume_duration: Normal<f64>,
    wait_duration: Normal<f64>,
    stats: ConsumerStats,
}
```

**Update draw methods to use self.rng:**
```rust
impl ConsumerProcess {
    fn draw_arrival_interval(&mut self) -> usize {
        self.arrival_interval.sample(&mut self.rng) as usize  // Changed
    }

    fn draw_consume_duration(&mut self) -> usize {
        self.consume_duration
            .sample(&mut self.rng)  // Changed
            .floor()
            .max(0.0) as usize
    }

    fn draw_wait_duration(&mut self) -> usize {
        self.wait_duration.sample(&mut self.rng).floor().max(0.0) as usize  // Changed
    }
}
```

**Update Cargo.toml dependencies:**
```toml
[dependencies]
des = { path = "../des" }
rand = "0.9"
rand_distr = "0.5"
```

---

### 4. Make Response Fields Accessible

The `des::Response` already has public fields, but tests need to access them. No change needed.

---

## Step-by-Step Implementation

### Phase 1: Enable Basic Testing (15 minutes)

1. Make Resource fields public:
   - Change all fields in `Resource` struct to `pub`
   - Change all fields in `ResourceStats` struct to `pub`

2. Run the tests to see which ones pass:
   ```bash
   cargo test -p simple_queue
   ```

   Expected: Most resource_behavior_tests will compile and pass. Integration tests will fail because ConsumerProcess isn't deterministic yet.

### Phase 2: Add Determinism (30 minutes)

1. Update `ConsumerProcess` struct to use `StdRng` instead of `rand::rng()`

2. Add `new_with_seed` constructor (cfg(test) gated)

3. Update `draw_*` methods to use `self.rng`

4. Update main.rs constructor to use default RNG:
   ```rust
   ConsumerProcess::new(
       counter_id,
       1.0 / 100.0,
       (120.0, 20.0),
       (20.0, 2.0),
   )
   ```

5. Run tests again:
   ```bash
   cargo test -p simple_queue
   ```

   Expected: All tests should now compile and pass.

### Phase 3: Document and Extend (ongoing)

1. Add more test cases as needed
2. Document testing approach in simple_queue/README.md (if exists)
3. Apply same pattern to other simulation modules

---

## Minimal Working Example

Here's the absolute minimum change to make one test work:

**File: simple_queue/src/lib.rs**

```rust
// At the top of Resource struct definition
pub struct Resource {
    pub resource_id: usize,        // Add 'pub'
    pub consumer_total: usize,     // Add 'pub'
    pub consumer_count: usize,     // Add 'pub'
    pub consumer_queue: VecDeque<(usize, usize)>,  // Add 'pub'
    pub consumers_active: HashSet<usize>,          // Add 'pub'
    pub stats: ResourceStats,      // Add 'pub'
}

// At the top of ResourceStats struct definition
pub struct ResourceStats {
    pub arrival_count: usize,      // Add 'pub'
    pub acquired_count: usize,     // Add 'pub'
    pub expiry_count: usize,       // Add 'pub'
    pub consume_sum: usize,        // Add 'pub'
    pub wait_sum: usize,           // Add 'pub'
}
```

**Then run:**
```bash
# This test should now work
cargo test -p simple_queue given_resource_has_capacity_when_consumer_requests_then_immediately_acquired
```

---

## Testing the Tests

After making changes, verify everything works:

```bash
# Run all simple_queue tests
cargo test -p simple_queue

# Run specific test file
cargo test -p simple_queue --test resource_behavior_tests

# Run specific test
cargo test -p simple_queue given_full_resource_when_consumer_requests

# Run with output to see println! statements
cargo test -p simple_queue -- --nocapture
```

---

## Alternative: Staged Approach

If you want to implement this gradually:

### Stage 1: Just Resource Tests
- Make only Resource and ResourceStats fields public
- Comment out integration_tests.rs (uses ConsumerProcess)
- Verify resource_behavior_tests work

### Stage 2: Add ConsumerProcess Determinism
- Implement RNG seeding for ConsumerProcess
- Uncomment integration_tests.rs
- Verify all tests work

### Stage 3: Apply to Other Modules
- Use same pattern for evolving_market agents
- Use same pattern for evolution_coop agents

---

## Common Issues and Solutions

### Issue: "field `consumer_count` of struct `Resource` is private"
**Solution**: Make the field public: `pub consumer_count: usize`

### Issue: Tests are flaky (pass sometimes, fail sometimes)
**Solution**: Ensure all agents use seeded RNGs for deterministic behavior

### Issue: "method `act` not found in scope"
**Solution**: Import the Agent trait: `use des::Agent;`

### Issue: Can't pattern match on Event variants
**Solution**: Make sure Event enum is imported: `use simple_queue::Event;`

### Issue: Tests pass locally but different results on different machines
**Solution**: This suggests RNG seeding isn't working. Double-check that agents are using StdRng with consistent seeds.

---

## Next Steps

Once simple_queue tests are working:

1. **Document the pattern**: Add testing examples to main README.md
2. **Create test template**: Make a template file for new simulations
3. **Apply to other modules**: Use same approach for evolving_market, evolution_coop
4. **Add CI**: Set up GitHub Actions to run tests automatically
5. **Coverage**: Consider using `cargo-tarpaulin` to measure test coverage

---

## Benefits You'll See

After implementing this:

✅ **Faster debugging**: Pinpoint exactly which agent behavior is wrong
✅ **Safer refactoring**: Know immediately if you break something
✅ **Better documentation**: Tests show how agents are supposed to behave
✅ **Research validation**: Verify your implementation matches the paper
✅ **Confidence**: Make changes without fear

---

## Questions?

If you encounter issues:
1. Check that all struct fields are public
2. Verify imports are correct
3. Ensure RNG seeding is consistent
4. Run with `--nocapture` to see debug output
5. Try running just one test first before running all tests
