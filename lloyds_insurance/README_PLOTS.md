# Plotting Paper Figures

This directory contains scripts to recreate figures from the paper:

**Olmez, Ahmed, Kam, Feng, Tua (2024). "Exploring the Dynamics of the Specialty Insurance Market Using a Novel Discrete Event Simulation Framework: a Lloyd's of London Case Study"**

## Quick Start

### 1. Run a simulation experiment

```bash
# Run Experiment 1 (Scenario 1 - Base case with attritional losses only)
cargo run --release -p lloyds_insurance -- exp1

# Or run a specific scenario experiment
cargo run --release -p lloyds_insurance -- exp2  # With catastrophes
cargo run --release -p lloyds_insurance -- exp3  # VaR exposure management
cargo run --release -p lloyds_insurance -- exp4  # Lead-follow syndication
```

This will generate CSV files like:
- `exp1_rep0_time_series.csv` (market-level data)
- `exp1_rep0_syndicate_time_series.csv` (syndicate-level data)
- ... for each replication (0-9)

### 2. Generate plots

```bash
# Plot Figure 4 (capital and premium) for Scenario 1, Replication 0
python3 plot_paper_figures.py 1 0

# Plot for Scenario 2, Replication 3
python3 plot_paper_figures.py 2 3
```

This creates three PNG files:
- `figure_4a_scenario1_rep0.png` - Syndicate capital over time
- `figure_4b_scenario1_rep0.png` - Premium distribution over time
- `figure_4_combined_scenario1_rep0.png` - Both panels side-by-side

## Figure Descriptions

### Figure 4a: Syndicate Capital Over Time

Shows how syndicate capital evolves over 50 years. Key observations from the paper:
- Some syndicates go bankrupt (capital → 0)
- Capital fluctuates due to premiums collected vs claims paid
- Dividend payments (40% of profits) and catastrophe losses deplete capital

**What to look for:**
- Insolvency events (lines reaching zero)
- Capital volatility (how much lines fluctuate)
- Recovery patterns after major losses

### Figure 4b: Premium Offered Over Time

Box plots showing premium distribution across syndicates at each time point. Key observations:
- Premiums converge toward fair price (~$300k per risk) over time
- Premium variance decreases as market matures
- Catastrophes (Scenario 2) cause premium spikes

**What to look for:**
- Convergence to the green dashed line (fair price)
- Width of boxes (premium variance across syndicates)
- Outliers (syndicates with extreme pricing)

## Paper Scenarios

The paper defines 4 scenarios (see `ModelConfig` in `lib.rs`):

| Scenario | Description | Key Parameters |
|----------|-------------|----------------|
| 1 | Base case | Attritional losses only, no catastrophes |
| 2 | Catastrophes | Adds catastrophe events (λ=0.05/year) |
| 3 | VaR EM | Scenario 2 + VaR-based exposure management |
| 4 | Lead-Follow | Scenario 1 + lead-follow syndication dynamics |

## Expected Results

From the paper's findings:

**Scenario 1 (Base case):**
- Premiums converge to ~$300k fair price
- Some insolvencies expected (2-3 out of 5 syndicates)
- Low premium volatility

**Scenario 2 (Catastrophes):**
- More insolvencies (3-5 out of 5)
- Higher premium volatility
- Premium spikes after catastrophe events

**Scenario 3 (VaR EM):**
- Fewer insolvencies than Scenario 2
- More uniform exposure across peril regions
- Lower uniform deviation metric

**Scenario 4 (Lead-Follow):**
- **Zero insolvencies** (strong claim from paper)
- Tightly coupled loss ratios across syndicates
- Lower premium volatility than Scenario 1

## Dependencies

```bash
pip install pandas matplotlib numpy
```

## Troubleshooting

**Problem:** `FileNotFoundError: exp1_rep0_syndicate_time_series.csv`

**Solution:** Run the simulation first:
```bash
cargo run --release -p lloyds_insurance -- exp1
```

**Problem:** Plots look different from paper

**Explanation:** The simulation is stochastic. Different random seeds produce different trajectories. The paper likely shows representative runs or averages across multiple replications. Try plotting different replications (0-9) to see the range of outcomes.

## Advanced: Aggregating Multiple Replications

To create plots averaging across multiple replications (like the paper likely does), you can modify the plotting script to:

1. Load all 10 replications for a scenario
2. Calculate mean and confidence intervals
3. Plot aggregated results

Example structure:
```python
all_reps = []
for rep in range(10):
    df = load_syndicate_data(scenario=1, replication=rep)
    all_reps.append(df)

combined = pd.concat(all_reps)
mean_capital = combined.groupby(['year', 'syndicate_id'])['capital'].mean()
# ... plot mean with confidence bands
```

## Citation

If you use these plots or the simulation in research, please cite:

```bibtex
@article{olmez2024lloyds,
  title={Exploring the Dynamics of the Specialty Insurance Market Using a Novel Discrete Event Simulation Framework: a Lloyd's of London Case Study},
  author={Olmez, Sedar and Ahmed, Akhil and Kam, Keith and Feng, Zhe and Tua, Alan},
  journal={Journal of Artificial Societies and Social Simulation},
  volume={27},
  number={2},
  pages={7},
  year={2024},
  doi={10.18564/jasss.5401}
}
```
