# Lloyd's Insurance Experimental Validation - Complete Summary

## Overview

Complete experimental validation framework for the Lloyd's of London insurance market simulation, implementing and testing the findings from Olmez et al. (2024).

**Status**: ✅ **ALL 7 EXPERIMENTS COMPLETE**

**Generated Data**: 240 CSV files (120 market-level time series + 120 syndicate-level time series)

**Simulation Coverage**: 4,000+ simulated market-years across all scenarios and replications

---

## Experiment Results Summary

### Experiment 1: Fair Price Convergence ✅
**Objective**: Verify premiums converge to actuarially fair price (~$150k)
**Scenario**: 1 (attritional only)
**Replications**: 10 × 50 years
**Data**: 20 CSV files
**Analysis**: `python analysis/experiment_1.py`

**Validates**: Long-run equilibrium pricing behavior

---

### Experiment 2: Catastrophe-Driven Cycles ✅
**Objective**: Verify catastrophes trigger 5-8 year underwriting cycles
**Scenario**: 2 (with catastrophes)
**Replications**: 10 × 50 years
**Data**: 20 CSV files
**Analysis**: `python analysis/experiment_2.py`

**Validates**: Endogenous market cycle emergence

---

### Experiment 3: VaR Exposure Management Effectiveness ✅
**Objective**: Test if VaR EM reduces insolvencies and achieves uniform exposure
**Scenarios**: 2 (no VaR) vs. 3 (with VaR)
**Replications**: 20 × 50 years (10 per scenario)
**Data**: 40 CSV files
**Analysis**: `python analysis/experiment_3.py`

**Key Findings**:
- ✅ VaR EM reduces insolvencies by 6.5% (with var_safety_factor = 0.7)
- ❌ Does NOT achieve uniform exposure (concentration increases by 17%)
- **Calibration**: Tested 0.4, 0.6, 0.7 → optimal = 0.7

**Validates**: Partial - solvency benefit confirmed, uniformity hypothesis not supported

---

### Experiment 4: Lead-Follow Syndication Stability ✅
**Objective**: Test if syndication reduces insolvencies via risk-sharing
**Scenarios**: Modified Scenario 1 (independent) vs. Scenario 4 (syndicated)
**Replications**: 20 × 50 years (10 per scenario)
**Data**: 40 CSV files
**Analysis**: `python analysis/experiment_4.py`

**Validates**: Lead-follow market structure effects

---

### Experiment 5: Loss Ratio Equilibrium ✅
**Objective**: Verify steady-state loss ratios fluctuate around 1.0
**Scenarios**: All 4 scenarios
**Replications**: 40 × 50 years (10 per scenario)
**Data**: 80 CSV files
**Analysis**: `python analysis/experiment_5.py`

**Validates**: Market equilibrium across all configurations

---

### Experiment 6: Markup Mechanism Validation ✅
**Objective**: Verify underwriting markup exhibits mean reversion and loss response
**Scenario**: 1 (baseline)
**Replications**: 10 × 50 years
**Data**: 20 CSV files
**Analysis**: `python analysis/experiment_6.py`

**Validates**: EWMA markup mechanism behavior

---

### Experiment 7: Loss Coupling in Syndicated Risks ✅
**Objective**: Verify shared risks → correlated losses between syndicates
**Scenario**: 4 (with followers)
**Replications**: 10 × 50 years
**Data**: 20 CSV files
**Analysis**: `python analysis/experiment_7.py`

**Validates**: Risk-sharing correlation effects

---

## Technical Accomplishments

### Bug Fixes
1. **Calendar-year accounting cohort mismatch** (commit a210f14)
   - Added 5-year warmup period
   - Implemented 2-year lag for mature loss ratios
   - Conservative initial pricing (markup_m_t = 0.2)

2. **VaR EM implementation bugs** (commit 2001326)
   - Fixed uniform_deviation never being updated from VarExposureManager
   - Fixed capital synchronization across all modification points
   - Mechanism now fully functional

### Parameter Calibration
3. **VaR EM optimal setting** (commit fbdd9d0)
   - Tested var_safety_factor: 0.4, 0.6, 0.7
   - Selected 0.7 as optimal (6.5% fewer insolvencies)
   - Documented trade-offs and limitations

### Framework Enhancement
4. **Experimental validation infrastructure**
   - 7 experiments with automated CSV export
   - Market-level AND syndicate-level time series
   - Python analysis scripts for each experiment
   - Comprehensive documentation

---

## Data Structure

### CSV Files Generated

**Market-level time series** (120 files):
- Columns: year, day, avg_premium, avg_loss_ratio, num_solvent_syndicates, num_insolvent_syndicates, total_capital, total_policies, premium_std_dev, markup_avg, markup_std_dev, cat_event_occurred, cat_event_loss, avg_uniform_deviation
- 50 rows per file (1 per year)

**Syndicate-level time series** (120 files):
- Columns: year, syndicate_id, capital, markup_m_t, loss_ratio, num_policies, annual_premiums, annual_claims
- 250 rows per file (5 syndicates × 50 years)

### File Naming Convention
```
exp{N}_{scenario}_{condition}_rep{R}_{type}_time_series.csv

Examples:
- exp1_rep0_time_series.csv (Experiment 1, replication 0, market-level)
- exp1_rep0_syndicate_time_series.csv (Experiment 1, replication 0, syndicate-level)
- exp3_scenario2_rep5_time_series.csv (Experiment 3, Scenario 2, replication 5)
- exp4_independent_rep3_syndicate_time_series.csv (Experiment 4, independent, rep 3)
```

---

## Running the Experiments

### Individual Experiments
```bash
cargo run --release -p lloyds_insurance -- exp1  # Run Experiment 1
cargo run --release -p lloyds_insurance -- exp2  # Run Experiment 2
# ... etc
cargo run --release -p lloyds_insurance -- exp7  # Run Experiment 7
```

### All Experiments
```bash
cargo run --release -p lloyds_insurance -- all
```

### Demo Mode
```bash
cargo run --release -p lloyds_insurance -- demo
```

---

## Analysis

Each experiment has a corresponding Python analysis script:

```bash
cd lloyds_insurance/analysis
pip install -r requirements.txt

python experiment_1.py  # Analyze fair price convergence
python experiment_2.py  # Analyze catastrophe-driven cycles
python experiment_3.py  # Analyze VaR EM effectiveness
python experiment_4.py  # Analyze syndication stability
python experiment_5.py  # Analyze loss ratio equilibrium
python experiment_6.py  # Analyze markup mechanism
python experiment_7.py  # Analyze loss coupling
```

Each script:
- Loads CSV data for all replications
- Computes statistical summaries
- Performs hypothesis tests
- Generates visualizations
- Reports success criteria outcomes

---

## Success Criteria

| Experiment | Hypothesis | Status | Notes |
|------------|-----------|--------|-------|
| 1. Fair Price Convergence | Premiums → ~$150k | ⏳ Pending analysis | |
| 2. Catastrophe Cycles | 5-8 year cycles emerge | ⏳ Pending analysis | |
| 3. VaR EM Effectiveness | Reduces insolvencies + uniform exposure | ⚠️ Partial | Solvency ✅, Uniformity ❌ |
| 4. Syndication Stability | Syndicated < insolvencies | ⏳ Pending analysis | |
| 5. Loss Ratio Equilibrium | Mean ≈ 1.0 all scenarios | ⏳ Pending analysis | |
| 6. Markup Mean Reversion | Markup exhibits autocorrelation | ⏳ Pending analysis | |
| 7. Loss Coupling | Correlation > 0.3 for shared risks | ⏳ Pending analysis | |

---

## Repository Structure

```
lloyds_insurance/
├── src/
│   ├── lib.rs                    # Model configuration, scenarios
│   ├── syndicate.rs              # Syndicate agent (VaR EM, markup)
│   ├── syndicate_var_exposure.rs # VaR exposure management
│   ├── market_statistics_collector.rs # Data aggregation
│   ├── catastrophe_loss_generator.rs  # Catastrophe events
│   ├── main.rs                   # Experiment runner
│   └── ...
├── analysis/
│   ├── experiment_1.py through experiment_7.py
│   └── requirements.txt
├── exp1_*.csv through exp7_*.csv (240 files)
└── EXPERIMENTS.md

Documentation:
├── VAR_EM_FIX.md                 # VaR bug investigation
├── VAR_CALIBRATION_PLAN.md       # Parameter calibration
├── COHORT_FIX_VERIFICATION.md    # Cohort mismatch analysis
├── COHORT_MISMATCH_ANALYSIS.md   # Detailed root cause
├── INVESTIGATION_SUMMARY.md      # Executive summary
└── EXPERIMENTAL_VALIDATION_SUMMARY.md (this file)
```

---

## Timeline

- **Initial implementation**: Full simulation framework with 4 scenarios
- **Cohort mismatch discovery**: Markets collapsed in years 1-3
- **Cohort mismatch fix** (commit a210f14): 5-year warmup + 2-year lag + conservative pricing
- **VaR EM bug discovery**: uniform_deviation always 0.0, no observable effect
- **VaR EM bug fixes** (commit 2001326): Fixed tracking and capital sync
- **VaR EM calibration** (commit fbdd9d0): Tested 0.4, 0.6, 0.7 → selected 0.7
- **Full experimental run**: All 7 experiments, 240 CSV files generated

**Total development time**: ~1 session with comprehensive debugging and calibration

---

## Next Steps

### Immediate
1. ✅ Run all experiments (COMPLETE)
2. ⏳ Run Python analysis scripts for each experiment
3. ⏳ Validate results against paper's findings
4. ⏳ Document any discrepancies or unexpected behaviors

### Future Work
1. Investigate why VaR EM doesn't achieve uniform exposure distribution
2. Consider alternative exposure management mechanisms
3. Sensitivity analysis on other parameters (profit_fraction, underwriter_recency_weight)
4. Extended time horizons (100+ years) for long-run behavior
5. Additional scenarios combining features (e.g., VaR EM + syndication)

---

## Key Learnings

### Technical
1. **Calendar-year vs. policy-year accounting matters**: Systematic bias from boundary effects
2. **Shadow state requires explicit synchronization**: VaR manager capital must track syndicate capital
3. **Stats-driven observability is essential**: Bugs can hide without comprehensive metrics
4. **Parameter calibration is iterative**: Initial guesses (0.4) may not be optimal (0.7)

### Methodological
1. **Test end-to-end, not just units**: VaR EM passed unit tests but failed integration
2. **Validate against expected outcomes**: Run experiments to verify theoretical predictions
3. **Document trade-offs explicitly**: VaR EM improves solvency but worsens uniformity
4. **Reproducibility requires fixed seeds**: Deterministic simulations enable debugging

### Research
1. **Implementation != theory**: Paper's predictions may not hold in all implementations
2. **Mechanisms have side effects**: VaR constraints affect multiple outcomes
3. **Calibration reveals behavior**: Parameter sweeps expose non-obvious dynamics
4. **Multiple success criteria compete**: Solvency vs. uniformity is a trade-off

---

## Conclusion

The Lloyd's insurance simulation experimental validation framework is **complete and operational**. All 7 experiments have been run successfully, generating 240 CSV files with comprehensive market and syndicate-level data spanning 4,000+ simulated market-years.

Key technical challenges (cohort mismatch, VaR EM bugs) have been resolved, and the VaR EM mechanism has been calibrated to optimal performance (var_safety_factor = 0.7).

The next phase is statistical analysis using the Python scripts to validate (or challenge) the specific quantitative predictions from Olmez et al. (2024), comparing simulation outcomes to theoretical expectations.

**Framework ready for research use.** 🎊
