#!/usr/bin/env python3
"""
Experiment 5 Analysis: Loss Ratio Equilibrium

Tests all 4 scenarios to verify steady-state loss ratios fluctuate around 1.0

Success Criteria:
1. Mean loss ratio in [0.8, 1.2] for all scenarios
2. t-test vs 1.0 shows p > 0.01 (not significantly different from 1.0)
3. All scenarios converge to equilibrium
"""

import pandas as pd
import numpy as np
from scipy import stats
import matplotlib.pyplot as plt

print("="*60)
print("EXPERIMENT 5: LOSS RATIO EQUILIBRIUM ANALYSIS")
print("="*60)
print()

# Load data for all 4 scenarios
scenarios_data = {f'Scenario {i}': [] for i in range(1, 5)}

print("Loading data...")
for scenario_num in range(1, 5):
    for rep in range(10):
        df = pd.read_csv(f'lloyds_insurance/exp5_scenario{scenario_num}_rep{rep}_time_series.csv')
        scenarios_data[f'Scenario {scenario_num}'].append(df)

for name, data in scenarios_data.items():
    print(f"  {name}: {len(data)} replications")
print()

# Analysis 1: Steady-State Loss Ratios
print("="*60)
print("ANALYSIS 1: STEADY-STATE LOSS RATIOS (Years 30-49)")
print("="*60)

steady_state_results = {}

for name, data in scenarios_data.items():
    # Extract loss ratios from steady-state period (years 30-49)
    loss_ratios = []
    for df in data:
        steady_state = df[df['year'] >= 30]['avg_loss_ratio']
        loss_ratios.extend(steady_state.values)

    mean_lr = np.mean(loss_ratios)
    std_lr = np.std(loss_ratios)

    # t-test against theoretical equilibrium of 1.0
    t_stat, p_value = stats.ttest_1samp(loss_ratios, 1.0)

    steady_state_results[name] = {
        'mean': mean_lr,
        'std': std_lr,
        't_stat': t_stat,
        'p_value': p_value,
        'loss_ratios': loss_ratios
    }

    print(f"\n{name}:")
    print(f"  Mean loss ratio: {mean_lr:.4f}")
    print(f"  Std dev: {std_lr:.4f}")
    print(f"  95% CI: [{mean_lr - 1.96*std_lr/np.sqrt(len(loss_ratios)):.4f}, "
          f"{mean_lr + 1.96*std_lr/np.sqrt(len(loss_ratios)):.4f}]")
    print(f"  t-test vs 1.0: t={t_stat:.4f}, p={p_value:.4f}")
    print(f"  In range [0.8, 1.2]: {'YES' if 0.8 <= mean_lr <= 1.2 else 'NO'}")
    print(f"  Not significantly different from 1.0: {'YES' if p_value > 0.01 else 'NO'}")

# Analysis 2: Convergence Analysis
print()
print("="*60)
print("ANALYSIS 2: CONVERGENCE TO EQUILIBRIUM")
print("="*60)

print("\nLoss ratio evolution (mean across replications):")
print("Year   Scenario 1  Scenario 2  Scenario 3  Scenario 4")
print("-" * 60)

for year in [0, 10, 20, 30, 40, 49]:
    row = f"{year:4d}  "
    for scenario_num in range(1, 5):
        name = f'Scenario {scenario_num}'
        data = scenarios_data[name]
        mean_df = pd.concat(data).groupby('year').mean()
        lr = mean_df.loc[year, 'avg_loss_ratio']
        row += f"{lr:10.4f}  "
    print(row)

# Analysis 3: Variance Comparison
print()
print("="*60)
print("ANALYSIS 3: VARIANCE ACROSS SCENARIOS")
print("="*60)

variances = []
for name in [f'Scenario {i}' for i in range(1, 5)]:
    var = np.var(steady_state_results[name]['loss_ratios'])
    variances.append(var)
    print(f"\n{name}:")
    print(f"  Variance: {var:.6f}")
    print(f"  Coefficient of variation: {np.sqrt(var)/steady_state_results[name]['mean']:.4f}")

# F-test for equality of variances (Levene's test - robust to non-normality)
lr_arrays = [steady_state_results[f'Scenario {i}']['loss_ratios'] for i in range(1, 5)]
f_stat, p_value_levene = stats.levene(*lr_arrays)

print(f"\nLevene's test for equality of variances:")
print(f"  F-statistic: {f_stat:.4f}")
print(f"  p-value: {p_value_levene:.4f}")
print(f"  Variances are equal: {'YES' if p_value_levene > 0.05 else 'NO'}")

# Success Criteria
print()
print("="*60)
print("SUCCESS CRITERIA EVALUATION")
print("="*60)

criteria_results = []

for scenario_num in range(1, 5):
    name = f'Scenario {scenario_num}'
    result = steady_state_results[name]

    # Criterion 1: Mean in [0.8, 1.2]
    in_range = 0.8 <= result['mean'] <= 1.2

    # Criterion 2: Not significantly different from 1.0
    not_sig_diff = result['p_value'] > 0.01

    criteria_results.append((name, in_range, not_sig_diff, result['mean'], result['p_value']))

print()
for name, in_range, not_sig_diff, mean, p_val in criteria_results:
    print(f"{name}:")
    print(f"  Mean in [0.8, 1.2]: {'✅ PASS' if in_range else '❌ FAIL'} (mean={mean:.4f})")
    print(f"  Not sig. diff from 1.0: {'✅ PASS' if not_sig_diff else '❌ FAIL'} (p={p_val:.4f})")
    print()

all_pass = all(in_range and not_sig_diff for _, in_range, not_sig_diff, _, _ in criteria_results)
print(f"Overall: {'✅ ALL SCENARIOS PASS' if all_pass else '❌ SOME SCENARIOS FAIL'}")
print()

# Key Finding
print("="*60)
print("KEY FINDING")
print("="*60)
print()

if all_pass:
    print("✅ Loss ratios converge to equilibrium around 1.0 in ALL scenarios")
    print()
    print("This validates the model's equilibrium behavior:")
    print("  - Premiums adjust to match expected losses")
    print("  - Markup mechanism achieves mean reversion")
    print("  - Market reaches actuarially fair pricing in long run")
else:
    print("⚠️  Some scenarios deviate from equilibrium")
    print()
    for name, in_range, not_sig_diff, mean, p_val in criteria_results:
        if not (in_range and not_sig_diff):
            print(f"  {name}: mean={mean:.4f}, p={p_val:.4f}")

print()

# Visualization
print("Generating visualization...")
fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('Experiment 5: Loss Ratio Equilibrium Analysis', fontsize=16)

# Plot 1: Loss ratio time series for all scenarios
ax1 = axes[0, 0]
colors = ['blue', 'red', 'green', 'purple']
for i, scenario_num in enumerate(range(1, 5)):
    name = f'Scenario {scenario_num}'
    data = scenarios_data[name]
    mean_df = pd.concat(data).groupby('year').mean()
    ax1.plot(mean_df.index, mean_df['avg_loss_ratio'],
             label=name, linewidth=2, color=colors[i])
ax1.axhline(y=1.0, color='black', linestyle='--', linewidth=1, label='Equilibrium (1.0)')
ax1.axhline(y=0.8, color='gray', linestyle=':', linewidth=1, alpha=0.5)
ax1.axhline(y=1.2, color='gray', linestyle=':', linewidth=1, alpha=0.5)
ax1.set_xlabel('Year')
ax1.set_ylabel('Average Loss Ratio')
ax1.set_title('Loss Ratio Evolution')
ax1.legend()
ax1.grid(True, alpha=0.3)

# Plot 2: Steady-state distributions
ax2 = axes[0, 1]
x_pos = np.arange(4)
means = [steady_state_results[f'Scenario {i}']['mean'] for i in range(1, 5)]
stds = [steady_state_results[f'Scenario {i}']['std'] / np.sqrt(200) for i in range(1, 5)]  # SEM
ax2.bar(x_pos, means, yerr=stds, capsize=5, color=colors, alpha=0.7)
ax2.axhline(y=1.0, color='black', linestyle='--', linewidth=2, label='Equilibrium')
ax2.axhline(y=0.8, color='gray', linestyle=':', linewidth=1)
ax2.axhline(y=1.2, color='gray', linestyle=':', linewidth=1)
ax2.set_xticks(x_pos)
ax2.set_xticklabels([f'S{i}' for i in range(1, 5)])
ax2.set_ylabel('Mean Loss Ratio (Years 30-49)')
ax2.set_title('Steady-State Loss Ratios')
ax2.legend()
ax2.grid(True, alpha=0.3, axis='y')

# Plot 3: Histograms of steady-state loss ratios
ax3 = axes[1, 0]
for i, scenario_num in enumerate(range(1, 5)):
    name = f'Scenario {scenario_num}'
    loss_ratios = steady_state_results[name]['loss_ratios']
    ax3.hist(loss_ratios, bins=30, alpha=0.5, label=name, color=colors[i])
ax3.axvline(x=1.0, color='black', linestyle='--', linewidth=2, label='Equilibrium')
ax3.set_xlabel('Loss Ratio')
ax3.set_ylabel('Frequency')
ax3.set_title('Distribution of Steady-State Loss Ratios')
ax3.legend()
ax3.grid(True, alpha=0.3, axis='y')

# Plot 4: Variance comparison
ax4 = axes[1, 1]
variances_plot = [np.var(steady_state_results[f'Scenario {i}']['loss_ratios']) for i in range(1, 5)]
ax4.bar(x_pos, variances_plot, color=colors, alpha=0.7)
ax4.set_xticks(x_pos)
ax4.set_xticklabels([f'S{i}' for i in range(1, 5)])
ax4.set_ylabel('Variance')
ax4.set_title(f'Loss Ratio Variance (Levene p={p_value_levene:.4f})')
ax4.grid(True, alpha=0.3, axis='y')

plt.tight_layout()
plt.savefig('exp5_analysis.png', dpi=300, bbox_inches='tight')
print("Saved: exp5_analysis.png")
print()

print("="*60)
print("ANALYSIS COMPLETE")
print("="*60)
