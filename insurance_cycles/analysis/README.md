# Insurance Cycles Analysis Suite

Python scripts for analyzing simulation results from the insurance_cycles experimental framework.

## Setup

```bash
# Create virtual environment
python3 -m venv venv

# Activate (Unix/Mac)
source venv/bin/activate

# Activate (Windows)
venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
```

## Scripts Overview

### Core Analysis Scripts

#### 1. `cycle_analysis.py` - Cycle Detection & Statistical Validation

Analyzes cyclical behavior using multiple methods:
- Peak detection
- AR(2) model fitting (Yule-Walker equations)
- Spectral analysis (periodogram)
- Autocorrelation functions (ACF, PACF)

**Usage:**
```bash
python cycle_analysis.py ../results/baseline_validation/
```

**Output:**
- Prints: Cycle detection rate, AR(2) coefficients, dominant frequency
- Generates: `cycle_diagnostics.png` (4-panel plot)

#### 2. `anomaly_detection.py` - Quality Assurance Checks

Validates simulation correctness:
- Shadow state consistency
- Capacity constraint violations
- Insolvency rates
- Loss ratio explosions (>2.0)
- Stationarity tests (Augmented Dickey-Fuller)

**Usage:**
```bash
python anomaly_detection.py ../results/baseline_validation/
```

**Output:**
- Prints: Comprehensive anomaly report
- Generates: `anomaly_report.json`

#### 3. `market_structure.py` - Market Concentration Analysis

Analyzes competitive dynamics:
- Herfindahl-Hirschman Index (HHI) evolution
- Gini coefficient trends
- Market share distribution
- Top firms identification

**Usage:**
```bash
python market_structure.py ../results/baseline_validation/
```

**Output:**
- Prints: Concentration statistics, top 5 firms
- Generates: `market_concentration.png`, `market_share_heatmap.png`, `top_firms.png`
- Data: `hhi_timeseries.csv`, `market_share_evolution.csv`

#### 4. `parameter_sensitivity.py` - Parameter Sweep Analysis

Analyzes parameter sweep experiments:
- Aggregate metrics per parameter value
- Correlation analysis
- Statistical significance testing

**Usage:**
```bash
python parameter_sensitivity.py ../results/beta_sensitivity/
```

**Output:**
- Prints: Parameter sweep summary, correlations, expected relationships
- Generates: `sensitivity_panel.png`, `sensitivity_cycle_period.png`, `sensitivity_volatility.png`
- Data: `sensitivity_data.csv`

### Visualization Scripts

#### 5. `visualization/plot_timeseries.py` - Time Series Plots

Multi-run overlay plots with:
- Faint lines for individual runs
- Bold line for mean
- Confidence bands (±1 std)
- Paper target range overlay
- Annotated peaks/troughs

**Usage:**
```bash
python visualization/plot_timeseries.py ../results/baseline_validation/ [metric]
# Metrics: loss_ratio (default), avg_claim, avg_price
```

**Output:**
- `timeseries_{metric}.png` - Single metric
- `timeseries_panel.png` - 3-panel (loss ratio, claims, prices)

#### 6. `visualization/plot_phase_diagrams.py` - Phase Space Visualization

Creates phase space portraits to visualize cycle dynamics:
- x_t vs x_{t-1} (state space - should show ellipse for stable cycles)
- x_t vs dx/dt (velocity field)
- 3D trajectory (x_t vs x_{t-1} vs x_{t-2})

**Usage:**
```bash
python visualization/plot_phase_diagrams.py ../results/baseline_validation/
```

**Output:**
- `phase_state_space.png` - 2D state space portrait
- `phase_velocity.png` - Velocity field
- `phase_3d.png` - 3D trajectory

#### 7. `visualization/plot_spectral.py` - Spectral Analysis

Power spectral density analysis:
- Multi-run aggregated periodogram
- Dominant frequency detection
- Comparison to paper's 5.9 year target
- Period distribution histogram

**Usage:**
```bash
python visualization/plot_spectral.py ../results/baseline_validation/
```

**Output:**
- Prints: Mean cycle period, comparison to paper
- `spectral_periodogram.png` - Aggregated periodogram
- `spectral_period_histogram.png` - Distribution of detected periods

## Typical Workflow

### 1. Run Baseline Validation

```bash
# From project root
cd insurance_cycles
cargo run --release --bin run_experiment -- experiments/baseline_validation.toml
```

### 2. Comprehensive Analysis

```bash
cd analysis
source venv/bin/activate

# Core analyses
python cycle_analysis.py ../results/baseline_validation/
python anomaly_detection.py ../results/baseline_validation/
python market_structure.py ../results/baseline_validation/

# Visualizations
python visualization/plot_timeseries.py ../results/baseline_validation/
python visualization/plot_phase_diagrams.py ../results/baseline_validation/
python visualization/plot_spectral.py ../results/baseline_validation/
```

### 3. Parameter Sensitivity

```bash
# Run sweep
cd ..
cargo run --release --bin run_experiment -- experiments/beta_sensitivity.toml

# Analyze
cd analysis
python parameter_sensitivity.py ../results/beta_sensitivity/
```

## Expected Results (Baseline β=0.3)

From 30 Monte Carlo runs:

✅ **Cycle detection rate**: ≥95%
✅ **Mean loss ratio**: 0.95 - 1.05
✅ **Cycle period**: 2.5 - 7.0 years (implementation: ~3.1 years vs. paper's 5.9 years)
✅ **AR(2) conditions met**: ≥80% of runs
✅ **Stationarity**: p < 0.05 (ADF test)
✅ **No anomalies**: Shadow state consistent, no capacity violations

## Interpreting Outputs

### Cycle Diagnostics Plot (`cycle_diagnostics.png`)

**Top-left (Time Series)**: Should show oscillations around 1.0
- Red circles = peaks
- Green circles = troughs
- Period = average peak-to-peak distance

**Top-right (ACF)**: Should show damped oscillation
- Significant lag-1 autocorrelation (usually positive)
- Oscillating pattern indicates cyclical behavior

**Bottom-left (PACF)**: Should show spikes at lags 1-2
- Indicates AR(2) process

**Bottom-right (Periodogram)**: Should show clear peak
- Red line = detected dominant frequency
- Green line = paper's target (5.9 years)

### Anomaly Report

**Shadow State**: Coordinator aggregates should match sum of insurers
- Green check = consistent (<3% error)
- Red warning = inconsistent (potential bug)

**Capacity Violations**: Should be 0 if constraints enforced
- Non-zero = implementation issue

**Insolvency Rate**: Should be <20%
- Higher = parameter misconfiguration or implementation issue

### Sensitivity Panel (`sensitivity_panel.png`)

For **β (underwriter_smoothing)**:
- **Top-left**: Cycle detection rate should stay high for β ∈ [0.2, 0.4], drop at β=1.0
- **Top-right**: Cycle period should decrease as β increases (negative correlation)
- **Bottom-left**: Volatility should increase as β increases (positive correlation)
- **Bottom-right**: AR(2) conditions met should be high for low β, drop at high β

## Troubleshooting

**Import errors**: Activate venv and install requirements
```bash
source venv/bin/activate
pip install -r requirements.txt
```

**No data found**: Check that experiment output exists
```bash
ls ../results/baseline_validation/run_*/
```

**Plots not showing**: Use output_path parameter or check display settings
```bash
# Save to file instead of display
python cycle_analysis.py ../results/baseline_validation/ --output figures/
```

**Python version**: Requires Python ≥3.8
```bash
python --version
```

## Dependencies

Core packages (see `requirements.txt`):
- `numpy` - Numerical computing
- `pandas` - Data manipulation
- `scipy` - Scientific computing (stats, signal processing)
- `statsmodels` - Time series analysis (ACF, PACF, ADF test)
- `matplotlib` - Plotting
- `seaborn` - Statistical visualization

## Contributing Analysis Scripts

When adding new scripts:
1. Add shebang: `#!/usr/bin/env python3`
2. Add docstring explaining purpose
3. Accept experiment directory as CLI argument
4. Print summary to stdout
5. Save plots/data to experiment directory
6. Update this README

## References

- **Owadally et al. (2018)**: Insurance industry as complex social system
- **AR(2) cycle conditions**: Section 3.2, Equations 9-11
- **Spectral analysis**: Section 4, Figure 3
