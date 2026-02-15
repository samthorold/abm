# Insurance Industry Complex Social System

Implementation of **Owadally et al. (2018)**: "Insurance Industry as a Complex Social System: Competition, Cycles and Crises"

## Overview

This ABM demonstrates how simple individual-level firm behaviors generate complex industry-wide underwriting cycles through the interplay between:

- **Actuaries**: Use credibility theory to blend own experience with industry data
- **Underwriters**: Apply markup based on price elasticity to maximize profit

**Key Finding**: Endogenous cycles emerge (**5.0 year spectral period**, paper reports 5.9 years) without external shocks, driven purely by feedback between pricing and market allocation.

## Architecture

### Pattern: Coordinator

Following the **Coordinator + Entity Agents** pattern (like `zi_traders`):

- **MarketCoordinator**: Orchestrates annual cycle, allocates customers, tracks industry statistics
- **Insurer** (×20): Two-stage pricing (actuarial + underwriter markup)
- **ClaimGenerator**: Stochastic claim generation (Bernoulli × Gamma)

### Why Coordinator Pattern?

1. **Annual cycle orchestration**: Clear sequence (pricing → allocation → claims → year-end)
2. **Market clearing**: All 1000 customers allocated with capacity constraints
3. **Industry statistics**: Coordinator naturally computes aggregate metrics
4. **Separation of concerns**: Market mechanism vs. participant behavior

### Customers: Data Structures (Not Agents)

Customers are simple data structures, not agents, because:
- Simple behavior: calculate cost, choose lowest
- No learning or interaction
- Avoids 1000× broadcast overhead
- **Trade-off**: Loses extensibility for customer learning, gains major performance

## Implementation Details

### Two-Stage Pricing

**Stage 1 - Actuarial Price**:
```
blended_claim = z × ewma_claim + (1-z) × industry_avg_claim
actuarial_price = blended_claim + α × σ_claims
```

**Stage 2 - Underwriter Markup**:
```
ε = price_elasticity()  // Arc elasticity from last 2 years
m_hat = -1 / (1 + ε)    // if ε < -1 (elastic demand)
m_t = β × m_hat + (1-β) × m_{t-1}  // Smoothed update
market_price = actuarial_price × exp(m_t)
```

### Market Clearing

Greedy allocation algorithm:
```
for each customer:
    total_cost = price + γ × circular_distance(customer, insurer)
    allocate to insurer with minimum total_cost
```

**Circular Preference Landscape**: Customers and insurers positioned on [0, 2π) with wraparound distance.

### Claim Generation

For each customer-insurer pair:
1. **Bernoulli trial**: Does claim occur? (frequency = 1.0 = certain)
2. **Gamma distribution**: Sample claim amount (μ=100, σ=10)
3. **Random timing**: Schedule during year [year×365, (year+1)×365)

## Running Simulations

### Basic Run

```bash
cargo run -p insurance_cycles
```

Runs 100-year baseline simulation with β=0.3.

### Batch Experiments & Analysis

**NEW**: Comprehensive experimental framework for parameter sweeps, Monte Carlo analysis, and statistical validation.

See **[EXPERIMENTS.md](EXPERIMENTS.md)** for detailed documentation.

#### Quick Start

```bash
# Run baseline validation (30 runs × 100 years)
cargo run --release --bin run_experiment -- experiments/baseline_validation.toml

# Analyze results with Python
cd analysis
python3 -m venv venv && source venv/bin/activate
pip install -r requirements.txt
python cycle_analysis.py ../results/baseline_validation/
```

#### Available Experiments

- `baseline_validation.toml` - Validate against paper (30 runs)
- `beta_sensitivity.toml` - β sweep: [0.1, 1.0] (100 runs)
- `credibility_sweep.toml` - z sweep: [0.0, 0.5] (50 runs)
- `distance_cost_sweep.toml` - γ sweep: [0.02, 0.2] (50 runs)
- `leverage_sensitivity.toml` - Leverage sweep: [1.5, 3.0] (50 runs)

#### Outputs

- **CSV files**: Market time series, insurer snapshots
- **JSON summaries**: Cycle metrics, AR(2) coefficients, aggregate statistics
- **Python analysis**: Cycle detection, spectral analysis, visualization

### Parameter Sensitivity (Manual)

Modify `main.rs` to test different configurations:

```rust
let config = ModelConfig::low_beta();    // β=0.2 → stable cycles
let config = ModelConfig::baseline();    // β=0.3 → medium cycles
let config = ModelConfig::high_beta();   // β=0.6 → volatile
let config = ModelConfig::white_noise(); // β=1.0 → no cycles
```

## Model Parameters

### Critical Behavioral Parameters

| Parameter | Symbol | Baseline | Effect |
|-----------|--------|----------|--------|
| Underwriter smoothing | β | 0.3 | **CRITICAL**: Controls cycle stability |
| Risk loading factor | α | 0.001 | Actuarial risk premium |
| Distance cost | γ | 0.08 | Customer preference sensitivity |

### Actuarial Parameters

| Parameter | Symbol | Baseline |
|-----------|--------|----------|
| Credibility factor | z | 0.2 |
| EWMA smoothing | w | 0.2 |

### Claims Distribution

| Parameter | Distribution | Values |
|-----------|--------------|--------|
| Frequency | Bernoulli | b = 1.0 |
| Severity | Gamma | μ = 100, σ = 10 |

### Market Structure

- **Insurers**: N = 20
- **Customers**: M = 1000
- **Initial capital**: $10,000
- **Leverage ratio**: 2.0

## Expected Results

### Baseline (β=0.3, leverage_ratio=2.0)

From 200-year simulation (100-year burn-in, analyzing years 101-200):
- **Loss ratio mean**: 0.995 (≈ 1.0 ✓)
- **Loss ratio std dev**: 0.005
- **Cycles**: Detected ✓
- **Cycle period (peak-to-peak)**: ~3.5 years
- **Cycle period (spectral)**: **5.0 years** ✅ (paper: 5.9 years)
- **AR(2) coefficients**: a₁ = +0.086, a₂ = -0.442
- **Cycle conditions met**: ✅ (a₁ > 0, -1 < a₂ < 0, a₁² + 4a₂ < 0)
- **All insurers solvent**: 20/20 ✓

### β Sensitivity

| β | Outcome |
|---|---------|
| 0.2 | Stable cycles, high autocorrelation |
| 0.3 | Medium cycles (baseline) |
| 0.6 | High volatility, weaker cycles |
| 1.0 | White noise, cycles disappear |

## Implementation vs. Paper

### Validation Summary

This implementation has been validated against Owadally et al. (2018) through:
- **100-year burn-in period** (as specified in paper)
- **Leverage ratio calibration sweep** (tested {2.0, 2.5, 3.0, 3.5, 4.0})
- **Spectral analysis** for accurate cycle period detection

### Matches ✅

✅ **Cycle period**: 5.0 years (spectral) vs. paper's 5.9 years - **Close match!**
✅ **Endogenous cycles emerge** (no external shocks)
✅ **Loss ratios stationary around 1.0** (0.995 in simulation)
✅ **Cycle conditions met**: a₁ > 0, -1 < a₂ < 0, a₁² + 4a₂ < 0
✅ **β controls cycle stability** (validated across parameter ranges)
✅ **Two-stage pricing** (actuarial + underwriter)
✅ **Credibility blending** and EWMA claim tracking
✅ **All insurers remain solvent** (20/20)

### Differences ⚠️

⚠️ **AR(2) coefficient a₁**: 0.086 vs. paper's 0.467
   - Positive feedback present but weaker than paper
   - Cycles emerge but with muted amplification mechanism
   - Likely causes: allocation noise, simplified customer behavior, market clearing details

⚠️ **Cycle amplitude**: Moderate (std dev = 0.005) vs. paper's higher volatility
   - Related to weaker a₁ coefficient
   - Less dramatic boom-bust dynamics

### Leverage Ratio Calibration Results

Parameter sweep testing leverage_ratio ∈ {2.0, 2.5, 3.0, 3.5, 4.0}:

| Value | Spectral Period | a₁ (AR2) | Conditions Met | Recommendation |
|-------|----------------|----------|----------------|----------------|
| 2.0 ⭐ | 5.0 years     | +0.086   | ✅ YES         | **Best choice** |
| 2.5   | 5.0 years      | -0.016   | ❌ NO          | Negative feedback |
| 3.0   | 5.0 years      | +0.015   | ✅ YES         | Weak feedback |
| 3.5   | 5.0 years      | +0.076   | ✅ YES         | Good alternative |
| 4.0   | 5.0 years      | -0.029   | ❌ NO          | Negative feedback |

**Key finding**: Spectral period remains stable at 5.0 years across all values, validating core cycle mechanism. Value 2.0 provides strongest positive feedback while maintaining stability.

### Implementation Improvements Applied

1. ✅ **100-year burn-in period** - Paper explicitly requires discarding first 100 years
2. ✅ **Allocation noise fixed** - Now applies ±5% noise to total cost (price + distance) instead of just price
3. ✅ **Capacity constraints enforced** - leverage_ratio = 2.0 calibrated through parameter sweep
4. ✅ **Steady-state analysis** - Separate reporting for transient vs. equilibrium behavior

### Remaining Investigations

To understand the weaker a₁ coefficient (0.086 vs 0.467):

1. **Market clearing algorithm details** - Paper's "random allocation" may differ from implementation
2. **Customer behavior modeling** - Simplified vs. stochastic switching
3. **Timing of feedback loops** - Annual vs. continuous adjustment dynamics
4. **Initial conditions sensitivity** - Different equilibria possible

## Testing

### Unit Tests

```bash
cargo test -p insurance_cycles
```

**Coverage**: 42 tests
- Circular distance (helpers)
- Insurer pricing logic
- Market coordinator allocation
- Claim generator statistics
- Stats-based observable interfaces

### Stats-Based Testing Philosophy

Following the **event sourcing paradigm**:
- **Events**: Source of truth (what happened)
- **Agent state**: Private implementation detail
- **Stats**: Public observable interface

```rust
// Good: Test through Stats
let stats = insurer.stats();
assert!(stats.is_solvent());
assert_eq!(stats.num_customers, 50);

// Bad: Test internal state (not exposed)
// assert_eq!(insurer.capital, 10000.0);  // Won't compile
```

This enables:
- Refactoring safety (internal changes don't break tests)
- Observable behavior validation (matches paper descriptions)
- Event log replay and debugging

## File Structure

```
insurance_cycles/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Event enum, Stats structs, ModelConfig, Customer
│   ├── helpers.rs          # circular_distance()
│   ├── insurer.rs          # Insurer agent (two-stage pricing)
│   ├── market_coordinator.rs  # MarketCoordinator agent
│   ├── claim_generator.rs  # ClaimGenerator agent
│   └── main.rs             # EventLoop setup, run simulation
```

## Key Insights

### Why Cycles Emerge

1. **Profit-maximizing underwriters** → increase markup when demand is elastic
2. **Higher markups** → higher prices → fewer customers → elasticity decreases
3. **Lower elasticity** → optimal markup decreases → prices fall
4. **Falling prices** → more customers → elasticity increases → **cycle repeats**

This creates a **delayed negative feedback loop** (period ~5.9 years), mediated by:
- **β (smoothing)**: Dampens response → longer cycles
- **z (credibility)**: Blends own vs industry → synchronization
- **γ (distance cost)**: Affects price sensitivity → elasticity magnitude

### Role of β (Critical Parameter)

β controls how quickly underwriters adjust markup to price elasticity changes:

- **β = 0 (no update)**: Markup frozen at m₀ = 0, no cycles
- **β small (0.2)**: Slow adjustment → stable, long cycles → high autocorrelation
- **β medium (0.3)**: Moderate cycles (baseline)
- **β large (0.6)**: Fast adjustment → volatile, chaotic
- **β = 1.0 (full update)**: No memory → white noise, cycles disappear

**Goldilocks zone**: β ≈ 0.2-0.4 for stable, observable cycles.

## Performance

**Scale**: 1000 customers × 20 insurers = 20,000 cost evaluations/year

**Expected Performance**: ~5ms/year → 100 years in ~0.5 seconds

**Optimizations**:
- HashMap for O(1) price lookups
- Pre-allocated customer/insurer positions
- Bounded history (last 2 years only)
- Batch event scheduling

## Research Context

This implementation demonstrates **complex emergent behavior from simple rules**:

- **Micro**: Individual insurers follow textbook pricing (credibility + markup)
- **Macro**: Industry exhibits cyclical dynamics (5-7 year periods)
- **Mechanism**: Feedback between pricing and market allocation

**Implications**:
- Insurance cycles are **endogenous** (not just from external shocks)
- **Behavioral** factors (underwriter smoothing) critical for cycle dynamics
- **Regulatory** interventions should consider feedback loops

## References

Owadally, I., Zhou, F., & Wright, D. (2018). The insurance industry as a complex social system: Competition, cycles and crises. *Journal of Artificial Societies and Social Simulation*, 21(4), 2.

## License

MIT (matching parent repository)
