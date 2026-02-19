# Status: Failing Long-Tests — Diagnosis & Plan

**Date**: 2026-02-19
**Branch**: `lloyds`
**Test status**: 85 unit tests pass; 6 long-tests still fail (all behind `#[cfg(feature = "long-tests")]`)

---

## What Was Fixed This Session

Four bugs were fixed; all 85 unit tests now pass.

### 1. Follow quotes had no Premium EM check
`handle_follow_quote_request` in `syndicate.rs` ran VaR EM (Scenario 3) but had no
Premium EM fallback for Scenarios 1/4. Added the `else` branch with a proportional
line-size reduction on `ScalePremium` (can't scale price as a follower).

### 2. Premium cap expanded as premiums arrived (wrong denominator)
`check_premium_exposure` used `self.capital` as the denominator. Because capital
grows as each premium is collected, the 50% cap silently expanded from $5M to ~$9.7M
over the year. Fixed by adding `capital_at_year_start: f64` field, snapshotting it at
year-end (after dividends) in `handle_year_end`, and using it as the denominator.

### 3. S1 and S4 were structurally identical (both had `follow_top_k: 5`)
Per the paper S1 has no lead-follow mechanics; S4 adds following. Changed the default
`follow_top_k: 0`. S1 uses `..Self::default()`. S4 explicitly sets `follow_top_k: 5`.
One unit test (`test_responds_to_lead_quote_accepted_with_follow_requests`) that used
`ModelConfig::default()` needed to switch to `ModelConfig { follow_top_k: 5, .. }`.

### 4. EWMA markup had Jensen's inequality bias → systematic underpricing
The signal was `ln(LR)`. Because `E[ln(X)] < ln(E[X])` (Jensen), with a fair market
where `E[LR] = 1`, the signal has negative mean: `E[ln(LR)] < 0`. The EWMA therefore
drifted negative, meaning markup settled at ~-0.15 (≈14% underpricing), causing
`E[LR] ≈ 1.16` at equilibrium — syndicates perpetually lose money.

Fix: signal changed to `LR - 1` (unbiased: `E[LR - 1] = 0` when `E[LR] = 1`).
EWMA formula: `m_t = (1 - α) * m_{t-1} + α * (LR_t - 1)`, where
`underwriter_recency_weight` is now α (weight on new signal, standard EWMA).
Warmup: α = 0.05 (year 0), 0.10 (years 1–4), config value (years 5+, default 0.20).
Four markup unit tests updated to match new expected values.

---

## Root Cause of Remaining Failures

### The ScalePremium "soft cap" problem

**Location**: `syndicate.rs:250–253`, `handle_lead_quote_request`, Premium EM branch.

```rust
ExposureDecision::ScalePremium(factor) => {
    // Scale premium up to reduce attractiveness
    price *= factor;
    // BUG: still falls through to LeadQuoteOffered — risk is still written!
}
```

The intention of `ScalePremium` is to make the quote less attractive, but:
1. **The quote is still offered.** Brokers may still select it (especially if other
   syndicates are also at cap, making this the only/best quote).
2. **The premium recorded at acceptance is unscaled.** `handle_lead_accepted` (line 275)
   recalculates price independently — without the scaling factor — so `annual_premiums`
   grows by the unscaled amount regardless of what was quoted.
3. **Net effect**: syndicates write ≈$9.7M in annual premiums (≈97% of $10M capital)
   instead of the intended $5M (= 50% × $10M). The cap is completely ineffective.

**Evidence**: The Scenario 4 "zero insolvencies" test observed 50 insolvencies (10 reps
× 5 syndicates), with all syndicates going bankrupt by year 7–8. This makes sense: at
97% premium-to-capital loading, a single bad year causes insolvency. With S4's
Gamma(1,1) loss distribution (CV = 1.0, annual claims CV ≈ 77% for ~33 lead risks),
insolvency probability is ~30%+ per syndicate-year rather than near-zero.

### Secondary issue: scoped nature of the cap

Even with the hard cap enforced at 50% of capital, there is a question of whether the
Gamma(1,1) severity distribution naturally produces zero insolvencies in S4 over
500 syndicate-years (10 reps × 50 years × 5 syndicates). Rough estimates:

- At $5M annual premiums and $10M capital, syndicates write ~33 lead risks/year
- Gamma(1,1) (exponential, shape=1): CV of annual claims ≈ 100% / √33 ≈ 17% per-risk
  aggregate, but per-policy CV = 1.0, so total CV ≈ 1/√33 ≈ 17% of expected claims
- Expected claims ≈ $5M (at fair price), so annual claims std dev ≈ $850k
- P(insolvency in a year) ≈ P(claims > $10M capital) — low but non-zero
- With S4's diversification across syndicates via following, individual syndicate
  exposures differ — the paper claims this diversification → zero insolvencies

The hard cap fix is necessary first. If insolvencies persist after that, revisit
the variance/capital parameters or the paper's assumptions about diversification scale.

---

## Recommended Next Steps (in order)

### Step 1: Make Premium EM a hard cap (change `ScalePremium` → `Reject` for leads)

In `syndicate.rs`, `handle_lead_quote_request`, around line 250:

**Before**:
```rust
ExposureDecision::ScalePremium(factor) => {
    // Scale premium up to reduce attractiveness
    price *= factor;
}
```

**After**:
```rust
ExposureDecision::ScalePremium(_factor) => {
    // Hard cap: premium budget exhausted, decline rather than offering at inflated price.
    // Scaling the price up doesn't actually prevent the risk being written (broker may
    // still accept), so we treat this as a reject to enforce the 50% cap strictly.
    return Vec::new();
}
```

Also remove the `mut` from `let mut price` if VaR EM's `ScalePremium` is the only
remaining user of price mutation (check the VaR EM branch first).

The `check_premium_exposure` function's `ScalePremium` path (lines 196–202) may also
need adjustment: once `ScalePremium` is treated as `Reject` by all callers, there is
no point emitting it. Consider changing `check_premium_exposure` to return only
`Accept` or `Reject`, or keep `ScalePremium` for the VaR EM branch which still uses it.

### Step 2: Run test_scenario4_has_zero_insolvencies in isolation

```bash
cargo test --release -p lloyds_insurance --features long-tests -- test_scenario4_has_zero_insolvencies --nocapture 2>&1 | tail -20
```

This test takes ~150s. Expected outcome after Step 1: 0 insolvencies across 10 reps.

### Step 3: Run remaining failing tests one by one

In order of increasing complexity:
1. `test_scenario4_has_zero_insolvencies` (Step 2 above)
2. `test_scenario4_has_highly_correlated_loss_ratios` — S4 followers share lead's risk, so corr should be high
3. `test_scenario1_has_lower_loss_ratio_correlation_than_scenario4` — S1 no sharing → low corr
4. `test_scenario2_has_more_insolvencies_than_scenario1` — S2 adds cat losses → more insolvencies
5. `test_scenario3_has_fewer_insolvencies_than_scenario2` — S3 VaR EM → less exposure → fewer insolvencies
6. `test_scenario3_has_lower_uniform_deviation_than_scenario2` — S3 VaR EM → more uniform pricing

### Step 4: Remove diagnostic output from test_scenario4_has_zero_insolvencies

The test has a `println!` block dumping per-rep/per-syndicate bankruptcy data. Remove
this once the test passes.

---

## Key Files

| File | Purpose |
|---|---|
| `lloyds_insurance/src/syndicate.rs` | Syndicate agent: pricing, EM, markup EWMA |
| `lloyds_insurance/src/lib.rs` | ModelConfig, scenario constructors, all tests |
| `lloyds_insurance/src/central_risk_repository.rs` | Risk lifecycle, lead/follow selection |
| `lloyds_insurance/src/syndicate_var_exposure.rs` | VaR exposure manager (Scenario 3) |

## Key Config Parameters

| Parameter | Default | Meaning |
|---|---|---|
| `initial_capital` | $10M | Per-syndicate starting capital |
| `premium_reserve_ratio` | 0.5 | Premium cap = 50% of capital_at_year_start |
| `follow_top_k` | 0 | Number of followers per risk; 0 = no following (S1), 5 = S4 |
| `underwriter_recency_weight` | 0.2 | α in EWMA: weight on new LR signal |
| `yearly_claim_frequency` | 0.1 | Claims per risk per year |
| `gamma_mean` | 500k | Mean claim size per unit exposure |
| `gamma_cov` | 1.0 | CV of claim severity (1.0 = exponential) |
