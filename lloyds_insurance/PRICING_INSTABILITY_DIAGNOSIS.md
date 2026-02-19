# Pricing Instability Diagnosis: Lloyd's Insurance Simulation

## Problem Statement

All scenarios (1, 2, 3, 4) exhibit catastrophic market collapse within 3-6 years, driven by extreme pricing volatility in Years 1-2. This contradicts paper expectations and suggests a fundamental pricing calibration issue.

## Observed Symptoms

### Scenario 3 (VaR EM) Example:
```
Year 0: avg_premium=$54,123, loss_ratio=0.5399 (PROFITABLE)
Year 1: avg_premium=$32,249, loss_ratio=2.1645 (CATASTROPHIC - 40% premium drop!)
Year 2: avg_premium=$90,936, loss_ratio=0.8845 (premium rebounds but damage done)
Year 5: All syndicates insolvent
```

### Scenario 2 Example (Replication 9):
```
Year 0: avg_premium=$56,416, loss_ratio=0.6024 (PROFITABLE)
Year 1: avg_premium=$33,281, loss_ratio=2.0907 (CATASTROPHIC - 41% premium drop!)
Year 3: All syndicates insolvent
```

## Root Cause Analysis

### 1. Over-Aggressive Markup Adjustment

**Code Location:** `syndicate.rs` lines 473-504

```rust
fn update_underwriting_markup(&mut self) {
    let signal = loss_ratio.ln();  // log(loss_ratio)
    let beta = self.config.underwriter_recency_weight;  // 0.2
    self.markup_m_t = beta * self.markup_m_t + (1.0 - beta) * signal;
    //                20% old value    +    80% NEW SIGNAL
}
```

**The Problem:**
- Current year gets **80% weight** (`1.0 - 0.2 = 0.8`)
- Previous markup gets only **20% weight**
- No warmup period for early years

**Impact on Year 1 Pricing:**

When Year 0 is profitable (loss_ratio = 0.54):
```
signal = ln(0.54) = -0.616
markup_m_t = 0.2 * 0.0 + 0.8 * (-0.616) = -0.493
premium_multiplier = exp(-0.493) = 0.61  (39% CUT!)
```

This aggressive premium reduction leaves syndicates catastrophically under-priced for Year 1.

### 2. Asymmetric Warmup Logic

**Industry Stats Update** (syndicate.rs lines 628-634):
- Year 0: 10% weight on new data (cautious)
- Years 1-4: 20% weight on new data
- Years 5+: 40% weight on new data

**Markup Update** (syndicate.rs lines 473-504):
- **ALL YEARS: 80% weight on new data (NO WARMUP!)**

**The Mismatch:**
- Industry statistics (claim frequency, claim size) adjust slowly (10-40%)
- Pricing markup adjusts rapidly (80%)
- Result: Premiums swing wildly based on stale industry data

### 3. Insufficient Volatility Buffer

**Configuration** (lib.rs line 482):
```rust
volatility_weight: 0.2,  // Only 20% safety margin
```

**Expected Loss Calculation:**
- Attritional: 0.1 frequency × $1.5M per claim = $150k expected
- With 20% loading: $180k premium
- **But catastrophes can cause $2-3M losses!**

The 20% volatility buffer is adequate for attritional losses but grossly insufficient for catastrophe exposure.

### 4. Capital Extraction During Profitable Years

**Dividend Policy** (lib.rs line 488):
```rust
profit_fraction: 0.4,  // Pay out 40% of profits
```

**Cycle:**
1. Year 0 profitable → 40% dividend paid → capital reduced
2. Year 0 profitable → markup drops 39% → premiums slashed
3. Year 1 under-priced + reduced capital → catastrophe → insolvency

## Quantitative Evidence

### Markup Volatility (Year 0 → Year 1):

**Scenario 3:**
- Year 0: markup_avg = -0.5062
- Year 1: markup_avg = +0.5179
- **Swing: +1.024 (101% change in one year!)**

**Scenario 2 Rep 9:**
- Year 0: markup_avg = -0.4142
- Year 1: markup_avg = +0.4904
- **Swing: +0.9046 (90% change)**

This extreme volatility creates a destructive boom-bust cycle.

## Comparison to Paper Expectations

**Paper Section 4.3.2** states:
> "m_t captures competitive pressure based on loss experience"

This suggests gradual adjustment to market conditions, not violent swings.

**Paper Figure 4** shows:
- Gradual capital growth/decline over 50 years
- Multiple syndicates surviving to year 50
- Underwriting cycles visible but NOT catastrophic

**Our Implementation:**
- Catastrophic collapse within 3-6 years
- 100% insolvency in Scenario 3 (VaR EM)
- 50% insolvency in Scenario 2 (catastrophe only)

## Proposed Fixes

### Fix 1: Add Warmup Period to Markup Adjustment

```rust
fn update_underwriting_markup(&mut self) {
    if let Some(loss_ratio) = current_year_loss_ratio {
        let signal = loss_ratio.ln();

        // WARMUP PERIOD: Match industry stats logic
        let beta = if self.years_elapsed == 0 {
            0.9  // Year 0: 90% weight on old markup (only 10% new)
        } else if self.years_elapsed < 5 {
            0.8  // Years 1-4: 80% weight on old markup (20% new)
        } else {
            0.2  // Years 5+: Current behavior (80% new)
        };

        self.markup_m_t = beta * self.markup_m_t + (1.0 - beta) * signal;
    }
}
```

### Fix 2: Increase Base Volatility Weight for Catastrophe Scenarios

```rust
pub fn scenario_2() -> Self {
    Self {
        mean_cat_events_per_year: 0.05,
        volatility_weight: 0.5,  // Increase from 0.2 to 0.5 (50% buffer)
        ..Self::default()
    }
}

pub fn scenario_3() -> Self {
    Self {
        mean_cat_events_per_year: 0.05,
        volatility_weight: 0.5,  // Increase from 0.2 to 0.5
        var_safety_factor: 0.7,
        ..Self::default()
    }
}
```

### Fix 3: Reduce Dividend Payout to Preserve Capital

```rust
profit_fraction: 0.2,  // Reduce from 0.4 (40%) to 0.2 (20%)
```

This retains more capital as buffer against future catastrophes.

### Fix 4: Increase Default Underwriter Recency Weight

```rust
underwriter_recency_weight: 0.5,  // Increase from 0.2 to 0.5
// New signal now gets 50% weight instead of 80%
```

This provides more stability while still responding to market conditions.

## Testing Strategy

1. **Unit Test:** Verify markup adjustment with warmup period
2. **Scenario Test:** Run Scenario 2 with fixes, expect bimodal distribution (monopoly vs collapse)
3. **Scenario Test:** Run Scenario 3 with fixes, expect uniform_deviation → 0 and fewer insolvencies than Scenario 2
4. **Ensemble Test:** Run 100 replications to verify statistical properties match paper

## Expected Outcomes After Fixes

### Scenario 2 (Catastrophes):
- Underwriting cycles visible (hard/soft market alternation)
- Bimodal distribution: Some monopolies, some collapses
- Survivors reach year 50 in ~50% of replications

### Scenario 3 (VaR EM):
- Uniform deviation approaches zero (better risk distribution)
- **Fewer insolvencies than Scenario 2** (currently WORSE!)
- Better capitalized portfolios

## Current Status

**Diagnosis Complete:** Identified four interacting issues causing instability
**Fixes Proposed:** Warmup period, increased volatility buffer, reduced dividends, increased markup stability
**Next Step:** Implement fixes and validate against paper expectations
