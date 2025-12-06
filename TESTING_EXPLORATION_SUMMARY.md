# Testing Exploration Summary

## Overview

This exploration examined the current testing strategy and proposes improvements for testing agent behavior in the ABM simulation framework. The focus is on applying "given-when-then" (BDD-style) testing to verify agent state transitions and interactions.

---

## Current State

### What exists:
- **des crate**: 5 infrastructure tests (EventLoop mechanics, event ordering)
- **evolving_market**: 5 pure function tests (stochastic auction, loyalty calculations)
- **simple_queue**: No tests
- **evolution_coop**: Not examined

### Key finding:
**Zero agent behavior tests exist** - all tests verify infrastructure or pure functions, not agent decision-making or state transitions.

---

## Core Problem

Agents are the heart of ABM simulations, but we can't verify:
- How agents respond to specific events
- Whether agent state changes correctly
- If multi-agent interactions work as designed
- If implementations match research papers

This makes debugging difficult and refactoring risky.

---

## Proposed Solution: Three-Level Testing

### Level 1: Unit Tests (Individual Agent Responses)
```
GIVEN: Agent in specific state + simulation time
WHEN:  Event is broadcast to agent
THEN:  Verify response events, agent state changes, stats updates
```

**Example**: "Given resource has capacity, when consumer requests, then immediately acquired"

### Level 2: Integration Tests (Multi-Agent Interactions)
```
GIVEN: Multiple agents in specific states
WHEN:  Event sequence is processed
THEN:  Verify expected interaction protocol and outcomes
```

**Example**: "Given consumer process and resource, when consumer lifecycle completes, then stats show successful transaction"

### Level 3: Scenario Tests (Emergent Behavior)
```
GIVEN: Complete simulation setup
WHEN:  Simulation runs for extended period
THEN:  Verify emergent properties and research hypotheses
```

**Example**: "Queue reaches steady state under constant load with ρ < 1"

---

## Deliverables

Created three documents:

### 1. **TESTING_STRATEGY.md** (comprehensive, ~400 lines)
- Detailed analysis of current state
- Complete testing approach with 20+ code examples
- Architectural changes needed for testability
- Implementation roadmap
- Test suite structure recommendations

### 2. **TESTING_IMPLEMENTATION_GUIDE.md** (practical, ~200 lines)
- Step-by-step instructions to enable testing
- Required code changes with diffs
- Minimal working example
- Troubleshooting guide
- Alternative staged approaches

### 3. **Example Tests** (concrete, ~300 lines)
- `simple_queue/tests/resource_behavior_tests.rs`: 8 Level 1 tests
- `simple_queue/tests/integration_tests.rs`: 5 Level 2 tests
- Demonstrates given-when-then pattern
- Shows what good agent tests look like

---

## Key Architectural Changes Needed

### For Testability:

1. **Observable state**: Make agent fields public or add inspection methods
   ```rust
   pub struct Resource {
       pub consumer_count: usize,  // Was private
       pub consumer_queue: VecDeque<(usize, usize)>,
       // ...
   }
   ```

2. **Deterministic testing**: Add RNG seeding support
   ```rust
   pub fn new_with_seed(resource_id: usize, seed: u64, ...) -> Self {
       ConsumerProcess {
           rng: StdRng::seed_from_u64(seed),  // Deterministic
           // ...
       }
   }
   ```

3. **Testable stats**: Make stats fields public
   ```rust
   pub struct ResourceStats {
       pub arrival_count: usize,  // Was private
       // ...
   }
   ```

---

## Example: Before and After

### Before (no tests)
```rust
// Only way to verify behavior: run main.rs and inspect output
cargo run -p simple_queue
// [10] Consumer 0 requested Resource 0
// [10] Consumer 0 acquired Resource 0
// ... manual inspection required
```

### After (tested behavior)
```rust
#[test]
fn given_resource_has_capacity_when_consumer_requests_then_immediately_acquired() {
    let mut resource = Resource::new(0, 2);
    let response = resource.act(10, &Event::ResourceRequested(0, 42));

    assert_eq!(response.events.len(), 1);
    assert!(matches!(response.events[0].1, Event::ResourceAcquired(0, 42, 10)));
    assert_eq!(resource.consumer_count, 1);
}
```

**Benefits**:
- ✅ Runs in milliseconds
- ✅ Automated (no manual inspection)
- ✅ Pinpoints exact failure
- ✅ Documents expected behavior
- ✅ Prevents regressions

---

## Implementation Recommendations

### Quick Start (1-2 hours)
1. Make Resource and ResourceStats fields public in simple_queue
2. Run the example tests in `simple_queue/tests/resource_behavior_tests.rs`
3. Fix any compilation errors
4. Verify 8 tests pass

### Full Implementation (1 week)
1. Add RNG seeding to ConsumerProcess
2. Enable all integration tests
3. Create test utilities in des crate
4. Apply pattern to evolving_market
5. Apply pattern to evolution_coop
6. Document approach in main README

### Long Term (ongoing)
- Write scenario tests for research hypothesis validation
- Add property-based testing with proptest
- Set up CI to run tests automatically
- Track test coverage

---

## Why This Matters for Research

### Current workflow:
1. Implement agent from paper
2. Run full simulation
3. Inspect output manually
4. Hope it's correct 🤞

### Problems:
- Can't verify individual agent logic
- Hard to debug when results are wrong
- Risky to refactor or optimize
- No way to confirm paper implementation is accurate

### With behavioral tests:
1. Write test for specific agent behavior from paper
2. Implement agent to pass test
3. Verify with unit tests (fast feedback)
4. Validate with integration tests
5. Confirm with scenario tests
6. **Know** it's correct ✅

### Research benefits:
- **Validation**: Verify your code matches the paper's model
- **Exploration**: Test "what if" scenarios easily
- **Replication**: Others can verify your implementation
- **Extension**: Safely modify models knowing tests will catch errors
- **Publication**: Demonstrate rigorous implementation methodology

---

## Trade-offs

### Pros:
- Much faster debugging (minutes vs hours)
- Safe refactoring
- Documentation through tests
- Research credibility
- Easier collaboration

### Cons:
- Upfront time investment (~1-2 days for first module)
- Need to maintain tests when changing agents
- Requires thinking about testability during design

### Verdict:
**Worth it** for any simulation you plan to:
- Use for research publications
- Maintain/extend over time
- Share with collaborators
- Build upon for future work

**Maybe skip** for:
- Quick one-off experiments
- Throw-away prototypes
- Very simple toy examples

---

## Next Actions

Choose your path:

### Path A: Dive In (Recommended)
1. Read `TESTING_IMPLEMENTATION_GUIDE.md`
2. Make Resource fields public
3. Run `cargo test -p simple_queue`
4. Fix any issues
5. Celebrate when tests pass 🎉

### Path B: Understand First
1. Read `TESTING_STRATEGY.md` (comprehensive)
2. Review example tests in `simple_queue/tests/`
3. Decide which levels of testing you want
4. Then follow Path A

### Path C: Gradual Adoption
1. Just do Level 1 tests for Resource first
2. See if you like the approach
3. Expand to ConsumerProcess
4. Add integration tests
5. Apply to other modules

---

## Open Questions for Discussion

1. **Scope**: Should we apply this to all simulations or just new ones?

2. **Stats design**: Should `stats()` return current state (for mid-simulation testing) or keep cumulative only?

3. **Public fields**: Are we comfortable with public fields, or prefer accessor methods?

4. **Test organization**: One big test file per module or many small files?

5. **CI/CD**: Should we set up automated testing in GitHub Actions?

6. **Coverage goals**: What % test coverage should we aim for?

---

## Files Created

```
/home/user/abm/
├── TESTING_STRATEGY.md                    # Comprehensive strategy (read second)
├── TESTING_IMPLEMENTATION_GUIDE.md        # How-to guide (read first)
├── TESTING_EXPLORATION_SUMMARY.md         # This file (read for overview)
└── simple_queue/
    └── tests/
        ├── resource_behavior_tests.rs     # 8 Level 1 examples
        └── integration_tests.rs           # 5 Level 2 examples
```

**Estimated reading time**:
- This summary: 5 minutes
- Implementation guide: 15 minutes
- Full strategy: 30 minutes
- Example tests: 10 minutes (skim)

---

## Conclusion

The current testing approach tests infrastructure but not agent behavior. By applying given-when-then testing at three levels (unit, integration, scenario), we can:

1. Verify agents behave correctly in isolation
2. Validate multi-agent interactions
3. Confirm emergent properties match research hypotheses

The example tests demonstrate this is **practical and valuable** for ABM research codebases. Small architectural changes (public fields, RNG seeding) enable comprehensive behavioral testing.

**Recommendation**: Start with simple_queue as proof-of-concept, then expand to other modules.
