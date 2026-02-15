# VaR Exposure Management Fix

## Problem Statement

After implementing the experimental validation framework, Experiment 3 revealed that Scenarios 2 (catastrophes without VaR EM) and Scenario 3 (catastrophes with VaR EM) produced **identical results**:

- Both scenarios: 4-5 syndicates insolvent by year 49
- Both scenarios: `avg_uniform_deviation = 0.0000` (never changed from zero)
- VaR exposure management had zero observable effect

Parameter tuning (`var_safety_factor: 1.0 → 0.4`) did not resolve the issue, indicating a deeper implementation problem.

## Root Cause Analysis

Investigation revealed **two critical bugs** in the VaR Exposure Management integration:

### Bug #1: `uniform_deviation` Metric Never Updated

**Location**: `lloyds_insurance/src/syndicate.rs:510-518` (`update_stats()` method)

**Problem**: The `update_stats()` method never retrieved `uniform_deviation` from the `VarExposureManager`:

```rust
fn update_stats(&mut self) {
    self.stats.capital = self.capital;
    self.stats.update_loss_ratio();
    self.stats.update_profit();
    self.stats.markup_m_t = self.markup_m_t;

    // Update exposure by peril region (simplified - would need risk info)
    // For now, just track total exposure
    // ❌ BUG: uniform_deviation never updated!
}
```

**Impact**:
- `self.stats.uniform_deviation` remained at its initial value (0.0) forever
- `Event::SyndicateCapitalReported` sent 0.0 to `MarketStatisticsCollector`
- `avg_uniform_deviation` in market snapshots was always 0.0
- No way to observe VaR EM effectiveness

### Bug #2: VaR Manager Capital Never Synchronized

**Location**: Multiple capital modification points in `syndicate.rs`

**Problem**: When syndicate capital changed, `VarExposureManager.update_capital()` was never called:

```rust
// Premium collection (lead quotes)
self.capital += price;
// ❌ BUG: VaR manager capital not updated!

// Premium collection (follow quotes)
self.capital += price;
// ❌ BUG: VaR manager capital not updated!

// Claim handling
self.capital -= amount;
// ❌ BUG: VaR manager capital not updated!

// Dividend payment
self.capital -= dividend;
// ❌ BUG: VaR manager capital not updated!
```

**Impact**:
- VaR threshold calculated as: `threshold = capital * var_safety_factor`
- Capital stuck at initial value (10,000,000) instead of tracking actual capital
- As syndicates accumulated profits, actual capital increased but VaR threshold stayed constant
- VaR constraints became progressively less binding over time
- By year 10+, constraints were effectively disabled

## Solution

### Fix #1: Update `uniform_deviation` in `update_stats()`

**File**: `lloyds_insurance/src/syndicate.rs`

```rust
fn update_stats(&mut self) {
    self.stats.capital = self.capital;
    self.stats.update_loss_ratio();
    self.stats.update_profit();
    self.stats.markup_m_t = self.markup_m_t;

    // ✅ FIX: Update uniform_deviation from VaR manager if enabled
    if let Some(ref var_em) = self.var_exposure_manager {
        self.stats.uniform_deviation = var_em.uniform_deviation();
    } else {
        self.stats.uniform_deviation = 0.0;
    }
}
```

### Fix #2: Synchronize Capital in All Modification Points

**File**: `lloyds_insurance/src/syndicate.rs`

#### Premium Collection (Lead & Follow Quotes)
```rust
// Record exposure in VaR manager and update capital
if let Some(ref mut var_em) = self.var_exposure_manager {
    var_em.record_exposure(peril_region, exposure);
    var_em.update_capital(self.capital);  // ✅ Added
}
```

#### Claim Handling
```rust
fn handle_claim(&mut self, _risk_id: usize, amount: f64) -> Vec<(usize, Event)> {
    self.capital -= amount;
    self.loss_history.push(amount);
    self.annual_claims += amount;
    self.annual_claims_count += 1;

    // ✅ FIX: Update VaR manager capital if enabled
    if let Some(ref mut var_em) = self.var_exposure_manager {
        var_em.update_capital(self.capital);
    }

    self.stats.total_claims_paid += amount;
    self.stats.num_claims += 1;
    // ...
}
```

#### Dividend Payment
```rust
if annual_profit > 0.0 {
    let dividend = self.config.profit_fraction * annual_profit;
    if self.capital >= dividend {
        self.capital -= dividend;
        self.stats.total_dividends_paid += dividend;

        // ✅ FIX: Update VaR manager capital if enabled
        if let Some(ref mut var_em) = self.var_exposure_manager {
            var_em.update_capital(self.capital);
        }
    }
}
```

## Verification

### Test Suite
All 67 tests pass after fixes:
```bash
cargo test -p lloyds_insurance --lib
# test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Experiment 3 Re-run
Running Experiment 3 with fixes applied to verify:
1. `avg_uniform_deviation` now shows non-zero values in Scenario 3
2. Scenarios 2 and 3 produce **different** outcomes
3. VaR EM reduces insolvencies and achieves uniform exposure distribution

Expected results:
- **Scenario 2** (no VaR EM): Higher insolvency rates, concentrated exposure
- **Scenario 3** (with VaR EM): Lower insolvency rates, `avg_uniform_deviation < 0.05`

## Lessons Learned

1. **Stats-driven observability**: The bug was only discovered when we added `avg_uniform_deviation` to market snapshots for experimental validation. Without observable metrics, silent failures can persist.

2. **State synchronization**: When wrapper objects (like `VarExposureManager`) maintain shadow state, explicit synchronization is critical. Consider:
   - Centralizing capital updates through a single method
   - Using Rust's type system (e.g., newtype pattern) to enforce synchronization
   - Adding debug assertions to catch desynchronization

3. **Incremental implementation risks**: The comment "For now, just track total exposure" in `update_stats()` was a TODO that became a latent bug. When deferring implementation:
   - Add `todo!()` or `unimplemented!()` macros to fail fast
   - Create tracking issues for incomplete features
   - Add tests that verify end-to-end behavior, not just unit behavior

## Timeline

- **2025-02-15 18:00**: Experiment 3 revealed VaR EM not working
- **2025-02-15 18:30**: Investigated `syndicate_var_exposure.rs` - implementation correct
- **2025-02-15 18:45**: Found Bug #1 in `update_stats()` - `uniform_deviation` never updated
- **2025-02-15 19:00**: Found Bug #2 - capital synchronization missing
- **2025-02-15 19:15**: Applied fixes, all tests passing
- **2025-02-15 19:20**: Re-running Experiment 3 for verification
