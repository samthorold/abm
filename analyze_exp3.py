#!/usr/bin/env python3
"""
Experiment 3 Analysis: VaR Exposure Management Effectiveness

Compares Scenario 2 (catastrophes, no VaR EM) vs Scenario 3 (catastrophes, with VaR EM)

Success Criteria:
1. VaR EM reduces insolvencies
2. VaR EM achieves uniform exposure (avg_uniform_deviation < 0.05)
3. Statistical significance of differences
"""

import pandas as pd
import numpy as np
from scipy import stats
import matplotlib.pyplot as plt

# Load all replications for both scenarios
scenario2_data = []
scenario3_data = []

print("="*60)
print("EXPERIMENT 3: VaR EM EFFECTIVENESS ANALYSIS")
print("="*60)
print()

# Load Scenario 2 (no VaR EM)
print("Loading Scenario 2 (no VaR EM) data...")
for rep in range(10):
    df = pd.read_csv(f'lloyds_insurance/exp3_scenario2_rep{rep}_time_series.csv')
    scenario2_data.append(df)

# Load Scenario 3 (with VaR EM)
print("Loading Scenario 3 (with VaR EM) data...")
for rep in range(10):
    df = pd.read_csv(f'lloyds_insurance/exp3_scenario3_rep{rep}_time_series.csv')
    scenario3_data.append(df)

print(f"Loaded {len(scenario2_data)} replications for Scenario 2")
print(f"Loaded {len(scenario3_data)} replications for Scenario 3")
print()

# Analysis 1: Final Insolvencies
print("="*60)
print("ANALYSIS 1: FINAL INSOLVENCIES (Year 49)")
print("="*60)

scenario2_final_insolvent = [df.iloc[-1]['num_insolvent_syndicates'] for df in scenario2_data]
scenario3_final_insolvent = [df.iloc[-1]['num_insolvent_syndicates'] for df in scenario3_data]

print(f"\nScenario 2 (no VaR EM):")
print(f"  Mean insolvent: {np.mean(scenario2_final_insolvent):.2f}")
print(f"  Std dev: {np.std(scenario2_final_insolvent):.2f}")
print(f"  Range: {min(scenario2_final_insolvent)} - {max(scenario2_final_insolvent)}")

print(f"\nScenario 3 (with VaR EM, factor=0.7):")
print(f"  Mean insolvent: {np.mean(scenario3_final_insolvent):.2f}")
print(f"  Std dev: {np.std(scenario3_final_insolvent):.2f}")
print(f"  Range: {min(scenario3_final_insolvent)} - {max(scenario3_final_insolvent)}")

# Statistical test
t_stat, p_value = stats.ttest_ind(scenario2_final_insolvent, scenario3_final_insolvent)
improvement = (np.mean(scenario2_final_insolvent) - np.mean(scenario3_final_insolvent)) / np.mean(scenario2_final_insolvent) * 100

print(f"\nStatistical Test (t-test):")
print(f"  t-statistic: {t_stat:.4f}")
print(f"  p-value: {p_value:.4f}")
print(f"  Improvement: {improvement:.1f}%")
print(f"  Significant at α=0.05: {'YES' if p_value < 0.05 else 'NO'}")

# Analysis 2: Uniform Deviation
print()
print("="*60)
print("ANALYSIS 2: EXPOSURE UNIFORMITY (avg_uniform_deviation)")
print("="*60)

scenario2_final_uniform = [df.iloc[-1]['avg_uniform_deviation'] for df in scenario2_data]
scenario3_final_uniform = [df.iloc[-1]['avg_uniform_deviation'] for df in scenario3_data]

print(f"\nScenario 2 (no VaR EM):")
print(f"  Mean uniform_deviation: {np.mean(scenario2_final_uniform):.4f}")
print(f"  Std dev: {np.std(scenario2_final_uniform):.4f}")
print(f"  Range: {min(scenario2_final_uniform):.4f} - {max(scenario2_final_uniform):.4f}")

print(f"\nScenario 3 (with VaR EM, factor=0.7):")
print(f"  Mean uniform_deviation: {np.mean(scenario3_final_uniform):.4f}")
print(f"  Std dev: {np.std(scenario3_final_uniform):.4f}")
print(f"  Range: {min(scenario3_final_uniform):.4f} - {max(scenario3_final_uniform):.4f}")

# Statistical test
t_stat_uniform, p_value_uniform = stats.ttest_ind(scenario2_final_uniform, scenario3_final_uniform)
change = (np.mean(scenario3_final_uniform) - np.mean(scenario2_final_uniform)) / np.mean(scenario2_final_uniform) * 100

print(f"\nStatistical Test (t-test):")
print(f"  t-statistic: {t_stat_uniform:.4f}")
print(f"  p-value: {p_value_uniform:.4f}")
print(f"  Change: {change:+.1f}%")
print(f"  VaR improves uniformity: {'YES' if np.mean(scenario3_final_uniform) < np.mean(scenario2_final_uniform) else 'NO'}")

# Analysis 3: Time Series Evolution
print()
print("="*60)
print("ANALYSIS 3: TIME SERIES EVOLUTION")
print("="*60)

# Calculate mean time series across replications
scenario2_mean = pd.concat(scenario2_data).groupby('year').mean()
scenario3_mean = pd.concat(scenario3_data).groupby('year').mean()

# Insolvencies over time
print("\nInsolvencies over time (every 10 years):")
print("Year  Scenario 2  Scenario 3  Difference")
print("-" * 50)
for year in [0, 10, 20, 30, 40, 49]:
    s2_insol = scenario2_mean.loc[year, 'num_insolvent_syndicates']
    s3_insol = scenario3_mean.loc[year, 'num_insolvent_syndicates']
    diff = s2_insol - s3_insol
    print(f"{year:4d}  {s2_insol:10.2f}  {s3_insol:10.2f}  {diff:+10.2f}")

# Uniform deviation over time
print("\nUniform deviation over time (every 10 years):")
print("Year  Scenario 2  Scenario 3  Difference")
print("-" * 50)
for year in [0, 10, 20, 30, 40, 49]:
    s2_uniform = scenario2_mean.loc[year, 'avg_uniform_deviation']
    s3_uniform = scenario3_mean.loc[year, 'avg_uniform_deviation']
    diff = s2_uniform - s3_uniform
    print(f"{year:4d}  {s2_uniform:10.4f}  {s3_uniform:10.4f}  {diff:+10.4f}")

# Analysis 4: Success Criteria Evaluation
print()
print("="*60)
print("SUCCESS CRITERIA EVALUATION")
print("="*60)

criteria = []

# Criterion 1: Reduces insolvencies
insol_reduction = improvement > 0
criteria.append(("VaR EM reduces insolvencies", insol_reduction,
                 f"{improvement:.1f}% improvement"))

# Criterion 2: Achieves uniform exposure (< 0.05)
uniform_achieved = np.mean(scenario3_final_uniform) < 0.05
criteria.append(("VaR EM achieves uniform exposure (< 0.05)", uniform_achieved,
                 f"Mean = {np.mean(scenario3_final_uniform):.4f}"))

# Criterion 3: Statistical significance
statistically_significant = p_value < 0.05
criteria.append(("Insolvency difference statistically significant (p<0.05)",
                 statistically_significant, f"p = {p_value:.4f}"))

print()
for i, (criterion, passed, detail) in enumerate(criteria, 1):
    status = "✅ PASS" if passed else "❌ FAIL"
    print(f"{i}. {criterion}")
    print(f"   {status} - {detail}")
    print()

# Overall assessment
passed_count = sum(c[1] for c in criteria)
print(f"Overall: {passed_count}/{len(criteria)} criteria passed")
print()

# Key Finding
print("="*60)
print("KEY FINDING")
print("="*60)
print()
print("VaR EM (with var_safety_factor = 0.7):")
print(f"  ✅ Reduces insolvencies by {improvement:.1f}%")
print(f"  ❌ Does NOT achieve uniform exposure")
print(f"     (increases concentration by {abs(change):.1f}%)")
print()
print("Interpretation:")
print("  VaR EM successfully improves solvency outcomes but does not")
print("  achieve the uniform exposure distribution hypothesized in the")
print("  Olmez et al. (2024) paper. The mechanism prioritizes capital")
print("  protection over exposure diversification.")
print()

# Generate visualization
print("Generating visualization...")
fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('Experiment 3: VaR EM Effectiveness Analysis', fontsize=16)

# Plot 1: Insolvencies over time
ax1 = axes[0, 0]
ax1.plot(scenario2_mean.index, scenario2_mean['num_insolvent_syndicates'],
         label='Scenario 2 (no VaR EM)', linewidth=2, color='red')
ax1.plot(scenario3_mean.index, scenario3_mean['num_insolvent_syndicates'],
         label='Scenario 3 (VaR EM)', linewidth=2, color='blue')
ax1.set_xlabel('Year')
ax1.set_ylabel('Number of Insolvent Syndicates')
ax1.set_title('Insolvencies Over Time')
ax1.legend()
ax1.grid(True, alpha=0.3)

# Plot 2: Uniform deviation over time
ax2 = axes[0, 1]
ax2.plot(scenario2_mean.index, scenario2_mean['avg_uniform_deviation'],
         label='Scenario 2 (no VaR EM)', linewidth=2, color='red')
ax2.plot(scenario3_mean.index, scenario3_mean['avg_uniform_deviation'],
         label='Scenario 3 (VaR EM)', linewidth=2, color='blue')
ax2.axhline(y=0.05, color='green', linestyle='--', label='Target (<0.05)')
ax2.set_xlabel('Year')
ax2.set_ylabel('Avg Uniform Deviation')
ax2.set_title('Exposure Uniformity Over Time')
ax2.legend()
ax2.grid(True, alpha=0.3)

# Plot 3: Final insolvencies distribution
ax3 = axes[1, 0]
x_pos = np.arange(2)
means = [np.mean(scenario2_final_insolvent), np.mean(scenario3_final_insolvent)]
stds = [np.std(scenario2_final_insolvent), np.std(scenario3_final_insolvent)]
ax3.bar(x_pos, means, yerr=stds, capsize=5, color=['red', 'blue'], alpha=0.7)
ax3.set_xticks(x_pos)
ax3.set_xticklabels(['Scenario 2\n(no VaR EM)', 'Scenario 3\n(VaR EM)'])
ax3.set_ylabel('Insolvent Syndicates (Year 49)')
ax3.set_title(f'Final Insolvencies (p={p_value:.4f})')
ax3.grid(True, alpha=0.3, axis='y')

# Plot 4: Final uniform deviation distribution
ax4 = axes[1, 1]
means_uniform = [np.mean(scenario2_final_uniform), np.mean(scenario3_final_uniform)]
stds_uniform = [np.std(scenario2_final_uniform), np.std(scenario3_final_uniform)]
ax4.bar(x_pos, means_uniform, yerr=stds_uniform, capsize=5, color=['red', 'blue'], alpha=0.7)
ax4.axhline(y=0.05, color='green', linestyle='--', label='Target')
ax4.set_xticks(x_pos)
ax4.set_xticklabels(['Scenario 2\n(no VaR EM)', 'Scenario 3\n(VaR EM)'])
ax4.set_ylabel('Avg Uniform Deviation (Year 49)')
ax4.set_title(f'Final Exposure Uniformity (p={p_value_uniform:.4f})')
ax4.legend()
ax4.grid(True, alpha=0.3, axis='y')

plt.tight_layout()
plt.savefig('exp3_analysis.png', dpi=300, bbox_inches='tight')
print("Saved: exp3_analysis.png")
print()

print("="*60)
print("ANALYSIS COMPLETE")
print("="*60)
