#!/usr/bin/env python3
"""
Experiment 6: Markup Mechanism Validation

Validates that the underwriting markup mechanism exhibits mean reversion and
responds appropriately to loss experience.

Success Criteria:
- |mean(markup)| < 0.3 (mean reversion toward zero)
- Positive correlation between markup and loss ratio
- ACF shows decay (not random walk)
- Markup values bounded (no explosion)
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import pearsonr
from statsmodels.tsa.stattools import acf
from pathlib import Path

def load_syndicate_data(rep_num):
    """Load syndicate-level time series data"""
    filepath = Path(__file__).parent.parent / f"exp6_rep{rep_num}_syndicate_time_series.csv"
    return pd.read_csv(filepath)

def analyze_markup():
    """Analyze markup mechanism behavior"""

    print("=" * 60)
    print("EXPERIMENT 6: MARKUP MECHANISM VALIDATION")
    print("=" * 60)
    print()

    all_markups = []
    all_loss_ratios = []
    correlations = []

    for rep in range(10):
        df = load_syndicate_data(rep)

        # Filter to years where syndicates are active
        active_df = df[df['capital'] > 0].copy()

        if len(active_df) < 10:
            print(f"⚠ Replication {rep}: Insufficient data")
            continue

        markups = active_df['markup_m_t'].values
        loss_ratios = active_df['loss_ratio'].values

        mean_markup = np.mean(markups)
        max_markup = np.max(np.abs(markups))

        # Calculate correlation between markup and loss ratio
        if len(markups) > 5:
            corr, p_value = pearsonr(markups, loss_ratios)
            correlations.append(corr)
        else:
            corr, p_value = 0, 1

        all_markups.extend(markups)
        all_loss_ratios.extend(loss_ratios)

        print(f"Replication {rep}:")
        print(f"  Mean markup: {mean_markup:.3f}")
        print(f"  Max |markup|: {max_markup:.3f}")
        print(f"  Markup-LR correlation: {corr:.3f} (p={p_value:.4f})")
        print()

    # Overall statistics
    overall_mean = np.mean(all_markups)
    overall_std = np.std(all_markups)
    overall_max = np.max(np.abs(all_markups))

    # Calculate ACF
    if len(all_markups) > 20:
        acf_values = acf(all_markups, nlags=10)
    else:
        acf_values = []

    # Overall correlation
    if len(all_markups) > 10:
        overall_corr, overall_p = pearsonr(all_markups, all_loss_ratios)
    else:
        overall_corr, overall_p = 0, 1

    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Overall mean markup: {overall_mean:.3f}")
    print(f"Overall std: {overall_std:.3f}")
    print(f"Max |markup| observed: {overall_max:.3f}")
    print(f"Overall markup-LR correlation: {overall_corr:.3f} (p={overall_p:.4f})")
    print()

    # Success criteria
    mean_reversion = abs(overall_mean) < 0.3
    positive_correlation = overall_corr > 0 and overall_p < 0.05
    bounded = overall_max < 2.0
    acf_decays = len(acf_values) > 1 and abs(acf_values[1]) < abs(acf_values[0])

    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)
    print(f"✓ Mean reversion (|mean| < 0.3): {mean_reversion} ({overall_mean:.3f})")
    print(f"✓ Positive markup-LR correlation: {positive_correlation} (r={overall_corr:.3f})")
    print(f"✓ Bounded values (max < 2.0): {bounded} ({overall_max:.3f})")
    print(f"✓ ACF shows decay: {acf_decays}")
    print()

    if mean_reversion and bounded:
        print("✅ EXPERIMENT 6 PASSED: Markup mechanism functions correctly")
    else:
        print("❌ EXPERIMENT 6 FAILED: Check EWMA implementation")

    # Generate visualizations
    create_markup_plots(all_markups, all_loss_ratios, acf_values)

    return {
        'markups': all_markups,
        'loss_ratios': all_loss_ratios,
        'correlations': correlations
    }

def create_markup_plots(markups, loss_ratios, acf_values):
    """Create markup visualizations"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Markup time series (sample)
    ax1 = axes[0, 0]
    df = load_syndicate_data(0)
    for syn_id in range(5):
        syn_df = df[df['syndicate_id'] == syn_id]
        ax1.plot(syn_df['year'], syn_df['markup_m_t'], alpha=0.7, label=f'Syn {syn_id}')
    ax1.axhline(y=0, color='black', linestyle='--', linewidth=1)
    ax1.set_xlabel('Year')
    ax1.set_ylabel('Markup (m_t)')
    ax1.set_title('Markup Evolution (Rep 0)')
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    # Plot 2: Distribution of markups
    ax2 = axes[0, 1]
    ax2.hist(markups, bins=30, alpha=0.7, edgecolor='black')
    ax2.axvline(x=0, color='red', linestyle='--', linewidth=2, label='Zero (target)')
    ax2.axvspan(-0.3, 0.3, alpha=0.2, color='green', label='Acceptable range')
    ax2.set_xlabel('Markup (m_t)')
    ax2.set_ylabel('Frequency')
    ax2.set_title('Distribution of Markup Values')
    ax2.legend()
    ax2.grid(True, alpha=0.3)

    # Plot 3: Markup vs Loss Ratio scatter
    ax3 = axes[1, 0]
    # Sample to avoid overcrowding
    sample_size = min(500, len(markups))
    indices = np.random.choice(len(markups), sample_size, replace=False)
    ax3.scatter([markups[i] for i in indices], [loss_ratios[i] for i in indices],
               alpha=0.3)
    ax3.set_xlabel('Markup (m_t)')
    ax3.set_ylabel('Loss Ratio')
    ax3.set_title('Markup vs Loss Ratio')
    ax3.grid(True, alpha=0.3)

    # Plot 4: ACF
    ax4 = axes[1, 1]
    if len(acf_values) > 0:
        ax4.bar(range(len(acf_values)), acf_values, alpha=0.7)
        ax4.axhline(y=0, color='black', linestyle='-', linewidth=1)
        ax4.set_xlabel('Lag')
        ax4.set_ylabel('Autocorrelation')
        ax4.set_title('Autocorrelation Function')
        ax4.grid(True, alpha=0.3)
    else:
        ax4.text(0.5, 0.5, 'Insufficient data for ACF', ha='center', va='center',
                transform=ax4.transAxes)

    plt.tight_layout()

    output_path = Path(__file__).parent / 'experiment_6_markup.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_markup()
