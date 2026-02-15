# Lloyd's Insurance Implementation Review

**Date**: 2026-02-15
**Scope**: Implementation vs. Olmez et al. (2024) paper goals
**Prior review**: `REVIEW_AND_PROPOSED_CHANGES.md` (2026-02-04)

---

## Summary

The implementation is a competent translation of the paper's *architecture* into the DES framework, but it does not yet produce evidence sufficient to validate the paper's scientific claims. Several agents exist primarily as event routers with no meaningful state, inflating the agent count without adding simulation fidelity. The core economics are approximately correct but undertested, and key mechanisms that drive the paper's headline findings (underwriting markup, dividends, exposure management) are absent.

---

## 1. Scenario Configuration is Contradictory

The `ModelConfig::default()` sets `mean_cat_events_per_year: 0.05`, and `scenario_1()` returns this default. But the paper defines Scenario 1 as **attritional losses only** — no catastrophes.

```rust
// Current: scenario_1 includes catastrophes (wrong)
pub fn scenario_1() -> Self { Self::default() }

// scenario_2 is identical to scenario_1
pub fn scenario_2() -> Self {
    Self { mean_cat_events_per_year: 0.05, ..Self::default() }  // already the default
}
```

This means `main.rs` claims to run "Scenario 1 (Base Case - Attritional Only)" while actually including catastrophe events. `scenario_1()` should set `mean_cat_events_per_year: 0.0`, or the default should be 0.0 with scenario_2 enabling it.

As things stand, Scenarios 1 and 2 are indistinguishable.

---

## 2. Agents That Should Not Be Agents

The review brief asks to favour fewer agents and identify agents that exist purely to track state. Three agents in the current design are candidates for removal or consolidation.

### 2a. BrokerSyndicateNetwork — a stateless function as an agent

This agent holds no meaningful state (just an RNG and a config reference). It receives **every** event broadcast and ignores all but two. It returns a dummy `BrokerStats` with broker_id 0 for its stats, which is misleading. Its entire purpose is a pure function: "given a risk, pick k random syndicates."

**Suggestion**: Fold this logic into the Broker. Each Broker already knows the market structure (via config). The Broker could select syndicates when it broadcasts a risk. This eliminates one agent from the broadcast loop and removes the fake stats return.

Alternatively, fold it into the CentralRiskRepository — when a risk is registered, the repo selects syndicates and emits quote requests. This keeps routing centralised.

### 2b. TimeGenerator — a timer, not a simulation entity

`TimeGenerator` is a clock, not a market participant. It returns a dummy `Stats::BrokerStats(BrokerStats::new(0))`, which pollutes the stats output. It has one field (`current_day: usize`) and one behaviour: emit the next Day event, plus Month/Year on schedule.

**Suggestion**: The DES framework could support seed events that self-replicate (which is what TimeGenerator already does via `vec![(current_t + 1, Event::Day)]`). If this agent must exist, at a minimum give it an honest stats type (`Stats::None` or `Stats::TimeGenerator { days_elapsed }`) rather than borrowing BrokerStats.

### 2c. 25 Brokers — agent proliferation without differentiation

The simulation creates 25 Broker agents, all with identical configuration. Each broker independently samples from the same Poisson distribution with the same λ. Because events are broadcast to all agents, every event passes through 25 identical brokers, each checking `self.our_risks.contains(risk_id)` for an event that belongs to at most one of them.

**Suggestion**: Consolidate into a single `BrokerPool` agent that internally manages N broker states. It generates risks for all brokers in one `act()` call on Day events, and tracks risk ownership in a single HashMap. This reduces the broadcast fan-out from 35 agents to 11 without changing the simulation dynamics.

The paper uses 25 brokers to provide statistical mass (many independent risk sources). A single agent with 25 internal generators produces the same event stream.

---

## 3. Actuarial Pricing: Correct on Paper, Weakly Validated

The pricing formula `P̃_t = z·X̄_t + (1-z)·λ'_t·μ'_t` is implemented. The unit test `test_actuarial_price_calculation` verifies the no-history case only. The integration test `test_market_loss_ratios_are_realistic` accepts loss ratios between 0.5 and 1.8 — a 3.6x-wide window that would pass even with seriously broken economics.

The paper expects premiums to converge to ~$300k (for the full risk). With `default_lead_line_size: 0.5`, the lead premium should be ~$150k. After 10 simulated years with only 2 syndicates and 2 brokers, variance is high, but the test's acceptance range is so broad that it provides almost no signal.

**Suggestion**: Either run a longer simulation in the test (50 years, 5 syndicates, 25 brokers — matching the paper), or tighten the bounds. A test that passes when the answer is both 0.5 and 1.8 isn't validating anything.

---

## 4. Missing Mechanisms That Drive the Paper's Key Findings

### 4a. Underwriting markup (Equation 3)

The paper's underwriting cycle — its primary contribution — requires `P_t = P_at · e^(m_t)`, where `m_t` is an exponentially weighted moving average of market conditions. Without it, premiums adjust only through the slow-moving actuarial EWMA. The sharp post-catastrophe spikes in Figure 6 cannot emerge.

### 4b. Dividends (Equation 5)

`D = γ · Pr_t` extracts 40% of profits each year. Without dividends, syndicates accumulate capital indefinitely, making insolvency nearly impossible. The paper shows insolvencies even in Scenario 1 — but here, syndicates never give back profits, so capital only grows (minus claims). This fundamentally changes the dynamics.

### 4c. Premium and VaR exposure management

Without exposure management, syndicates accept every risk regardless of concentration. The paper's Scenario 3 (VaR EM → uniform deviation → 0) is unimplementable.

### 4d. Dynamic industry statistics

Syndicates use `self.config.gamma_mean * self.config.yearly_claim_frequency` as the industry average — a hardcoded constant. The paper has syndicates receive actual industry-wide statistics that change over time. This prevents the feedback loop where a catastrophe changes industry loss experience, which changes pricing across the market.

---

## 5. Unbounded State Growth

`CentralRiskRepository` stores every risk and every policy in HashMaps that grow without bound. Over 50 simulated years:
- `risks`: ~27k entries
- `policies`: ~27k entries
- `lead_quotes` and `follow_quotes`: ~54k entries

None are ever cleaned up. For policies that have expired (risk expiration_time has passed), the quotes, risk, and policy records serve no further purpose. The same applies to `Syndicate.policies` and `Syndicate.loss_history` — both are append-only vectors.

For a 50-year simulation this is tolerable (~tens of MB), but it would prevent scaling to longer runs or larger markets.

---

## 6. Follow Mechanics are Wired but Hollow

`BrokerSyndicateNetwork` sends `FollowQuoteRequested` with `lead_price: 0.0` hardcoded. The `Syndicate` handler ignores `_lead_price` entirely. The follow pricing formula `(full_risk_price / default_lead_line_size) * line_size` is a linear rescaling of the lead price — it doesn't implement the paper's "pricing strength" mechanism where followers compare their own price to the lead's price to decide line size.

The infrastructure is in place, but the follow quotes have no economic content.

---

## 7. Catastrophe Loss Distribution

`sample_pareto_loss` implements a standard Pareto via inverse CDF, but the paper specifies a **truncated** Pareto. The code caps at `x_min * 100` as a hard ceiling. The paper's truncation likely uses a different mechanism (truncation at the risk limit, for instance). This affects the tail behaviour of catastrophe losses.

Also, catastrophe losses are distributed equally among affected risks (`loss_per_risk = total_loss / affected_risks.len()`). The paper applies the full catastrophe loss to each risk's limit proportionally — the loss per risk should be `min(loss_share, risk.limit)`, capped at the policy limit.

---

## 8. What the Outputs Actually Tell Us

Running the simulation produces final-state statistics only. There is no time-series data despite `TimeSeriesStats` and `MarketSnapshot` types existing in the codebase — they are defined but never populated.

Without temporal output, we cannot observe:
- Premium convergence to fair price (Scenario 1)
- Post-catastrophe premium spikes (Scenario 2)
- Uniform deviation trend (Scenario 3)
- Loss ratio coupling (Scenario 4)

The `export_time_series_csv` function in `main.rs` writes a single-row-per-syndicate summary of final state, not a time series. The `TimeSeriesStats` type is entirely unused.

**This is the most significant gap**: the paper's validation is entirely based on temporal dynamics (Figures 5-12). Without time-series output, the implementation cannot produce the evidence needed to evaluate whether it replicates the paper.

---

## 9. Positive Observations

- The event types faithfully mirror the paper's event taxonomy
- The core DES loop works correctly and efficiently (~1 second for 50 simulated years)
- The testing philosophy (stats as observable state) is followed consistently
- The code is well-organised: one agent per file, clear separation of types
- Attritional loss generation correctly uses Poisson frequency × Gamma severity
- The CentralRiskRepository's lead selection (cheapest quote wins) and catastrophe cascade logic are correct
- Seeded RNGs throughout enable reproducibility

---

## Recommendations (Prioritised)

1. ✅ **COMPLETED - Fix scenario configs**: Make scenario_1 have `mean_cat_events_per_year: 0.0`
   - Changed `scenario_1()` to explicitly set `mean_cat_events_per_year: 0.0`
   - Added tests: `test_scenario_1_has_no_catastrophes`, `test_scenario_2_has_catastrophes`, `test_scenarios_1_and_2_are_distinct`
   - Verified simulation output: 0 catastrophes, $0 catastrophe loss
2. ✅ **COMPLETED - Add dividend payments** on Year events — formula D = γ · Pr_t
   - Added `annual_premiums` and `annual_claims` tracking to Syndicate
   - Implemented `handle_year_end()`: calculates annual profit, pays dividend (40% of profit if positive), resets counters
   - Added `Event::Year` handler in `act()` method
   - Added `total_dividends_paid` to SyndicateStats
   - Added 4 tests: profitable year, loss year, accumulation, event triggering
   - Verified simulation output: syndicates paying dividends ($2.29M, $1.91M, etc.) making capital dynamics realistic
3. ✅ **COMPLETED - Populate TimeSeriesStats** from syndicate capital/premium/loss data on Year events
   - Created `MarketStatisticsCollector` agent to aggregate market snapshots over time
   - Syndicates emit `SyndicateCapitalReported` events on Year (even when insolvent)
   - Collector aggregates reports and creates `MarketSnapshot` entries
   - Updated CSV export to write actual time series data (year, capital, loss ratios, solvency counts)
   - Added 4 tests for collector behavior
   - Verified simulation output: 50 annual snapshots showing capital evolution and insolvencies over time
4. ✅ **COMPLETED - Consolidate brokers** into a single BrokerPool agent
   - Created `BrokerPool` agent managing N broker states internally
   - Each broker maintains independent RNG, risk ID counter, and stats
   - Risk ownership tracked in single HashMap for O(1) lookup vs O(N) broadcast
   - Reduces agent count from 36 to 11 (71% reduction in broadcast fan-out)
   - Added 6 tests for BrokerPool behavior
   - Verified simulation output: identical behavior to 25 individual brokers
5. ✅ **COMPLETED - Fold BrokerSyndicateNetwork** into CentralRiskRepository
   - Moved syndicate selection logic into CentralRiskRepository
   - Added config, num_syndicates, rng fields to CentralRiskRepository
   - RiskBroadcasted event now triggers LeadQuoteRequested events
   - LeadQuoteAccepted event now triggers FollowQuoteRequested events
   - Removed BrokerSyndicateNetwork agent entirely
   - Reduces agent count from 11 to 10 (9% reduction in broadcast fan-out)
   - Added 3 tests for syndicate selection behavior
   - Verified simulation output: identical behavior with one less agent
6. ✅ **COMPLETED - Implement underwriting markup** (Equation 3: P_t = P_at · e^(m_t))
   - Added `markup_m_t` field to Syndicate to track EWMA of market conditions
   - Implemented `apply_underwriting_markup()` method: multiplies actuarial price by e^(m_t)
   - Implemented `update_underwriting_markup()` method: EWMA using loss ratios
   - Formula: m_t = β · m_{t-1} + (1-β) · log(loss_ratio_t)
   - High loss ratios (>1) → positive m_t → higher premiums (competitive pressure)
   - Low loss ratios (<1) → negative m_t → lower premiums (market competition)
   - Markup updated on Year events using annual loss ratio data
   - Applied markup in lead and follow quote handlers
   - Added 3 tests verifying markup mechanism works correctly
   - Verified simulation output: premiums now respond to loss experience
7. **Tighten the loss ratio test** or add a longer-running integration test with narrower bounds
8. **Implement follow pricing strength** — the lead-follow story is the paper's strongest validation (zero insolvencies in Scenario 4)
