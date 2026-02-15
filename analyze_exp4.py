#!/usr/bin/env python3
"""
Experiment 4 Analysis: Lead-Follow Syndication Stability

Compares independent syndicates (follow_top_k=0) vs syndicated (follow_top_k=5)

Success Criteria:
1. Syndication reduces insolvencies via risk-sharing
2. Syndicated has lower loss ratio variance
3. Syndicated has more stable capital evolution
"""

import pandas as pd
import numpy as np
from scipy import stats
import matplotlib.pyplot as plt

print("="*60)
print("EXPERIMENT 4: LEAD-FOLLOW SYNDICATION ANALYSIS")
print("="*60)
print()

# Load data
independent_data = []
syndicated_data = []

print("Loading data...")
for rep in range(10):
    df_ind = pd.read_csv(f'lloyds_insurance/exp4_independent_rep{rep}_time_series.csv')
    independent_data.append(df_ind)

    df_syn = pd.read_csv(f'lloyds_insurance/exp4_syndicated_rep{rep}_time_series.csv')
    syndicated_data.append(df_syn)

print(f"Loaded {len(independent_data)} replications for Independent")
print(f"Loaded {len(syndicated_data)} replications for Syndicated")
print()

# Analysis 1: Final Insolvencies
print("="*60)
print("ANALYSIS 1: FINAL INSOLVENCIES (Year 49)")
print("="*60)

ind_final_insolvent = [df.iloc[-1]['num_insolvent_syndicates'] for df in independent_data]
syn_final_insolvent = [df.iloc[-1]['num_insolvent_syndicates'] for df in syndicated_data]

print(f"\nIndependent Syndicates (no followers):")
print(f"  Mean insolvent: {np.mean(ind_final_insolvent):.2f}")
print(f"  Std dev: {np.std(ind_final_insolvent):.2f}")
print(f"  Range: {min(ind_final_insolvent):.0f} - {max(ind_final_insolvent):.0f}")

print(f"\nSyndicated (follow_top_k=5):")
print(f"  Mean insolvent: {np.mean(syn_final_insolvent):.2f}")
print(f"  Std dev: {np.std(syn_final_insolvent):.2f}")
print(f"  Range: {min(syn_final_insolvent):.0f} - {max(syn_final_insolvent):.0f}")

t_stat, p_value = stats.ttest_ind(ind_final_insolvent, syn_final_insolvent)
improvement = (np.mean(ind_final_insolvent) - np.mean(syn_final_insolvent)) / np.mean(ind_final_insolvent) * 100

print(f"\nStatistical Test:")
print(f"  t-statistic: {t_stat:.4f}")
print(f"  p-value: {p_value:.4f}")
print(f"  Improvement: {improvement:.1f}%")
print(f"  Significant: {'YES' if p_value < 0.05 else 'NO'}")

# Analysis 2: Loss Ratio Variance
print()
print("="*60)
print("ANALYSIS 2: LOSS RATIO VARIANCE")
print("="*60)

# Calculate variance of loss ratios over time for each replication
ind_lr_variances = []
syn_lr_variances = []

for df in independent_data:
    # Exclude warmup period (first 5 years)
    lr_series = df[df['year'] >= 5]['avg_loss_ratio']
    ind_lr_variances.append(np.var(lr_series))

for df in syndicated_data:
    lr_series = df[df['year'] >= 5]['avg_loss_ratio']
    syn_lr_variances.append(np.var(lr_series))

print(f"\nIndependent - Loss Ratio Variance:")
print(f"  Mean variance: {np.mean(ind_lr_variances):.6f}")
print(f"  Std dev: {np.std(ind_lr_variances):.6f}")

print(f"\nSyndicated - Loss Ratio Variance:")
print(f"  Mean variance: {np.mean(syn_lr_variances):.6f}")
print(f"  Std dev: {np.std(syn_lr_variances):.6f}")

t_stat_var, p_value_var = stats.ttest_ind(ind_lr_variances, syn_lr_variances)
variance_reduction = (np.mean(ind_lr_variances) - np.mean(syn_lr_variances)) / np.mean(ind_lr_variances) * 100

print(f"\nStatistical Test:")
print(f"  t-statistic: {t_stat_var:.4f}")
print(f"  p-value: {p_value_var:.4f}")
print(f"  Variance reduction: {variance_reduction:.1f}%")
print(f"  Syndication reduces variance: {'YES' if variance_reduction > 0 else 'NO'}")

# Analysis 3: Capital Stability
print()
print("="*60)
print("ANALYSIS 3: CAPITAL STABILITY")
print("="*60)

# Calculate coefficient of variation (CV) of total capital over time
ind_capital_cv = []
syn_capital_cv = []

for df in independent_data:
    capital_series = df[df['year'] >= 5]['total_capital']
    cv = np.std(capital_series) / np.mean(capital_series)
    ind_capital_cv.append(cv)

for df in syndicated_data:
    capital_series = df[df['year'] >= 5]['total_capital']
    cv = np.std(capital_series) / np.mean(capital_series)
    syn_capital_cv.append(cv)

print(f"\nIndependent - Capital CV:")
print(f"  Mean CV: {np.mean(ind_capital_cv):.4f}")
print(f"  Std dev: {np.std(ind_capital_cv):.4f}")

print(f"\nSyndicated - Capital CV:")
print(f"  Mean CV: {np.mean(syn_capital_cv):.4f}")
print(f"  Std dev: {np.std(syn_capital_cv):.4f}")

t_stat_cv, p_value_cv = stats.ttest_ind(ind_capital_cv, syn_capital_cv)
stability_improvement = (np.mean(ind_capital_cv) - np.mean(syn_capital_cv)) / np.mean(ind_capital_cv) * 100

print(f"\nStatistical Test:")
print(f"  t-statistic: {t_stat_cv:.4f}")
print(f"  p-value: {p_value_cv:.4f}")
print(f"  Stability improvement: {stability_improvement:.1f}%")
print(f"  Syndication improves stability: {'YES' if stability_improvement > 0 else 'NO'}")

# Analysis 4: Time Series Evolution
print()
print("="*60)
print("ANALYSIS 4: TIME SERIES EVOLUTION")
print("="*60)

ind_mean = pd.concat(independent_data).groupby('year').mean()
syn_mean = pd.concat(syndicated_data).groupby('year').mean()

print("\nInsolvencies over time (every 10 years):")
print("Year  Independent  Syndicated  Difference")
print("-" * 50)
for year in [0, 10, 20, 30, 40, 49]:
    ind = ind_mean.loc[year, 'num_insolvent_syndicates']
    syn = syn_mean.loc[year, 'num_insolvent_syndicates']
    diff = ind - syn
    print(f"{year:4d}  {ind:11.2f}  {syn:10.2f}  {diff:+10.2f}")

# Success Criteria
print()
print("="*60)
print("SUCCESS CRITERIA EVALUATION")
print("="*60)

criteria = []

# Criterion 1: Reduces insolvencies
reduces_insol = improvement > 0 and p_value < 0.05
criteria.append(("Syndication reduces insolvencies (significant)",
                 reduces_insol, f"{improvement:.1f}%, p={p_value:.4f}"))

# Criterion 2: Reduces variance
reduces_var = variance_reduction > 0
criteria.append(("Syndication reduces loss ratio variance",
                 reduces_var, f"{variance_reduction:.1f}%"))

# Criterion 3: Improves stability
improves_stability = stability_improvement > 0
criteria.append(("Syndication improves capital stability",
                 improves_stability, f"{stability_improvement:.1f}%"))

print()
for i, (criterion, passed, detail) in enumerate(criteria, 1):
    status = "✅ PASS" if passed else "❌ FAIL"
    print(f"{i}. {criterion}")
    print(f"   {status} - {detail}")
    print()

passed_count = sum(c[1] for c in criteria)
print(f"Overall: {passed_count}/{len(criteria)} criteria passed")
print()

# Key Finding
print("="*60)
print("KEY FINDING")
print("="*60)
print()
print("Lead-Follow Syndication:")
print(f"  Insolvencies: {improvement:+.1f}% ({'significant' if p_value < 0.05 else 'not significant'})")
print(f"  Loss ratio variance: {variance_reduction:+.1f}%")
print(f"  Capital stability: {stability_improvement:+.1f}%")
print()

if reduces_insol:
    print("✅ Syndication SIGNIFICANTLY improves market stability")
elif improvement > 0:
    print("⚠️  Syndication shows marginal improvement (not statistically significant)")
else:
    print("❌ Syndication does NOT improve stability")
print()

# Visualization
print("Generating visualization...")
fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('Experiment 4: Lead-Follow Syndication Analysis', fontsize=16)

# Plot 1: Insolvencies over time
ax1 = axes[0, 0]
ax1.plot(ind_mean.index, ind_mean['num_insolvent_syndicates'],
         label='Independent', linewidth=2, color='red')
ax1.plot(syn_mean.index, syn_mean['num_insolvent_syndicates'],
         label='Syndicated', linewidth=2, color='blue')
ax1.set_xlabel('Year')
ax1.set_ylabel('Number of Insolvent Syndicates')
ax1.set_title('Insolvencies Over Time')
ax1.legend()
ax1.grid(True, alpha=0.3)

# Plot 2: Total capital over time
ax2 = axes[0, 1]
ax2.plot(ind_mean.index, ind_mean['total_capital'],
         label='Independent', linewidth=2, color='red')
ax2.plot(syn_mean.index, syn_mean['total_capital'],
         label='Syndicated', linewidth=2, color='blue')
ax2.set_xlabel('Year')
ax2.set_ylabel('Total Market Capital')
ax2.set_title('Capital Evolution')
ax2.legend()
ax2.grid(True, alpha=0.3)

# Plot 3: Final insolvencies distribution
ax3 = axes[1, 0]
x_pos = np.arange(2)
means = [np.mean(ind_final_insolvent), np.mean(syn_final_insolvent)]
stds = [np.std(ind_final_insolvent), np.std(syn_final_insolvent)]
ax3.bar(x_pos, means, yerr=stds, capsize=5, color=['red', 'blue'], alpha=0.7)
ax3.set_xticks(x_pos)
ax3.set_xticklabels(['Independent', 'Syndicated'])
ax3.set_ylabel('Insolvent Syndicates (Year 49)')
ax3.set_title(f'Final Insolvencies (p={p_value:.4f})')
ax3.grid(True, alpha=0.3, axis='y')

# Plot 4: Loss ratio variance
ax4 = axes[1, 1]
means_var = [np.mean(ind_lr_variances), np.mean(syn_lr_variances)]
stds_var = [np.std(ind_lr_variances), np.std(syn_lr_variances)]
ax4.bar(x_pos, means_var, yerr=stds_var, capsize=5, color=['red', 'blue'], alpha=0.7)
ax4.set_xticks(x_pos)
ax4.set_xticklabels(['Independent', 'Syndicated'])
ax4.set_ylabel('Loss Ratio Variance')
ax4.set_title(f'Loss Ratio Stability (p={p_value_var:.4f})')
ax4.grid(True, alpha=0.3, axis='y')

plt.tight_layout()
plt.savefig('exp4_analysis.png', dpi=300, bbox_inches='tight')
print("Saved: exp4_analysis.png")
print()

print("="*60)
print("ANALYSIS COMPLETE")
print("="*60)
