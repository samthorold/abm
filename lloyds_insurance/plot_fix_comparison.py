#!/usr/bin/env python3
"""
Compare Scenario 3 results before and after pricing stability fixes.
"""

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

# Load data (need to run old version and save as scenario3_old_*.csv first)
# For now, use hardcoded values from PRICING_FIX_RESULTS.md

before_data = {
    'year': [0, 1, 2, 3, 4, 5],
    'premium': [54123, 32249, 90936, 62860, 101545, 109294],
    'loss_ratio': [0.54, 2.16, 0.88, 1.40, 0.93, 1.99],
    'markup': [-0.51, 0.52, 0.30, 0.48, 0.53, 0.68],
    'solvent': [5, 3, 3, 2, 1, 0],
}

# Load current (after fixes) data
after_df = pd.read_csv('scenario3_market_time_series.csv')

fig, axes = plt.subplots(2, 2, figsize=(14, 10))
fig.suptitle('Scenario 3: Impact of Pricing Stability Fixes', fontsize=16, fontweight='bold')

# ============================================================================
# Plot 1: Average Premium Comparison
# ============================================================================
ax = axes[0, 0]
years_before = before_data['year']
years_after = after_df['year'].values

ax.plot(years_before, before_data['premium'], 'o-', color='#d62728',
        linewidth=2, markersize=8, label='Before (unstable)')
ax.plot(years_after, after_df['avg_premium'].values, 's-', color='#2ca02c',
        linewidth=2, markersize=8, label='After (with fixes)')

# Highlight Year 0-1 improvement
ax.axvspan(-0.5, 1.5, alpha=0.1, color='blue')
ax.text(0.5, max(before_data['premium']) * 0.9,
        'Warmup Period\nActive', ha='center', fontsize=10,
        bbox=dict(boxstyle='round', facecolor='lightblue', alpha=0.5))

ax.set_xlabel('Year', fontsize=12)
ax.set_ylabel('Average Premium ($)', fontsize=12)
ax.set_title('Average Premium Over Time', fontsize=13, fontweight='bold')
ax.legend(fontsize=11, loc='upper right')
ax.grid(True, alpha=0.3)
ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, p: f'${x/1000:.0f}k'))

# ============================================================================
# Plot 2: Loss Ratio Comparison
# ============================================================================
ax = axes[0, 1]

ax.plot(years_before, before_data['loss_ratio'], 'o-', color='#d62728',
        linewidth=2, markersize=8, label='Before')
ax.plot(years_after, after_df['avg_loss_ratio'].values, 's-', color='#2ca02c',
        linewidth=2, markersize=8, label='After')

# Break-even line
ax.axhline(y=1.0, color='black', linestyle='--', linewidth=1, alpha=0.5, label='Break-even')

ax.set_xlabel('Year', fontsize=12)
ax.set_ylabel('Loss Ratio', fontsize=12)
ax.set_title('Loss Ratio Over Time', fontsize=13, fontweight='bold')
ax.legend(fontsize=11)
ax.grid(True, alpha=0.3)

# ============================================================================
# Plot 3: Markup (m_t) Comparison
# ============================================================================
ax = axes[1, 0]

ax.plot(years_before, before_data['markup'], 'o-', color='#d62728',
        linewidth=2, markersize=8, label='Before (wild swings)')
ax.plot(years_after, after_df['markup_avg'].values, 's-', color='#2ca02c',
        linewidth=2, markersize=8, label='After (stable)')

# Fair pricing line
ax.axhline(y=0.0, color='black', linestyle='--', linewidth=1, alpha=0.5, label='Actuarially fair')

# Highlight extreme Year 0 markup before fix
ax.annotate('Extreme\nnegative\nmarkup!',
            xy=(0, before_data['markup'][0]),
            xytext=(-0.5, -0.3),
            arrowprops=dict(arrowstyle='->', color='red', lw=2),
            fontsize=10, color='red', fontweight='bold')

# Highlight improved Year 0 markup after fix
ax.annotate('Muted by\nwarmup',
            xy=(0, after_df['markup_avg'].values[0]),
            xytext=(0.5, 0.1),
            arrowprops=dict(arrowstyle='->', color='green', lw=2),
            fontsize=10, color='green', fontweight='bold')

ax.set_xlabel('Year', fontsize=12)
ax.set_ylabel('Markup (m_t)', fontsize=12)
ax.set_title('Underwriting Markup Over Time', fontsize=13, fontweight='bold')
ax.legend(fontsize=11)
ax.grid(True, alpha=0.3)

# ============================================================================
# Plot 4: Solvent Syndicates Comparison
# ============================================================================
ax = axes[1, 1]

ax.plot(years_before, before_data['solvent'], 'o-', color='#d62728',
        linewidth=2, markersize=8, label='Before')
ax.plot(years_after, after_df['num_solvent_syndicates'].values, 's-', color='#2ca02c',
        linewidth=2, markersize=8, label='After')

ax.fill_between(years_before, 0, before_data['solvent'], alpha=0.2, color='#d62728')
ax.fill_between(years_after, 0, after_df['num_solvent_syndicates'].values,
                alpha=0.2, color='#2ca02c')

ax.set_xlabel('Year', fontsize=12)
ax.set_ylabel('Number of Solvent Syndicates', fontsize=12)
ax.set_title('Market Survival Over Time', fontsize=13, fontweight='bold')
ax.set_ylim(-0.5, 5.5)
ax.set_yticks([0, 1, 2, 3, 4, 5])
ax.legend(fontsize=11)
ax.grid(True, alpha=0.3)

# Add collapse markers
collapse_year_before = 5
collapse_year_after = 7
ax.axvline(x=collapse_year_before, color='#d62728', linestyle=':', linewidth=2, alpha=0.5)
ax.text(collapse_year_before, 4.5, 'Collapse\n(before)', ha='center',
        fontsize=9, color='#d62728', fontweight='bold')

if collapse_year_after <= max(years_after):
    ax.axvline(x=collapse_year_after, color='#2ca02c', linestyle=':', linewidth=2, alpha=0.5)
    ax.text(collapse_year_after, 4.5, 'Collapse\n(after)', ha='center',
            fontsize=9, color='#2ca02c', fontweight='bold')

plt.tight_layout()
plt.savefig('scenario3_fix_comparison.png', dpi=300, bbox_inches='tight')
print('✓ Saved: scenario3_fix_comparison.png')

# ============================================================================
# Summary Statistics Table
# ============================================================================
print("\n" + "="*70)
print("PRICING FIX IMPACT SUMMARY")
print("="*70)

metrics = [
    ('Year 0 Premium', before_data['premium'][0], after_df['avg_premium'].values[0]),
    ('Year 0 Loss Ratio', before_data['loss_ratio'][0], after_df['avg_loss_ratio'].values[0]),
    ('Year 0 Markup', before_data['markup'][0], after_df['markup_avg'].values[0]),
    ('Year 1 Premium', before_data['premium'][1], after_df['avg_premium'].values[1]),
    ('Year 1 Loss Ratio', before_data['loss_ratio'][1], after_df['avg_loss_ratio'].values[1]),
    ('Year 1 Insolvencies', 5 - before_data['solvent'][1], 5 - after_df['num_solvent_syndicates'].values[1]),
]

for metric, before_val, after_val in metrics:
    pct_change = ((after_val - before_val) / abs(before_val)) * 100
    symbol = '✅' if abs(after_val) < abs(before_val) or (after_val > before_val and 'Premium' in metric) else '⚠️'
    print(f"{metric:25s} | Before: {before_val:8.2f} | After: {after_val:8.2f} | Change: {pct_change:+6.1f}% {symbol}")

print("\n" + "="*70)
print("SURVIVAL METRICS")
print("="*70)
print(f"Total Collapse Year (Before): {collapse_year_before}")
print(f"Total Collapse Year (After):  {collapse_year_after}")
print(f"Survival Extension:            +{collapse_year_after - collapse_year_before} years ({((collapse_year_after - collapse_year_before) / collapse_year_before) * 100:.0f}% improvement)")
print("="*70)
