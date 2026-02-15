#!/usr/bin/env python3
"""
Experiment 3: VaR Exposure Management Effectiveness

Compares Scenario 2 (catastrophes without VaR EM) vs Scenario 3 (with VaR EM) to validate
that VaR-based exposure management reduces insolvencies and achieves uniform exposure.

Success Criteria:
- VaR EM reduces insolvencies by ≥1 syndicate on average
- VaR EM achieves lower avg_uniform_deviation (< 0.05)
- VaR EM extends average market lifespan
- Statistical significance (t-test) for insolvency reduction
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import ttest_ind
from pathlib import Path

def load_scenario_data(scenario, rep_num):
    """Load time series data for a scenario/replication"""
    filepath = Path(__file__).parent.parent / f"exp3_scenario{scenario}_rep{rep_num}_time_series.csv"
    return pd.read_csv(filepath)

def analyze_var_effectiveness():
    """Compare scenarios with and without VaR EM"""

    print("=" * 60)
    print("EXPERIMENT 3: VaR EXPOSURE MANAGEMENT EFFECTIVENESS")
    print("=" * 60)
    print()

    scenario2_results = []
    scenario3_results = []

    # Analyze Scenario 2 (no VaR EM)
    print("Scenario 2 (No VaR EM):")
    print("-" * 40)
    for rep in range(10):
        df = load_scenario_data(2, rep)
        final_row = df.iloc[-1]

        active_years = len(df[df['num_solvent_syndicates'] > 0])
        num_insolvent = final_row['num_insolvent_syndicates']

        # Calculate average uniform deviation over active years
        active_df = df[df['num_solvent_syndicates'] > 0]
        avg_uniform_dev = active_df['avg_uniform_deviation'].mean()

        scenario2_results.append({
            'rep': rep,
            'active_years': active_years,
            'final_insolvent': num_insolvent,
            'avg_uniform_deviation': avg_uniform_dev
        })

        print(f"  Rep {rep}: {active_years}/50 years, {num_insolvent} insolvent, "
              f"uniform_dev={avg_uniform_dev:.4f}")

    print()

    # Analyze Scenario 3 (with VaR EM)
    print("Scenario 3 (With VaR EM):")
    print("-" * 40)
    for rep in range(10):
        df = load_scenario_data(3, rep)
        final_row = df.iloc[-1]

        active_years = len(df[df['num_solvent_syndicates'] > 0])
        num_insolvent = final_row['num_insolvent_syndicates']

        active_df = df[df['num_solvent_syndicates'] > 0]
        avg_uniform_dev = active_df['avg_uniform_deviation'].mean()

        scenario3_results.append({
            'rep': rep,
            'active_years': active_years,
            'final_insolvent': num_insolvent,
            'avg_uniform_deviation': avg_uniform_dev
        })

        print(f"  Rep {rep}: {active_years}/50 years, {num_insolvent} insolvent, "
              f"uniform_dev={avg_uniform_dev:.4f}")

    print()

    # Convert to dataframes for analysis
    df2 = pd.DataFrame(scenario2_results)
    df3 = pd.DataFrame(scenario3_results)

    # Summary statistics
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print("Scenario 2 (No VaR EM):")
    print(f"  Avg insolvencies: {df2['final_insolvent'].mean():.2f}")
    print(f"  Avg active years: {df2['active_years'].mean():.1f}")
    print(f"  Avg uniform deviation: {df2['avg_uniform_deviation'].mean():.4f}")
    print()
    print("Scenario 3 (With VaR EM):")
    print(f"  Avg insolvencies: {df3['final_insolvent'].mean():.2f}")
    print(f"  Avg active years: {df3['active_years'].mean():.1f}")
    print(f"  Avg uniform deviation: {df3['avg_uniform_deviation'].mean():.4f}")
    print()

    # Statistical tests
    insolvency_reduction = df2['final_insolvent'].mean() - df3['final_insolvent'].mean()
    t_stat_insolv, p_value_insolv = ttest_ind(df2['final_insolvent'], df3['final_insolvent'])

    t_stat_uniform, p_value_uniform = ttest_ind(
        df2['avg_uniform_deviation'],
        df3['avg_uniform_deviation']
    )

    print("Statistical Tests:")
    print(f"  Insolvency reduction: {insolvency_reduction:.2f} syndicates")
    print(f"  T-test p-value: {p_value_insolv:.4f}")
    print(f"  Uniform deviation improvement: {df2['avg_uniform_deviation'].mean() - df3['avg_uniform_deviation'].mean():.4f}")
    print(f"  T-test p-value: {p_value_uniform:.4f}")
    print()

    # Success criteria
    reduces_insolvency = insolvency_reduction >= 1.0
    improves_uniformity = df3['avg_uniform_deviation'].mean() < 0.05
    extends_lifespan = df3['active_years'].mean() > df2['active_years'].mean()
    statistically_significant = p_value_insolv < 0.1  # One-tailed test

    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)
    print(f"✓ Reduces insolvencies by ≥1: {reduces_insolvency} ({insolvency_reduction:.2f})")
    print(f"✓ Avg uniform deviation < 0.05: {improves_uniformity} ({df3['avg_uniform_deviation'].mean():.4f})")
    print(f"✓ Extends market lifespan: {extends_lifespan}")
    print(f"✓ Statistically significant (p<0.1): {statistically_significant}")
    print()

    if reduces_insolvency and improves_uniformity:
        print("✅ EXPERIMENT 3 PASSED: VaR EM improves market stability")
    else:
        print("❌ EXPERIMENT 3 FAILED: Check VaR EM implementation")

    # Generate visualizations
    create_comparison_plots(df2, df3)

    return df2, df3

def create_comparison_plots(df2, df3):
    """Create visualizations comparing scenarios"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Insolvency comparison
    ax1 = axes[0, 0]
    x = np.arange(2)
    heights = [df2['final_insolvent'].mean(), df3['final_insolvent'].mean()]
    errors = [df2['final_insolvent'].std(), df3['final_insolvent'].std()]
    ax1.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax1.set_xticks(x)
    ax1.set_xticklabels(['Scenario 2\n(No VaR EM)', 'Scenario 3\n(With VaR EM)'])
    ax1.set_ylabel('Average Insolvencies')
    ax1.set_title('Insolvency Reduction with VaR EM')
    ax1.grid(True, alpha=0.3, axis='y')

    # Plot 2: Uniform deviation comparison
    ax2 = axes[0, 1]
    heights = [df2['avg_uniform_deviation'].mean(), df3['avg_uniform_deviation'].mean()]
    errors = [df2['avg_uniform_deviation'].std(), df3['avg_uniform_deviation'].std()]
    ax2.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax2.set_xticks(x)
    ax2.set_xticklabels(['Scenario 2\n(No VaR EM)', 'Scenario 3\n(With VaR EM)'])
    ax2.axhline(y=0.05, color='orange', linestyle='--', label='Target (< 0.05)')
    ax2.set_ylabel('Avg Uniform Deviation')
    ax2.set_title('Exposure Uniformity with VaR EM')
    ax2.legend()
    ax2.grid(True, alpha=0.3, axis='y')

    # Plot 3: Market lifespan comparison
    ax3 = axes[1, 0]
    heights = [df2['active_years'].mean(), df3['active_years'].mean()]
    errors = [df2['active_years'].std(), df3['active_years'].std()]
    ax3.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax3.set_xticks(x)
    ax3.set_xticklabels(['Scenario 2\n(No VaR EM)', 'Scenario 3\n(With VaR EM)'])
    ax3.set_ylabel('Average Active Years')
    ax3.set_title('Market Lifespan Extension')
    ax3.grid(True, alpha=0.3, axis='y')

    # Plot 4: Time series comparison (sample replication)
    ax4 = axes[1, 1]
    df2_sample = load_scenario_data(2, 0)
    df3_sample = load_scenario_data(3, 0)

    ax4.plot(df2_sample['year'], df2_sample['num_solvent_syndicates'],
             label='Scenario 2 (No VaR)', linewidth=2)
    ax4.plot(df3_sample['year'], df3_sample['num_solvent_syndicates'],
             label='Scenario 3 (VaR EM)', linewidth=2)
    ax4.set_xlabel('Year')
    ax4.set_ylabel('Num Solvent Syndicates')
    ax4.set_title('Solvency Over Time (Rep 0)')
    ax4.legend()
    ax4.grid(True, alpha=0.3)

    plt.tight_layout()

    # Save plot
    output_path = Path(__file__).parent / 'experiment_3_var_comparison.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_var_effectiveness()
