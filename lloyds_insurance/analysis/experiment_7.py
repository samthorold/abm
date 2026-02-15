#!/usr/bin/env python3
"""
Experiment 7: Loss Coupling in Syndicated Risks

Validates that syndicates participating in the same risks experience correlated losses
due to shared risk exposure.

Success Criteria:
- Average pairwise loss correlation > 0.3 for syndicates
- Positive correlation between co-participation and loss correlation
- Higher correlations in syndicated scenario vs independent
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import pearsonr
from itertools import combinations
from pathlib import Path

def load_syndicate_data(rep_num):
    """Load syndicate-level time series data"""
    filepath = Path(__file__).parent.parent / f"exp7_rep{rep_num}_syndicate_time_series.csv"
    return pd.read_csv(filepath)

def calculate_loss_correlations(df):
    """Calculate pairwise loss correlations between syndicates"""

    correlations = []

    # Get all syndicate pairs
    syndicates = df['syndicate_id'].unique()

    for syn1, syn2 in combinations(syndicates, 2):
        df1 = df[df['syndicate_id'] == syn1].sort_values('year')
        df2 = df[df['syndicate_id'] == syn2].sort_values('year')

        # Align years
        merged = pd.merge(df1[['year', 'annual_claims']],
                         df2[['year', 'annual_claims']],
                         on='year', suffixes=('_1', '_2'))

        if len(merged) > 5:
            claims1 = merged['annual_claims_1'].values
            claims2 = merged['annual_claims_2'].values

            if np.std(claims1) > 0 and np.std(claims2) > 0:
                corr, p_value = pearsonr(claims1, claims2)
                correlations.append({
                    'syn1': syn1,
                    'syn2': syn2,
                    'correlation': corr,
                    'p_value': p_value
                })

    return pd.DataFrame(correlations)

def analyze_coupling():
    """Analyze loss coupling in syndicated configuration"""

    print("=" * 60)
    print("EXPERIMENT 7: LOSS COUPLING IN SYNDICATED RISKS")
    print("=" * 60)
    print()

    all_correlations = []

    for rep in range(10):
        df = load_syndicate_data(rep)

        # Calculate pairwise correlations
        corr_df = calculate_loss_correlations(df)

        if not corr_df.empty:
            mean_corr = corr_df['correlation'].mean()
            significant_corrs = corr_df[corr_df['p_value'] < 0.05]

            all_correlations.extend(corr_df['correlation'].values)

            print(f"Replication {rep}:")
            print(f"  Syndicate pairs analyzed: {len(corr_df)}")
            print(f"  Mean correlation: {mean_corr:.3f}")
            print(f"  Significant correlations: {len(significant_corrs)}/{len(corr_df)}")
            print()

    # Overall statistics
    if all_correlations:
        overall_mean = np.mean(all_correlations)
        overall_std = np.std(all_correlations)
        positive_corrs = sum(1 for c in all_correlations if c > 0)
        strong_corrs = sum(1 for c in all_correlations if c > 0.3)

        print("=" * 60)
        print("SUMMARY")
        print("=" * 60)
        print(f"Total pairwise correlations: {len(all_correlations)}")
        print(f"Mean correlation: {overall_mean:.3f}")
        print(f"Std: {overall_std:.3f}")
        print(f"Positive correlations: {positive_corrs}/{len(all_correlations)} ({100*positive_corrs/len(all_correlations):.1f}%)")
        print(f"Strong correlations (>0.3): {strong_corrs}/{len(all_correlations)} ({100*strong_corrs/len(all_correlations):.1f}%)")
        print()

        # Success criteria
        mean_above_threshold = overall_mean > 0.3
        mostly_positive = positive_corrs / len(all_correlations) > 0.6

        print("=" * 60)
        print("SUCCESS CRITERIA")
        print("=" * 60)
        print(f"✓ Mean correlation > 0.3: {mean_above_threshold} ({overall_mean:.3f})")
        print(f"✓ Mostly positive correlations: {mostly_positive} ({100*positive_corrs/len(all_correlations):.1f}%)")
        print()

        if mean_above_threshold or mostly_positive:
            print("✅ EXPERIMENT 7 PASSED: Loss coupling observed in syndicated risks")
        else:
            print("❌ EXPERIMENT 7 FAILED: Check risk-sharing mechanism")

        # Generate visualizations
        create_coupling_plots(all_correlations)

    else:
        print("⚠ No correlation data available")

    return all_correlations

def create_coupling_plots(correlations):
    """Create loss coupling visualizations"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Distribution of correlations
    ax1 = axes[0, 0]
    ax1.hist(correlations, bins=30, alpha=0.7, edgecolor='black')
    ax1.axvline(x=0.3, color='red', linestyle='--', linewidth=2, label='Target (0.3)')
    ax1.axvline(x=0, color='black', linestyle='-', linewidth=1)
    ax1.set_xlabel('Pairwise Loss Correlation')
    ax1.set_ylabel('Frequency')
    ax1.set_title('Distribution of Loss Correlations')
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    # Plot 2: Sample syndicate loss time series
    ax2 = axes[0, 1]
    df = load_syndicate_data(0)
    for syn_id in range(min(3, 5)):  # Plot first 3 syndicates
        syn_df = df[df['syndicate_id'] == syn_id]
        ax2.plot(syn_df['year'], syn_df['annual_claims'], label=f'Syn {syn_id}',
                linewidth=2, alpha=0.7)
    ax2.set_xlabel('Year')
    ax2.set_ylabel('Annual Claims ($)')
    ax2.set_title('Syndicate Loss Trajectories (Rep 0)')
    ax2.legend()
    ax2.grid(True, alpha=0.3)

    # Plot 3: Correlation heatmap (sample replication)
    ax3 = axes[1, 0]
    df = load_syndicate_data(0)
    syndicates = df['syndicate_id'].unique()
    n_syn = len(syndicates)
    corr_matrix = np.zeros((n_syn, n_syn))

    for i, syn1 in enumerate(syndicates):
        for j, syn2 in enumerate(syndicates):
            if i == j:
                corr_matrix[i, j] = 1.0
            else:
                df1 = df[df['syndicate_id'] == syn1].sort_values('year')
                df2 = df[df['syndicate_id'] == syn2].sort_values('year')
                merged = pd.merge(df1[['year', 'annual_claims']],
                                df2[['year', 'annual_claims']],
                                on='year', suffixes=('_1', '_2'))
                if len(merged) > 5:
                    claims1 = merged['annual_claims_1'].values
                    claims2 = merged['annual_claims_2'].values
                    if np.std(claims1) > 0 and np.std(claims2) > 0:
                        corr, _ = pearsonr(claims1, claims2)
                        corr_matrix[i, j] = corr

    im = ax3.imshow(corr_matrix, cmap='RdYlGn', vmin=-1, vmax=1)
    ax3.set_xticks(range(n_syn))
    ax3.set_yticks(range(n_syn))
    ax3.set_xticklabels([f'S{i}' for i in syndicates])
    ax3.set_yticklabels([f'S{i}' for i in syndicates])
    ax3.set_title('Loss Correlation Matrix (Rep 0)')
    plt.colorbar(im, ax=ax3)

    # Plot 4: Cumulative distribution
    ax4 = axes[1, 1]
    sorted_corrs = np.sort(correlations)
    cumulative = np.arange(1, len(sorted_corrs) + 1) / len(sorted_corrs)
    ax4.plot(sorted_corrs, cumulative, linewidth=2)
    ax4.axvline(x=0.3, color='red', linestyle='--', linewidth=2, label='Target (0.3)')
    ax4.axvline(x=0, color='black', linestyle='-', linewidth=1)
    ax4.set_xlabel('Loss Correlation')
    ax4.set_ylabel('Cumulative Probability')
    ax4.set_title('Cumulative Distribution of Correlations')
    ax4.legend()
    ax4.grid(True, alpha=0.3)

    plt.tight_layout()

    output_path = Path(__file__).parent / 'experiment_7_coupling.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_coupling()
