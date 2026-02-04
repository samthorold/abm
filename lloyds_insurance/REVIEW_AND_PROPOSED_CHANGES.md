# Lloyd's Insurance Implementation Review & Proposed Changes

**Review Date**: 2026-02-04
**Reviewer**: Critical Analysis of Implementation vs. Olmez et al. (2024)
**Status**: Phase 1 Complete, Core Validation Issues Identified

---

## Executive Summary

The implementation demonstrates solid software engineering but **fails to validate the paper's core scientific claims** due to:

1. **Critical Bug**: Actuarial pricing produces premiums 3-5x too high, creating unrealistic profits
2. **Missing Core Feature**: No catastrophe modeling → cannot demonstrate underwriting cycles
3. **Inactive Feature**: Lead-follow infrastructure exists but isn't wired up
4. **No Time-Series Output**: Cannot compare against paper's temporal dynamics

**Current Capability**: Demonstrates DES architecture ✅
**Scientific Replication**: Cannot validate any of the 4 scenarios ❌

---

## Critical Issues (Must Fix)

### 🔴 ISSUE #1: Actuarial Pricing Bug (CRITICAL)

**Observed**: Syndicates charge ~$825k per policy but expected loss is only $300k, creating loss ratios of 0.17 instead of near 1.0.

**Location**: `src/syndicate.rs:29-56`

**Root Cause Analysis**:
```rust
// Current pricing
let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;
// = $3,000,000 * 0.1 = $300,000 ✅ Correct

let base_price = z * syndicate_avg + (1.0 - z) * industry_avg_loss;
// With no history and z=0.5: 0.5 * $300k + 0.5 * $300k = $300k ✅ Correct

// BUT: Syndicates are actually charging $825k per policy
// Evidence: $3.7B premiums / 4,539 policies = $825k
```

**Hypothesis**: The pricing is being called multiple times or premiums are being double-counted somewhere in the policy acceptance flow.

**Proposed Fix**:

1. Add debug logging to track pricing:
```rust
// In syndicate.rs:58-73
fn handle_lead_quote_request(&mut self, risk_id: usize, current_t: usize) -> Vec<(usize, Event)> {
    let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;
    let price = self.calculate_actuarial_price(risk_id, industry_avg_loss);

    #[cfg(debug_assertions)]
    eprintln!("Syndicate {}: Quoting ${:.0} for risk {} (industry avg: ${:.0})",
              self.syndicate_id, price, risk_id, industry_avg_loss);

    let line_size = self.config.default_lead_line_size;
    vec![(current_t, Event::LeadQuoteOffered { risk_id, syndicate_id: self.syndicate_id, price, line_size })]
}
```

2. Verify premium collection isn't duplicated:
```rust
// In syndicate.rs:75-96 - Check if handle_lead_accepted is called multiple times
fn handle_lead_accepted(&mut self, risk_id: usize) {
    // CONCERN: This recalculates price instead of using the quoted price!
    // Should receive price from LeadQuoteAccepted event instead
    let industry_avg_loss = self.config.gamma_mean * self.config.yearly_claim_frequency;
    let price = self.calculate_actuarial_price(risk_id, industry_avg_loss);
    // ^ This might produce different results if loss_history changed

    self.capital += price;  // Is this the right price?
    // ...
}
```

**Recommended Solution**:

Change Event::LeadQuoteAccepted to include the accepted price:
```rust
// In lib.rs:84-87
LeadQuoteAccepted {
    risk_id: usize,
    syndicate_id: usize,
    price: f64,  // ADD THIS - pass through the quoted price
},
```

Update CentralRiskRepository to pass the price:
```rust
// In central_risk_repository.rs:68-74
events.push((
    current_t,
    Event::LeadQuoteAccepted {
        risk_id,
        syndicate_id: best_quote.syndicate_id,
        price: best_quote.price,  // ADD THIS
    },
));
```

Update Syndicate to use the passed price:
```rust
// In syndicate.rs:75-96
fn handle_lead_accepted(&mut self, risk_id: usize, price: f64) {
    // Use the price that was actually quoted and accepted
    self.capital += price;
    self.premium_history.push(price);

    let line_size = self.config.default_lead_line_size;
    let participation = PolicyParticipation {
        risk_id,
        line_size,
        premium_collected: price,
        is_lead: true,
    };
    self.policies.push(participation);

    self.stats.num_policies += 1;
    self.stats.total_premium_written += price;
    self.stats.total_premiums_collected += price;
    self.stats.total_line_size += line_size;
}
```

**Validation**: After fix, loss ratios should be near 1.0 (slightly below due to risk loading), and some syndicates should experience insolvency over 50 years.

---

### 🔴 ISSUE #2: No Catastrophe Modeling (Blocks Paper Validation)

**Impact**: Cannot demonstrate the paper's **primary contribution** - that catastrophe events drive underwriting cycles.

**Required**: Implement `CatastropheLossGenerator` agent.

**Proposed Implementation**:

Create `src/catastrophe_loss_generator.rs`:

```rust
use des::{Agent, Response};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Poisson};
use crate::{Event, Stats, CatastropheLossGeneratorStats, ModelConfig};

pub struct CatastropheLossGenerator {
    config: ModelConfig,
    stats: CatastropheLossGeneratorStats,
}

impl CatastropheLossGenerator {
    pub fn new(config: ModelConfig, sim_years: usize, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut stats = CatastropheLossGeneratorStats::new();

        // Pre-generate catastrophe events
        let lambda = config.mean_cat_events_per_year * sim_years as f64;
        let poisson = Poisson::new(lambda).unwrap();
        let num_catastrophes = poisson.sample(&mut rng) as usize;

        // Store events to be scheduled (would need to add to struct)
        // For each catastrophe:
        //   - Random time in [0, sim_years * 365]
        //   - Random peril region [0, num_peril_regions)
        //   - Truncated Pareto loss amount

        Self { config, stats }
    }
}

impl Agent<Event, Stats> for CatastropheLossGenerator {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        // On Day events, check if any pre-generated catastrophes should fire
        // Generate CatastropheLossOccurred events
        Response::new()
    }

    fn stats(&self) -> Stats {
        Stats::CatastropheLossGeneratorStats(self.stats.clone())
    }
}
```

**Note**: Need to implement Truncated Pareto distribution:
```rust
// Add to dependencies in Cargo.toml if not available in rand_distr
// Or implement manually based on paper's parameters
```

**Integration**:
```rust
// In main.rs:46
agents.push(Box::new(CatastropheLossGenerator::new(
    config.clone(),
    50, // 50 years
    77777
)));
```

**Validation**:
- With `mean_cat_events_per_year = 0.05`, expect ~2.5 catastrophes over 50 years
- Should see premium spikes in time-series output after catastrophes
- Loss ratios should spike above 1.0 during catastrophe years

---

### 🟡 ISSUE #3: Lead-Follow Mechanics Not Activated

**Problem**: Infrastructure exists but brokers never generate `FollowQuoteRequested` events.

**Evidence**: Output shows "Total follow quotes: 0"

**Location**: `src/broker.rs` doesn't implement the two-stage quote process.

**Proposed Fix**:

Update Broker to generate follow quote requests:

```rust
// In broker.rs:49-80
fn generate_risk(&mut self, current_t: usize) -> Vec<(usize, Event)> {
    let poisson = Poisson::new(self.config.risks_per_day).unwrap();

    if poisson.sample(&mut self.rng) > 0.0 {
        let risk_id = self.next_risk_id;
        self.next_risk_id += 1;

        // Random peril region
        let peril_region = (self.rng.next_u64() as usize) % self.config.num_peril_regions;

        self.stats.risks_generated += 1;

        // Schedule quote deadlines
        let lead_consolidation = current_t + 1;  // 1 day to quote
        let lead_selection = current_t + 2;      // 1 day to consolidate
        let follow_consolidation = current_t + 3; // ADD THIS
        let follow_selection = current_t + 4;     // ADD THIS

        vec![
            (current_t, Event::RiskBroadcasted {
                risk_id, peril_region, limit: self.config.risk_limit, broker_id: self.broker_id,
            }),
            (lead_consolidation, Event::LeadQuoteConsolidationDeadline { risk_id }),
            (lead_selection, Event::LeadQuoteSelectionDeadline { risk_id }),
            (follow_consolidation, Event::FollowQuoteConsolidationDeadline { risk_id }), // ADD
            (follow_selection, Event::FollowQuoteSelectionDeadline { risk_id }),         // ADD
        ]
    } else {
        Vec::new()
    }
}
```

Update BrokerSyndicateNetwork to request follow quotes:

```rust
// In broker_syndicate_network.rs
impl Agent<Event, Stats> for BrokerSyndicateNetwork {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::RiskBroadcasted { risk_id, .. } => {
                // Existing lead logic...
            }
            // ADD THIS:
            Event::LeadQuoteSelectionDeadline { risk_id } => {
                // After lead is selected, request follow quotes
                let selected_syndicates = self.select_syndicates(self.config.follow_top_k);
                let mut events = Vec::new();

                for syndicate_id in selected_syndicates {
                    events.push((
                        current_t,
                        Event::FollowQuoteRequested {
                            risk_id: *risk_id,
                            syndicate_id,
                            lead_price: 0.0, // Will need to get from repository
                        },
                    ));
                }

                Response::events(events)
            }
            _ => Response::new(),
        }
    }
}
```

**Challenge**: BrokerSyndicateNetwork needs to know the lead price. Two solutions:

1. **Option A**: CentralRiskRepository broadcasts lead price after selection:
```rust
Event::LeadPriceAnnounced { risk_id, price }
```

2. **Option B**: Followers just use their own pricing (simpler for now)

**Validation**: After fix, should see:
- "Total follow quotes" > 0 in output
- Multiple syndicates per policy
- More uniform loss experience across syndicates (Scenario 4 validation)

---

### 🟡 ISSUE #4: No Time-Series Output

**Problem**: Cannot observe temporal dynamics (premiums over time, capital over time) to compare against paper's Figure 6.

**Proposed Solution**:

Add a StatsCollector agent that records yearly snapshots:

```rust
// Create src/stats_collector.rs
use des::{Agent, Response};
use std::collections::HashMap;
use crate::{Event, Stats};

pub struct StatsCollector {
    yearly_snapshots: Vec<MarketSnapshot>,
    current_year: usize,
    syndicate_data: HashMap<usize, SyndicateYearlyData>,
}

#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub year: usize,
    pub avg_premium: f64,
    pub avg_loss_ratio: f64,
    pub num_insolvencies: usize,
    pub total_capital: f64,
}

impl Agent<Event, Stats> for StatsCollector {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Year => {
                // Collect stats from all agents and create snapshot
                self.current_year += 1;
                // Would need to subscribe to syndicate capital reports
            }
            Event::SyndicateCapitalReported { syndicate_id, capital } => {
                // Track capital changes
            }
            _ => {}
        }
        Response::new()
    }

    fn stats(&self) -> Stats {
        // Return yearly snapshots
        Stats::TimeSeriesStats(self.yearly_snapshots.clone())
    }
}
```

Add CSV export in main.rs:

```rust
// After simulation completes
if let Some(Stats::TimeSeriesStats(snapshots)) = stats.iter()
    .find(|s| matches!(s, Stats::TimeSeriesStats(_))) {

    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("lloyds_insurance/output/time_series.csv").unwrap();
    writeln!(file, "year,avg_premium,avg_loss_ratio,num_insolvencies,total_capital").unwrap();

    for snapshot in snapshots {
        writeln!(file, "{},{},{},{},{}",
                 snapshot.year,
                 snapshot.avg_premium,
                 snapshot.avg_loss_ratio,
                 snapshot.num_insolvencies,
                 snapshot.total_capital).unwrap();
    }
}
```

**Validation**: Can plot premium over time and compare against paper's Figure 6 cyclicality patterns.

---

### 🟢 ISSUE #5: Broker Stats Bug (Minor)

**Problem**: Output shows "Total risks bound: 0" despite 27,435 policies created.

**Location**: `src/broker.rs` - missing increment when risk is bound.

**Proposed Fix**:

Brokers need to listen for policy acceptance events:

```rust
// In broker.rs, update act() method
impl Agent<Event, Stats> for Broker {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Day => {
                Response::events(self.generate_risk(current_t))
            }
            // ADD THIS:
            Event::LeadQuoteAccepted { risk_id, .. } => {
                // Check if this was our risk
                if self.is_our_risk(*risk_id) {
                    self.stats.risks_bound += 1;
                }
                Response::new()
            }
            _ => Response::new(),
        }
    }
}
```

**Problem**: Broker doesn't track which risks it generated. Need to add:

```rust
pub struct Broker {
    // ... existing fields
    our_risks: HashSet<usize>, // ADD THIS
}

fn generate_risk(&mut self, current_t: usize) -> Vec<(usize, Event)> {
    // ...
    self.our_risks.insert(risk_id); // ADD THIS
    // ...
}

fn is_our_risk(&self, risk_id: usize) -> bool {
    self.our_risks.contains(&risk_id)
}
```

---

## Medium Priority Issues

### 🟡 ISSUE #6: No Industry Statistics Distribution

**Problem**: Syndicates use hardcoded industry averages instead of dynamic market statistics.

**Proposed Solution**:

Create IndustryStatsAggregator agent:

```rust
// src/industry_stats_aggregator.rs
pub struct IndustryStatsAggregator {
    syndicate_losses: HashMap<usize, Vec<f64>>,
    syndicate_premiums: HashMap<usize, Vec<f64>>,
}

impl Agent<Event, Stats> for IndustryStatsAggregator {
    fn act(&mut self, _current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Month => {
                // Calculate industry-wide statistics
                let avg_claim_frequency = /* compute from all syndicates */;
                let avg_claim_cost = /* compute from all syndicates */;

                Response::event(Event::IndustryLossStatsReported {
                    avg_claim_frequency,
                    avg_claim_cost,
                })
            }
            Event::ClaimReceived { syndicate_id, amount, .. } => {
                // Record for aggregation
                Response::new()
            }
            _ => Response::new(),
        }
    }
}
```

Update Syndicate to use distributed stats:

```rust
// In syndicate.rs
pub struct Syndicate {
    // ...
    industry_avg_loss: f64, // Cache latest industry stats
}

impl Agent<Event, Stats> for Syndicate {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::IndustryLossStatsReported { avg_claim_frequency, avg_claim_cost } => {
                self.industry_avg_loss = avg_claim_frequency * avg_claim_cost;
                Response::new()
            }
            // ... rest of implementation
        }
    }
}
```

---

### 🟡 ISSUE #7: Exposure by Peril Region Not Tracked

**Problem**: `SyndicateStats.exposure_by_peril_region` exists but is never populated.

**Location**: `src/syndicate.rs:121-123` has a TODO comment.

**Proposed Solution**:

```rust
// In syndicate.rs, update PolicyParticipation to include peril_region
pub struct PolicyParticipation {
    pub risk_id: usize,
    pub peril_region: usize,  // ADD THIS
    pub line_size: f64,
    pub premium_collected: f64,
    pub is_lead: bool,
}

// Update handle_lead_accepted to receive peril_region
fn handle_lead_accepted(&mut self, risk_id: usize, peril_region: usize, price: f64) {
    // ...
    let participation = PolicyParticipation {
        risk_id,
        peril_region,  // ADD THIS
        line_size,
        premium_collected: price,
        is_lead: true,
    };
    self.policies.push(participation);
}

// Update update_stats to compute exposure
fn update_stats(&mut self) {
    self.stats.capital = self.capital;
    self.stats.update_loss_ratio();
    self.stats.update_profit();

    // Compute exposure by peril region
    self.stats.exposure_by_peril_region.clear();
    for policy in &self.policies {
        *self.stats.exposure_by_peril_region
            .entry(policy.peril_region)
            .or_insert(0.0) += policy.premium_collected * policy.line_size;
    }

    // Compute uniform deviation (Scenario 3 metric)
    if !self.stats.exposure_by_peril_region.is_empty() {
        let total_exposure: f64 = self.stats.exposure_by_peril_region.values().sum();
        let num_regions = self.stats.exposure_by_peril_region.len();
        let target_exposure = total_exposure / num_regions as f64;

        let variance: f64 = self.stats.exposure_by_peril_region.values()
            .map(|&exp| (exp - target_exposure).powi(2))
            .sum::<f64>() / num_regions as f64;

        self.stats.uniform_deviation = variance.sqrt();
    }
}
```

**Validation**: With VaR EM (Scenario 3), uniform_deviation should approach 0.

---

## Low Priority Enhancements

### 🟢 Underwriting Markup (Scenario 1 Enhancement)

After fixing pricing bug and implementing catastrophes:

```rust
// Add to Syndicate
pub struct Syndicate {
    // ...
    underwriter_markup: f64,  // m_t in paper's formula
}

fn apply_underwriting_markup(&mut self, actuarial_price: f64) -> f64 {
    // P_t = P_at · e^(m_t)
    actuarial_price * self.underwriter_markup.exp()
}

fn update_markup(&mut self, market_conditions: /* TBD */) {
    // Exponentially weighted moving average based on competitive pressure
    // m_t = (1-β)·m_{t-1} + β·market_signal
    // where β = underwriter_recency_weight
}
```

---

### 🟢 Premium and VaR Exposure Management

Implement as separate sub-agents or modules within Syndicate:

```rust
// src/syndicate/premium_exposure_manager.rs
pub struct PremiumExposureManager {
    premium_to_capital_ratio: f64,
    max_ratio: f64,
}

impl PremiumExposureManager {
    pub fn evaluate_quote(&self, premium: f64, capital: f64) -> ExposureDecision {
        let current_ratio = premium / capital;
        if current_ratio > self.max_ratio {
            ExposureDecision::Reject
        } else if current_ratio > self.max_ratio * 0.8 {
            ExposureDecision::ScalePremium(0.5) // Reduce exposure
        } else {
            ExposureDecision::Accept
        }
    }
}
```

```rust
// src/syndicate/var_exposure_manager.rs
pub struct VaRExposureManager {
    peril_region_exposures: HashMap<usize, f64>,
    num_simulations: usize,
    exceedance_prob: f64,
}

impl VaRExposureManager {
    pub fn run_monte_carlo(&self, peril_region: usize) -> f64 {
        // Run simulations to estimate VaR
        // Compare against capital
        // Return rejection/scaling decision
    }
}
```

---

## Testing Recommendations

### Add Integration Tests

```rust
// tests/scenario_tests.rs

#[test]
fn scenario_1_base_case_produces_realistic_loss_ratios() {
    let config = ModelConfig::scenario_1();
    let (event_loop, stats) = run_simulation(config, 50);

    let syndicate_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s { Stats::SyndicateStats(ss) => Some(ss), _ => None })
        .collect();

    for s in syndicate_stats {
        // Loss ratio should be close to 1.0 (slightly below due to profit)
        assert!(s.loss_ratio > 0.8 && s.loss_ratio < 1.2,
                "Syndicate {} loss ratio {} is unrealistic", s.syndicate_id, s.loss_ratio);
    }
}

#[test]
fn scenario_2_catastrophes_cause_premium_spikes() {
    let config = ModelConfig::scenario_2();
    let (event_loop, stats) = run_simulation(config, 50);

    let time_series = extract_time_series(&stats);

    // Find catastrophe years
    let cat_years = find_catastrophe_years(&stats);

    for cat_year in cat_years {
        let premium_before = time_series.avg_premium[cat_year - 1];
        let premium_after = time_series.avg_premium[cat_year + 1];

        assert!(premium_after > premium_before * 1.2,
                "Expected premium spike after catastrophe in year {}", cat_year);
    }
}

#[test]
fn scenario_4_lead_follow_reduces_volatility() {
    let config_without_followers = ModelConfig::scenario_1();
    let config_with_followers = ModelConfig::scenario_4();

    let (_, stats_without) = run_simulation(config_without_followers, 50);
    let (_, stats_with) = run_simulation(config_with_followers, 50);

    let volatility_without = compute_loss_ratio_std_dev(&stats_without);
    let volatility_with = compute_loss_ratio_std_dev(&stats_with);

    assert!(volatility_with < volatility_without,
            "Lead-follow should reduce volatility (paper's Scenario 4)");
}
```

---

## Implementation Roadmap

### Phase 1.5: Fix Critical Bugs (1-2 days)
- [ ] Fix actuarial pricing bug (#1)
- [ ] Fix broker stats tracking (#5)
- [ ] Validate loss ratios are realistic
- [ ] Run regression tests

### Phase 2: Core Features (3-5 days)
- [ ] Implement CatastropheLossGenerator (#2)
- [ ] Wire up lead-follow mechanics (#3)
- [ ] Add time-series output (#4)
- [ ] Validate Scenario 1 and 2 against paper

### Phase 3: Advanced Features (3-5 days)
- [ ] Implement IndustryStatsAggregator (#6)
- [ ] Track exposure by peril region (#7)
- [ ] Add underwriting markup
- [ ] Validate Scenario 3 and 4 against paper

### Phase 4: Production Polish (2-3 days)
- [ ] Add Premium EM and VaR EM
- [ ] Integration test suite
- [ ] CSV/visualization output
- [ ] Performance optimization
- [ ] Documentation updates

**Total Estimated Effort**: 9-15 days to full paper replication

---

## Validation Checklist

After implementing changes, verify against paper:

### Scenario 1: Base Case
- [ ] Premiums converge to ~$300k (0.1 × $3M)
- [ ] Loss ratios fluctuate around 1.0
- [ ] Some syndicates experience insolvency
- [ ] Capital shows cyclical behavior

### Scenario 2: Catastrophe Events
- [ ] Pronounced cyclicality in premiums (Figure 6)
- [ ] Premium spikes following catastrophe events
- [ ] Loss ratios spike above 1.0 during catastrophes
- [ ] More insolvencies than Scenario 1

### Scenario 3: VaR Exposure Management
- [ ] Uniform deviation approaches 0
- [ ] Fewer insolvencies than Scenario 2
- [ ] Exposure distributed evenly across peril regions

### Scenario 4: Lead-Follow Dynamics
- [ ] Tightly coupled premium convergence
- [ ] Highly correlated loss ratios
- [ ] Zero insolvencies (vs insolvencies in Scenario 1)
- [ ] Lower volatility than lead-only

---

## Conclusion

The current implementation is a **solid foundation** but requires **critical bug fixes and feature additions** before it can claim to replicate the paper's findings. The architecture is sound—the missing pieces are well-defined and straightforward to implement.

**Priority**: Fix the pricing bug first. Everything else depends on having realistic economics.

**Timeline**: With focused effort, could achieve full paper replication in 2-3 weeks.

**Risk**: The pricing bug might reveal deeper issues with the loss distribution or event timing that require more extensive refactoring.
