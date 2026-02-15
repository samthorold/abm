# Experimental Validation Framework

This document describes how to run the 7 validation experiments for the Lloyd's of London insurance market simulation, based on Olmez et al. (2024).

## Overview

The experimental framework validates that the simulation replicates key findings from the research paper:

1. **Fair Price Convergence** - Premiums converge to ~$150k in equilibrium
2. **Catastrophe-Driven Cycles** - Catastrophes trigger 5-8 year premium cycles
3. **VaR Exposure Management** - VaR reduces insolvencies and achieves uniform exposure
4. **Lead-Follow Syndication** - Syndication improves stability via risk-sharing
5. **Loss Ratio Equilibrium** - Loss ratios equilibrate around 1.0
6. **Markup Mechanism** - Underwriting markup exhibits mean reversion
7. **Loss Coupling** - Shared risks create correlated losses

## Quick Start

### 1. Run Simulations

Run all experiments (generates CSV data for all scenarios):

```bash
cargo run --release -p lloyds_insurance -- all
```

Or run individual experiments:

```bash
cargo run --release -p lloyds_insurance -- exp1  # Experiment 1 only
cargo run --release -p lloyds_insurance -- exp2  # Experiment 2 only
# ... etc
```

**Expected runtime:**
- Single experiment: ~2-5 minutes (10 replications × 50 years each)
- All experiments: ~30-45 minutes

### 2. Install Python Dependencies

```bash
cd lloyds_insurance/analysis
pip install -r requirements.txt
```

### 3. Run Analysis Scripts

```bash
python analysis/experiment_1.py  # Analyze Experiment 1
python analysis/experiment_2.py  # Analyze Experiment 2
# ... etc
```

Each script will:
- Load CSV data from simulation outputs
- Perform statistical analyses
- Generate visualizations (PNG files in `analysis/` directory)
- Print validation results with success criteria
- Report PASS/FAIL status

## Experiments in Detail

### Experiment 1: Fair Price Convergence

**Scenario:** Scenario 1 (attritional losses only, no catastrophes)
**Replications:** 10
**Duration:** 50 years each

**Validates:**
- Premiums converge to actuarially fair price (~$150k per lead participation)
- Premium variance decreases over time (market matures)
- Coefficient of variation decreases in later period vs early period

**Success Criteria:**
- At least 7/10 replications have final 20-year average premium within ±20% of $180k (fair price with 20% loading)
- Average coefficient of variation decreases from early to late period

**Files Generated:**
- `exp1_rep{0-9}_time_series.csv` - Market-level time series (10 files)
- `exp1_rep{0-9}_syndicate_time_series.csv` - Per-syndicate time series (10 files)

**Analysis Output:**
- `experiment_1_convergence.png` - Visualization with 4 subplots:
  - Premium evolution over time (all replications)
  - Final 20-year average vs fair price
  - Coefficient of variation comparison (early vs late)
  - Distribution of final premiums

---

### Experiment 2: Catastrophe-Driven Cycles

**Scenario:** Scenario 2 (attritional + catastrophe losses)
**Replications:** 10
**Duration:** 50 years each

**Validates:**
- Catastrophe events drive 5-8 year underwriting cycles
- Catastrophes cause loss ratio spikes (> 1.5)
- Post-catastrophe premium increases average > 1.2x pre-catastrophe levels

**Success Criteria:**
- Catastrophes observed (avg ≥ 1 per replication)
- Post-catastrophe premium spikes average > 1.2x
- At least 3 replications show dominant cycle period in 5-8 year range (spectral analysis)

**Files Generated:**
- `exp2_rep{0-9}_time_series.csv` (10 files)
- `exp2_rep{0-9}_syndicate_time_series.csv` (10 files)

**Analysis Output:**
- `experiment_2_cycles.png` - Visualization with 4 subplots:
  - Premium time series with catastrophe markers
  - Loss ratio spikes in catastrophe years
  - Distribution of post-catastrophe premium spikes
  - Distribution of cycle periods (from spectral analysis)

---

### Experiment 3: VaR Exposure Management Effectiveness

**Scenarios:** Scenario 2 (no VaR EM) vs Scenario 3 (with VaR EM)
**Replications:** 10 per scenario (20 total)
**Duration:** 50 years each

**Validates:**
- VaR-based exposure management reduces insolvencies
- VaR achieves more uniform exposure distribution across peril regions
- VaR extends average market lifespan

**Success Criteria:**
- VaR reduces insolvencies by ≥1 syndicate on average
- VaR achieves avg_uniform_deviation < 0.05
- VaR extends market lifespan (avg active years)
- Statistical significance (t-test p < 0.1)

**Files Generated:**
- `exp3_scenario2_rep{0-9}_time_series.csv` (10 files)
- `exp3_scenario3_rep{0-9}_time_series.csv` (10 files)
- Corresponding syndicate time series files

**Analysis Output:**
- `experiment_3_var_comparison.png` - Visualization with 4 subplots:
  - Insolvency comparison (bar chart with error bars)
  - Uniform deviation comparison
  - Market lifespan extension
  - Solvency over time (sample replication)

---

### Experiment 4: Lead-Follow Syndication Stability

**Configurations:** Independent (follow_top_k=0) vs Syndicated (follow_top_k=5)
**Replications:** 10 per configuration (20 total)
**Duration:** 50 years each

**Validates:**
- Lead-follow syndication reduces insolvencies via risk-sharing
- Syndication reduces loss ratio variance (diversification benefit)
- Syndication improves capital retention

**Success Criteria:**
- Syndicated has fewer insolvencies than independent
- Syndicated has lower loss ratio variance
- Syndicated has higher average final capital

**Files Generated:**
- `exp4_independent_rep{0-9}_time_series.csv` (10 files)
- `exp4_syndicated_rep{0-9}_time_series.csv` (10 files)
- Corresponding syndicate time series files

**Analysis Output:**
- `experiment_4_syndication.png` - Visualization with 4 subplots:
  - Insolvency comparison
  - Capital retention comparison
  - Loss ratio variance comparison
  - Distribution of insolvencies

---

### Experiment 5: Loss Ratio Equilibrium

**Scenarios:** All 4 scenarios
**Replications:** 10 per scenario (40 total)
**Duration:** 50 years each

**Validates:**
- Steady-state loss ratios fluctuate around 1.0 across all scenarios
- Markup mechanism balances premiums and claims
- Pricing mechanism is stable across different configurations

**Success Criteria:**
- All scenarios have mean loss ratio in [0.8, 1.2]
- One-sample t-test against 1.0 shows p > 0.01 (not significantly different)

**Files Generated:**
- `exp5_scenario{1-4}_rep{0-9}_time_series.csv` (40 files)
- Corresponding syndicate time series files

**Analysis Output:**
- `experiment_5_equilibrium.png` - Visualization with 4 subplots:
  - Loss ratio distributions by scenario
  - Mean loss ratios by scenario (bar chart)
  - Time series example (Scenario 1, Rep 0)
  - P-values for each scenario (t-test results)

---

### Experiment 6: Markup Mechanism Validation

**Scenario:** Scenario 1 (attritional only)
**Replications:** 10
**Duration:** 50 years each

**Validates:**
- Underwriting markup exhibits mean reversion toward zero
- Markup responds to loss experience (positive correlation)
- EWMA prevents explosive growth
- Autocorrelation function shows decay (not random walk)

**Success Criteria:**
- |mean(markup)| < 0.3 across all replications
- Positive correlation between markup and loss ratio (significant)
- Markup values bounded (max |markup| < 2.0)
- ACF shows decay pattern

**Files Generated:**
- `exp6_rep{0-9}_time_series.csv` (10 files)
- `exp6_rep{0-9}_syndicate_time_series.csv` (10 files)

**Analysis Output:**
- `experiment_6_markup.png` - Visualization with 4 subplots:
  - Markup evolution over time (all 5 syndicates, Rep 0)
  - Distribution of markup values
  - Markup vs Loss Ratio scatter plot
  - Autocorrelation function (ACF)

---

### Experiment 7: Loss Coupling in Syndicated Risks

**Scenario:** Scenario 4 (with followers)
**Replications:** 10
**Duration:** 50 years each

**Validates:**
- Syndicates sharing risks experience correlated losses
- Co-participation creates systemic dependencies
- Risk-sharing mechanism functions as intended

**Success Criteria:**
- Average pairwise loss correlation > 0.3
- Majority of correlations are positive (> 60%)
- Correlation matrix shows clustering

**Files Generated:**
- `exp7_rep{0-9}_time_series.csv` (10 files)
- `exp7_rep{0-9}_syndicate_time_series.csv` (10 files)

**Analysis Output:**
- `experiment_7_coupling.png` - Visualization with 4 subplots:
  - Distribution of pairwise loss correlations
  - Sample syndicate loss trajectories
  - Correlation heatmap (Rep 0)
  - Cumulative distribution of correlations

---

## Data Files

### Market-Level Time Series

**Filename:** `{experiment}_{scenario}_rep{N}_time_series.csv`

**Columns:**
- `year` - Simulation year (0-49)
- `day` - Simulation day (0-18,249)
- `avg_premium` - Market-wide average premium per policy
- `avg_loss_ratio` - Market-wide average loss ratio (claims/premiums)
- `num_solvent_syndicates` - Number of solvent syndicates
- `num_insolvent_syndicates` - Number of insolvent syndicates
- `total_capital` - Total market capital
- `total_policies` - Total policies written this year
- `premium_std_dev` - Standard deviation of premiums across syndicates
- `markup_avg` - Average underwriting markup across syndicates
- `markup_std_dev` - Standard deviation of markup
- `cat_event_occurred` - Catastrophe occurred this year (0/1)
- `cat_event_loss` - Total catastrophe loss this year
- `avg_uniform_deviation` - Average exposure uniformity deviation

**Rows:** 50 (one per year)

### Syndicate-Level Time Series

**Filename:** `{experiment}_{scenario}_rep{N}_syndicate_time_series.csv`

**Columns:**
- `year` - Simulation year (0-49)
- `syndicate_id` - Syndicate identifier (0-4)
- `capital` - Syndicate capital at year end
- `markup_m_t` - Underwriting markup (m_t)
- `loss_ratio` - Annual loss ratio (claims/premiums)
- `num_policies` - Number of policies written this year
- `annual_premiums` - Total premiums collected this year
- `annual_claims` - Total claims paid this year

**Rows:** 250 (5 syndicates × 50 years)

---

## Interpreting Results

### PASS Criteria

An experiment **PASSES** if:
1. All statistical success criteria are met
2. Results align with Olmez et al. (2024) findings
3. Visualizations show expected patterns

### Expected Outcomes

Based on the paper, expect:

**Experiment 1:** Premiums converge to $150-180k, most replications pass
**Experiment 2:** 2-3 catastrophes per 50 years, clear premium spikes
**Experiment 3:** VaR reduces insolvencies by 1-2 syndicates, uniform deviation < 0.05
**Experiment 4:** Syndication reduces insolvencies to 0-1 vs 2-4 independent
**Experiment 5:** All scenarios show loss ratios around 0.9-1.1
**Experiment 6:** Markup mean near 0, bounded within [-1, 1]
**Experiment 7:** Average correlation 0.3-0.5, mostly positive

### Troubleshooting

**No CSV files generated:**
- Check that simulation completed (`cargo run --release`)
- Verify files are in `lloyds_insurance/` directory

**Python import errors:**
- Install dependencies: `pip install -r requirements.txt`
- Use Python 3.8+

**Empty plots:**
- Ensure simulation ran for full 50 years
- Check that active market years > 20

**Unexpected failures:**
- Verify random seeds match (fixed in code)
- Check configuration parameters in `lib.rs`
- Review simulation console output for errors

---

## Advanced Usage

### Running Custom Scenarios

Modify `main.rs` to create custom scenarios:

```rust
let mut config = ModelConfig::scenario_1();
config.mean_cat_events_per_year = 0.1;  // Higher catastrophe frequency
config.initial_capital = 20_000_000.0;  // Higher initial capital
```

### Batch Processing

Run all experiments in background:

```bash
nohup cargo run --release -p lloyds_insurance -- all > experiments.log 2>&1 &
```

Monitor progress:

```bash
tail -f experiments.log
```

### Parallel Analysis

Run all Python analyses in parallel:

```bash
cd lloyds_insurance/analysis
for i in {1..7}; do
    python experiment_${i}.py &
done
wait
echo "All analyses complete"
```

---

## References

**Olmez, F., Geiger, M., Guerrero, O. A., & Castañón, C.** (2024). *An Agent-Based Model of the Lloyd's of London Insurance Market*. Journal of Economic Dynamics and Control.

**Key Findings Validated:**
1. Fair pricing convergence under attritional losses
2. Catastrophe-driven underwriting cycles
3. VaR-based exposure management effectiveness
4. Benefits of lead-follow syndication structure
5. Loss ratio equilibration via markup mechanism
6. Correlated losses in syndicated risk pools

---

## Support

For questions or issues:
1. Check simulation console output for errors
2. Verify CSV files are correctly formatted
3. Review Python traceback messages
4. Consult `prior-art/olmez-2024-lloyds-insurance.md` for paper details

**Note:** This experimental framework is designed for research validation. Results may vary slightly due to stochastic simulation, but overall patterns should be consistent with the paper's findings.
