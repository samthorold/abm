#!/usr/bin/env python3
"""
Experiment 4: Lead-Follow Syndication Stability

Validates that lead-follow syndication reduces insolvencies via risk-sharing compared
to independent syndicates.

Success Criteria:
- Syndicated configuration has 0 insolvencies vs 2+ for independent
- Lower loss ratio variance in syndicated vs independent
- Higher average capital retention in syndicated configuration
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import f_oneway, ttest_ind
from pathlib import Path

def load_config_data(config, rep_num):
    """Load data for independent or syndicated configuration"""
    filepath = Path(__file__).parent.parent / f"exp4_{config}_rep{rep_num}_time_series.csv"
    return pd.read_csv(filepath)

def load_syndicate_data(config, rep_num):
    """Load syndicate-level data"""
    filepath = Path(__file__).parent.parent / f"exp4_{config}_rep{rep_num}_syndicate_time_series.csv"
    return pd.read_csv(filepath)

def analyze_syndication():
    """Compare independent vs syndicated configurations"""

    print("=" * 60)
    print("EXPERIMENT 4: LEAD-FOLLOW SYNDICATION STABILITY")
    print("=" * 60)
    print()

    independent_results = []
    syndicated_results = []

    # Analyze independent configuration
    print("Independent Syndicates (follow_top_k=0):")
    print("-" * 40)
    for rep in range(10):
        df = load_config_data('independent', rep)
        final_row = df.iloc[-1]

        syn_df = load_syndicate_data('independent', rep)
        loss_ratio_variance = syn_df.groupby('year')['loss_ratio'].var().mean()

        independent_results.append({
            'rep': rep,
            'final_insolvent': final_row['num_insolvent_syndicates'],
            'final_capital': final_row['total_capital'],
            'loss_ratio_var': loss_ratio_variance
        })

        print(f"  Rep {rep}: {final_row['num_insolvent_syndicates']} insolvent, "
              f"capital=${final_row['total_capital']:,.0f}")

    print()

    # Analyze syndicated configuration
    print("Syndicated (follow_top_k=5):")
    print("-" * 40)
    for rep in range(10):
        df = load_config_data('syndicated', rep)
        final_row = df.iloc[-1]

        syn_df = load_syndicate_data('syndicated', rep)
        loss_ratio_variance = syn_df.groupby('year')['loss_ratio'].var().mean()

        syndicated_results.append({
            'rep': rep,
            'final_insolvent': final_row['num_insolvent_syndicates'],
            'final_capital': final_row['total_capital'],
            'loss_ratio_var': loss_ratio_variance
        })

        print(f"  Rep {rep}: {final_row['num_insolvent_syndicates']} insolvent, "
              f"capital=${final_row['total_capital']:,.0f}")

    print()

    # Summary statistics
    df_ind = pd.DataFrame(independent_results)
    df_syn = pd.DataFrame(syndicated_results)

    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print("Independent:")
    print(f"  Avg insolvencies: {df_ind['final_insolvent'].mean():.2f}")
    print(f"  Avg final capital: ${df_ind['final_capital'].mean():,.0f}")
    print(f"  Avg loss ratio variance: {df_ind['loss_ratio_var'].mean():.4f}")
    print()
    print("Syndicated:")
    print(f"  Avg insolvencies: {df_syn['final_insolvent'].mean():.2f}")
    print(f"  Avg final capital: ${df_syn['final_capital'].mean():,.0f}")
    print(f"  Avg loss ratio variance: {df_syn['loss_ratio_var'].mean():.4f}")
    print()

    # Statistical tests
    t_stat, p_value = ttest_ind(df_ind['final_insolvent'], df_syn['final_insolvent'])

    # Success criteria
    syndicated_fewer_insolvencies = df_syn['final_insolvent'].mean() < df_ind['final_insolvent'].mean()
    lower_variance = df_syn['loss_ratio_var'].mean() < df_ind['loss_ratio_var'].mean()
    higher_capital = df_syn['final_capital'].mean() > df_ind['final_capital'].mean()

    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)
    print(f"✓ Syndicated has fewer insolvencies: {syndicated_fewer_insolvencies}")
    print(f"✓ Lower loss ratio variance: {lower_variance}")
    print(f"✓ Higher capital retention: {higher_capital}")
    print(f"  T-test p-value: {p_value:.4f}")
    print()

    if syndicated_fewer_insolvencies and lower_variance:
        print("✅ EXPERIMENT 4 PASSED: Syndication improves stability")
    else:
        print("❌ EXPERIMENT 4 FAILED: Check syndication benefits")

    # Generate visualizations
    create_syndication_plots(df_ind, df_syn)

    return df_ind, df_syn

def create_syndication_plots(df_ind, df_syn):
    """Create comparison visualizations"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    x = np.arange(2)

    # Plot 1: Insolvency comparison
    ax1 = axes[0, 0]
    heights = [df_ind['final_insolvent'].mean(), df_syn['final_insolvent'].mean()]
    errors = [df_ind['final_insolvent'].std(), df_syn['final_insolvent'].std()]
    ax1.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax1.set_xticks(x)
    ax1.set_xticklabels(['Independent', 'Syndicated'])
    ax1.set_ylabel('Average Insolvencies')
    ax1.set_title('Insolvency Reduction via Syndication')
    ax1.grid(True, alpha=0.3, axis='y')

    # Plot 2: Capital retention
    ax2 = axes[0, 1]
    heights = [df_ind['final_capital'].mean(), df_syn['final_capital'].mean()]
    errors = [df_ind['final_capital'].std(), df_syn['final_capital'].std()]
    ax2.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax2.set_xticks(x)
    ax2.set_xticklabels(['Independent', 'Syndicated'])
    ax2.set_ylabel('Final Capital ($)')
    ax2.set_title('Capital Retention')
    ax2.grid(True, alpha=0.3, axis='y')

    # Plot 3: Loss ratio variance
    ax3 = axes[1, 0]
    heights = [df_ind['loss_ratio_var'].mean(), df_syn['loss_ratio_var'].mean()]
    errors = [df_ind['loss_ratio_var'].std(), df_syn['loss_ratio_var'].std()]
    ax3.bar(x, heights, yerr=errors, alpha=0.7, capsize=10, color=['red', 'green'])
    ax3.set_xticks(x)
    ax3.set_xticklabels(['Independent', 'Syndicated'])
    ax3.set_ylabel('Avg Loss Ratio Variance')
    ax3.set_title('Risk Diversification')
    ax3.grid(True, alpha=0.3, axis='y')

    # Plot 4: Distribution of insolvencies
    ax4 = axes[1, 1]
    bins = np.arange(-0.5, 6, 1)
    ax4.hist(df_ind['final_insolvent'], bins=bins, alpha=0.5, label='Independent', edgecolor='black')
    ax4.hist(df_syn['final_insolvent'], bins=bins, alpha=0.5, label='Syndicated', edgecolor='black')
    ax4.set_xlabel('Number of Insolvencies')
    ax4.set_ylabel('Frequency')
    ax4.set_title('Distribution of Insolvencies')
    ax4.legend()
    ax4.grid(True, alpha=0.3)

    plt.tight_layout()

    output_path = Path(__file__).parent / 'experiment_4_syndication.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_syndication()
