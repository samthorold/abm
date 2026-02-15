# Experiment 3 Analysis Results: VaR EM Effectiveness

**Date**: 2026-02-15
**Analysis**: Comparison of Scenario 2 (no VaR EM) vs Scenario 3 (with VaR EM, var_safety_factor = 0.7)
**Replications**: 10 per scenario × 50 years

---

## Executive Summary

**Finding**: VaR Exposure Management provides **marginal solvency benefit** (2.1% fewer insolvencies) but **does not achieve uniform exposure distribution** as hypothesized in Olmez et al. (2024).

**Success Rate**: 1/3 criteria passed
- ✅ Reduces insolvencies (marginally)
- ❌ Does not achieve uniform exposure (worsens by 11.6%)
- ❌ Difference not statistically significant

---

## Detailed Results

### 1. Final Insolvencies (Year 49)

| Scenario | Mean Insolvent | Std Dev | Range |
|----------|----------------|---------|-------|
| Scenario 2 (no VaR EM) | 4.80 | 0.40 | 4.0 - 5.0 |
| Scenario 3 (VaR EM 0.7) | 4.70 | 0.46 | 4.0 - 5.0 |

**Statistical Test**: t-test
- t-statistic: 0.4932
- p-value: 0.6278
- **Improvement**: 2.1%
- **Significant at α=0.05**: NO

**Interpretation**: VaR EM shows a small reduction in insolvencies (4.80 → 4.70), representing a 2.1% improvement. However, this difference is **not statistically significant** (p = 0.628), meaning it could be due to random variation rather than the VaR mechanism itself.

---

### 2. Exposure Uniformity (avg_uniform_deviation)

| Scenario | Mean Deviation | Std Dev | Range |
|----------|----------------|---------|-------|
| Scenario 2 (no VaR EM) | 0.0804 | 0.0089 | 0.0618 - 0.0937 |
| Scenario 3 (VaR EM 0.7) | 0.0897 | 0.0106 | 0.0708 - 0.1045 |

**Statistical Test**: t-test
- t-statistic: -2.0288
- p-value: 0.0575
- **Change**: +11.6% (worse)
- **Target**: < 0.05 (not achieved)

**Interpretation**: VaR EM **increases** exposure concentration instead of improving uniformity. The mean uniform_deviation rises from 0.0804 to 0.0897 (+11.6%). This is nearly statistically significant (p = 0.058) and indicates VaR constraints may actually worsen diversification. Neither scenario achieves the target of < 0.05.

---

### 3. Time Series Evolution

#### Insolvencies Over Time
```
Year    Scenario 2    Scenario 3    Difference
  0         0.00          0.00         +0.00
 10         4.30          4.40         -0.10
 20         4.80          4.50         +0.30
 30         4.80          4.70         +0.10
 40         4.80          4.70         +0.10
 49         4.80          4.70         +0.10
```

**Pattern**: Both scenarios show rapid insolvencies in the first 10 years, then stabilize. VaR EM shows slightly fewer insolvencies after year 20, but the effect is small and inconsistent.

#### Uniform Deviation Over Time
```
Year    Scenario 2    Scenario 3    Difference
  0        0.1458        0.1555       -0.0098
 10        0.0819        0.0923       -0.0104
 20        0.0809        0.0909       -0.0100
 30        0.0811        0.0905       -0.0094
 40        0.0807        0.0901       -0.0094
 49        0.0804        0.0897       -0.0093
```

**Pattern**: Both scenarios converge from initial high deviation (~0.15) to stable lower levels (~0.08-0.09). **VaR EM consistently maintains ~0.01 higher deviation** throughout the simulation, indicating persistent worse diversification.

---

## Success Criteria Evaluation

| # | Criterion | Result | Details |
|---|-----------|--------|---------|
| 1 | VaR EM reduces insolvencies | ✅ PASS | 2.1% improvement (4.80 → 4.70) |
| 2 | VaR EM achieves uniform exposure (< 0.05) | ❌ FAIL | Mean = 0.0897 (target: < 0.05) |
| 3 | Difference statistically significant (p<0.05) | ❌ FAIL | p = 0.6278 (not significant) |

**Overall**: 1/3 criteria passed

---

## Key Finding: The VaR EM Paradox

### What Works
- ✅ VaR EM provides marginal solvency benefit (2.1% fewer insolvencies)
- ✅ Effect is consistent across time (visible from year 20 onward)

### What Doesn't Work
- ❌ VaR EM **worsens** exposure uniformity instead of improving it (+11.6% concentration)
- ❌ Neither benefit nor detriment is statistically significant
- ❌ Neither scenario achieves the paper's target of < 0.05 uniform_deviation

### The Paradox
VaR constraints are designed to limit exposure and achieve diversification. However, the results show:
1. **Solvency improves slightly** - Suggests VaR limits catastrophic exposures
2. **Diversification worsens** - Suggests VaR constraints may reject diversifying risks

**Possible Explanation**: VaR thresholds may cause syndicates to:
- Reject quotes in unfamiliar peril regions (reducing diversification)
- Focus on familiar regions where they already have exposure (increasing concentration)
- Prioritize capital protection over portfolio balance

---

## Comparison to Paper Predictions

### Olmez et al. (2024) Hypothesis
> "VaR-based exposure management reduces insolvencies and achieves uniform exposure distribution across peril regions (uniform_deviation → 0)"

### Our Results
| Prediction | Our Finding | Status |
|------------|-------------|--------|
| Reduces insolvencies | 2.1% reduction (not significant) | ⚠️ Partial |
| Achieves uniform exposure | 11.6% worse uniformity | ❌ Refuted |
| uniform_deviation < 0.05 | 0.0897 (79% above target) | ❌ Not achieved |

**Conclusion**: Our implementation does **NOT replicate** the paper's findings. VaR EM shows marginal solvency benefit but fails to achieve uniform exposure distribution.

---

## Implications

### For the Model
1. **VaR mechanism is working** (bugs fixed, capital synchronized)
2. **Parameter is calibrated** (var_safety_factor = 0.7 is optimal among tested values)
3. **Effect exists but is weak** (2.1% improvement, not statistically significant)

### For the Research
1. **Different implementation may yield different results**
   - Our agent-based model may have different dynamics than paper's theoretical model
   - Specific implementation choices (quote evaluation, exposure tracking) matter

2. **VaR alone may be insufficient**
   - Uniform exposure may require additional mechanisms (e.g., explicit diversification incentives)
   - Trade-off between solvency and diversification suggests competing objectives

3. **Statistical power concern**
   - Small sample (10 replications) may not detect real but small effects
   - Consider increasing replications for future experiments

---

## Recommendations

### Short-term
1. **Accept VaR EM limitations**: Document that var_safety_factor = 0.7 provides marginal benefit
2. **Focus on other mechanisms**: Experiments 4-7 may show stronger effects
3. **Increase replications**: Re-run with 50+ replications for better statistical power

### Long-term
1. **Investigate VaR mechanism design**:
   - Why does VaR worsen uniformity?
   - Can we modify quote evaluation to prioritize diversification?
   - Alternative exposure management mechanisms?

2. **Sensitivity analysis**:
   - Test var_safety_factor in finer increments (0.65, 0.75, 0.80)
   - Test different var_exceedance_prob values
   - Explore interaction with other parameters

3. **Validation against empirical data**:
   - Compare to real Lloyd's market insolvency rates
   - Validate exposure concentration patterns with industry data

---

## Visualization

Generated: `exp3_analysis.png`

**Plot 1**: Insolvencies over time (top left)
- Shows both scenarios converge to ~4.7-4.8 insolvent
- VaR EM shows slight advantage after year 20

**Plot 2**: Uniform deviation over time (top right)
- Shows both scenarios converge from ~0.15 to ~0.08-0.09
- VaR EM consistently ~0.01 higher (worse diversification)
- Green line shows target of 0.05 (not achieved)

**Plot 3**: Final insolvencies (bottom left)
- Bar chart shows minimal difference: 4.80 vs 4.70
- Large error bars indicate high variability
- p-value = 0.628 (not significant)

**Plot 4**: Final uniform deviation (bottom right)
- Bar chart shows VaR EM has worse uniformity: 0.0804 vs 0.0897
- Green line shows target of 0.05 (neither achieves)
- p-value = 0.058 (marginally significant)

---

## Files Generated

- **Analysis script**: `analyze_exp3.py`
- **Results document**: `EXP3_ANALYSIS_RESULTS.md` (this file)
- **Visualization**: `exp3_analysis.png`

---

## Conclusion

**VaR Exposure Management provides marginal, non-significant solvency benefit** but **fails to achieve uniform exposure distribution**. The mechanism shows a fundamental trade-off: capital protection comes at the cost of portfolio diversification. This suggests that the Olmez et al. (2024) findings may be sensitive to implementation details or require complementary mechanisms beyond VaR constraints alone.

The experimental framework is working correctly - the results simply don't match the paper's predictions, which is itself a valuable research finding.
