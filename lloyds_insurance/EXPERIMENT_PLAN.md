# Experiment Plan: Validating Economic Dynamics

**Goal**: Replace the overly-permissive loss ratio test (0.5-1.8) with targeted experiments that validate the simulation's economic behavior against the paper's theoretical expectations.

---

## Background: What We're Testing

The paper (Olmez et al. 2024) makes specific quantitative predictions:

1. **Fair price convergence**: Premiums should converge to ~$300k (full risk) or ~$150k (lead at 50% line size)
   - Theoretical: Expected loss = λ × μ = 0.1 claims/year × $3M/claim = $300k/year
   - With volatility loading and markup, should be close to this

2. **Loss ratios around 1.0**: Over long runs, claims/premiums ≈ 1.0
   - Short-term: can deviate significantly (variance from Gamma CoV=1.0)
   - Long-term: should converge toward 1.0 as premiums adjust via markup

3. **Underwriting cycles**: With markup enabled, loss ratios should oscillate
   - Post-catastrophe: spike above 1.0 → markup increases → premiums rise
   - Recovery period: premiums high → profits accumulate → markup decreases

4. **Insolvencies**: Some syndicates go insolvent in Scenario 1, more in Scenario 2
   - Capital drain from dividends + occasional large losses
   - With markup, should be fewer insolvencies (prices adjust to risk)

---

## Current Test Problems

```rust
test_market_loss_ratios_are_realistic()
```

Issues:
- Only 10 years (insufficient for convergence with high variance)
- Only 2 syndicates, 2 brokers (not the paper's 5 syndicates, 25 brokers)
- Accepts 0.5-1.8 loss ratio (3.6x range - too permissive)
- Uses old BrokerSyndicateNetwork (now removed)
- No validation of premium levels or convergence

---

## Proposed Experiments

### Experiment 1: Long-Run Loss Ratio Convergence (Scenario 1)

**Setup:**
- 5 syndicates, 25 brokers (via BrokerPool)
- Scenario 1 config (attritional only, no catastrophes)
- Run for 50 years (18,250 days)
- Seed: fixed for reproducibility

**Expected Outcomes:**
1. **Average loss ratio**: 0.8 to 1.2 (tighter than current 0.5-1.8)
   - Rationale: With markup, premiums adjust to losses over time
   - Some variance expected from stochastic claims (Gamma CoV=1.0)

2. **Per-syndicate loss ratios**: Most within 0.7 to 1.3
   - Individual variance higher than market average
   - A few outliers acceptable (one insolvent syndicate OK)

3. **Temporal pattern**: Loss ratios fluctuate but mean-reverting
   - No strong trend up or down over 50 years
   - Variance decreases over time as markup stabilizes

**Test Assertions:**
```rust
assert!(avg_loss_ratio >= 0.8 && avg_loss_ratio <= 1.2,
    "Average loss ratio should be 0.8-1.2 over 50 years");

let solvent_syndicates: Vec<_> = syndicate_stats.iter()
    .filter(|s| !s.is_insolvent).collect();
assert!(solvent_syndicates.len() >= 3,
    "At least 3/5 syndicates should remain solvent");

// For solvent syndicates only
let solvent_loss_ratios: Vec<_> = solvent_syndicates.iter()
    .map(|s| s.loss_ratio).collect();
assert!(solvent_loss_ratios.iter().all(|&lr| lr >= 0.6 && lr <= 1.4),
    "Solvent syndicates should have loss ratios 0.6-1.4");
```

---

### Experiment 2: Premium Convergence to Fair Price (Scenario 1)

**Setup:**
- Same as Experiment 1
- Extract time series data (we now have MarketStatisticsCollector!)
- Calculate average premium per risk over final 10 years

**Expected Outcomes:**
1. **Average premium (lead)**: $120k to $180k
   - Theoretical fair price: $150k (0.5 line size × $300k expected loss)
   - With volatility loading (default 0.0) and markup dynamics, some deviation OK
   - ±20% tolerance = $120k-$180k

2. **Premium convergence**: Standard deviation decreases over time
   - First 10 years: high variance (σ > $30k)
   - Last 10 years: lower variance (σ < $20k)

3. **Markup convergence**: m_t oscillates around 0
   - No persistent positive or negative bias
   - Indicates balanced market (not systematically over/underpricing)

**Test Assertions:**
```rust
// Need to collect premium data from time series
let final_10_years_premiums = extract_premiums_from_time_series(50..60);
let avg_premium = final_10_years_premiums.mean();

assert!(avg_premium >= 120_000.0 && avg_premium <= 180_000.0,
    "Average premium should converge to fair price (±20%)");

let early_variance = premiums_year_0_to_10.variance();
let late_variance = premiums_year_40_to_50.variance();
assert!(late_variance < early_variance * 0.7,
    "Premium variance should decrease as market matures");
```

**Note**: This requires extending MarketStatisticsCollector to track premiums. Currently it only tracks capital and solvency.

---

### Experiment 3: Catastrophe-Driven Cycles (Scenario 2)

**Setup:**
- 5 syndicates, 25 brokers
- Scenario 2 config (catastrophes enabled, λ=0.05/year)
- Run for 50 years
- Seed: fixed for reproducibility

**Expected Outcomes:**
1. **Post-catastrophe loss ratio spikes**: Detectable in time series
   - Years with catastrophes: loss ratio > 1.5 (claims exceed premiums)
   - Following 1-2 years: premiums increase (markup effect)
   - 3-5 years after: premiums decline back (markup decays)

2. **Average loss ratio**: Still 0.8 to 1.2 over 50 years
   - Despite catastrophes, long-run should balance
   - Markup adjusts premiums to compensate

3. **More insolvencies**: 3-5 syndicates go insolvent (vs. 1-2 in Scenario 1)
   - Catastrophes are tail events that can exceed capital
   - This validates the model's risk (paper shows more insolvencies in Scenario 2)

4. **Premium volatility**: Higher than Scenario 1
   - Standard deviation of annual average premium > Scenario 1
   - Demonstrates "pronounced cyclicality" mentioned in paper

**Test Assertions:**
```rust
// Count catastrophe years
let cat_years: Vec<_> = time_series.iter()
    .filter(|snapshot| snapshot.avg_loss_ratio > 1.5)
    .collect();
assert!(cat_years.len() >= 1,
    "Should observe at least one catastrophe year (loss ratio > 1.5)");

// Check post-cat premium increase
for cat_year in cat_years {
    let pre_cat_premium = avg_premium_at_year(cat_year.year - 1);
    let post_cat_premium = avg_premium_at_year(cat_year.year + 1);
    assert!(post_cat_premium > pre_cat_premium * 1.1,
        "Premiums should increase by >10% after catastrophe");
}

// More insolvencies than Scenario 1
let insolvencies_scenario2 = count_insolvencies();
let insolvencies_scenario1 = 2; // From Experiment 1
assert!(insolvencies_scenario2 > insolvencies_scenario1,
    "Scenario 2 should have more insolvencies due to catastrophes");
```

**Note**: This also requires premium tracking in time series data.

---

### Experiment 4: Markup Mechanism Validation

**Setup:**
- Extract individual syndicate time series data
- Track markup_m_t values over time (need to add to stats)
- Correlate with loss ratios and premiums

**Expected Outcomes:**
1. **Markup correlation with loss ratio**:
   - High loss year (t) → positive markup at year (t+1)
   - Correlation coefficient > 0.5

2. **Markup autocorrelation**: Moderate persistence
   - m_t and m_{t-1} correlation ≈ 0.2 (β = underwriter_recency_weight)
   - Demonstrates EWMA behavior

3. **Markup mean reversion**: Long-run mean ≈ 0
   - Over 50 years, average markup should be close to 0
   - Indicates market is not systematically biased

**Test Assertions:**
```rust
// Need to add markup_m_t to SyndicateStats first
let markup_loss_correlation = correlate(markups, loss_ratios);
assert!(markup_loss_correlation > 0.5,
    "Markup should be positively correlated with loss ratios");

let long_run_avg_markup = markups.mean();
assert!(long_run_avg_markup.abs() < 0.1,
    "Long-run average markup should be near zero");
```

**Note**: Requires adding `markup_m_t` to `SyndicateStats` for observability.

---

## Implementation Strategy

### Phase 1: Update Current Test (Quick Win)
- Replace `test_market_loss_ratios_are_realistic` with Experiment 1
- Use BrokerPool instead of individual Brokers
- Run 50 years instead of 10
- Tighten bounds to 0.8-1.2
- **Est. time**: 30 minutes

### Phase 2: Add Premium Tracking (Enables Experiments 2 & 3)
- Extend `MarketStatisticsCollector` to track avg premium per year
- Extend `SyndicateStats` to include premium data for aggregation
- Update CSV export to include premium time series
- **Est. time**: 1 hour

### Phase 3: Add Markup Observability (Enables Experiment 4)
- Add `markup_m_t` to `SyndicateStats`
- Update `Syndicate::stats()` to include current markup
- Export markup in CSV for analysis
- **Est. time**: 30 minutes

### Phase 4: Implement Experiments 2-4
- Write test functions with proper assertions
- Run experiments and validate against expectations
- Document any deviations from paper
- **Est. time**: 1-2 hours

---

## Success Criteria

We can confidently claim the simulation is economically valid if:

1. ✅ **Experiment 1 passes**: Loss ratios converge to 0.8-1.2 over 50 years
2. ✅ **Experiment 2 passes**: Premiums converge to fair price (±20%)
3. ✅ **Experiment 3 passes**: Catastrophes cause detectable premium cycles
4. ✅ **Experiment 4 passes**: Markup behaves as EWMA with mean reversion

If experiments fail, we'll have diagnostic information:
- Which mechanisms aren't working (markup? dividend? pricing?)
- Quantitative gaps (how far from expected values?)
- Temporal patterns that reveal bugs (trends vs. cycles vs. noise)

---

## Open Questions to Resolve During Experiments

1. **Volatility loading**: Currently set to 0.0 in config. Should we enable it?
   - Paper mentions α·F_t term in actuarial pricing
   - May improve convergence by adding safety margin

2. **Markup parameter tuning**: β = 0.2 is current default
   - Is this the paper's value?
   - May need adjustment based on experiment results

3. **Industry statistics**: Syndicates use hardcoded industry avg
   - Should they receive dynamic industry stats from MarketStatisticsCollector?
   - Paper mentions λ'_t and μ'_t are dynamic

4. **Follow pricing**: Currently hollow (no pricing strength mechanism)
   - Does this affect loss ratios?
   - Experiment 4 in paper shows lead-follow improves stability
