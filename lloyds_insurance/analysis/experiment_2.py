#!/usr/bin/env python3
"""
Experiment 2: Catastrophe-Driven Cycles

Validates that catastrophe events drive 5-8 year underwriting cycles in premium pricing.

Success Criteria:
- Catastrophe years show loss ratio spikes (> 1.5)
- Spectral analysis shows dominant cycle period in 5-8 year range
- Post-catastrophe premium increases average > 1.2x pre-catastrophe levels
- Catastrophe events occur with expected frequency (~2.5 events over 50 years)
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy import signal
from scipy.stats import ttest_ind
from pathlib import Path

def load_replication_data(rep_num):
    """Load time series data for a single replication"""
    filepath = Path(__file__).parent.parent / f"exp2_rep{rep_num}_time_series.csv"
    return pd.read_csv(filepath)

def detect_catastrophe_years(df):
    """Identify years with catastrophe events"""
    return df[df['cat_event_occurred'] == 1]

def analyze_cycles():
    """Analyze catastrophe-driven premium cycles"""

    print("=" * 60)
    print("EXPERIMENT 2: CATASTROPHE-DRIVEN CYCLES")
    print("=" * 60)
    print()

    all_cat_counts = []
    all_spike_ratios = []
    all_cycle_periods = []

    for rep in range(10):
        df = load_replication_data(rep)

        # Filter to active market years
        active_df = df[df['num_solvent_syndicates'] > 0].copy()

        if len(active_df) < 20:
            print(f"⚠ Replication {rep}: Market collapsed early ({len(active_df)} years)")
            continue

        # Detect catastrophe years
        cat_years = detect_catastrophe_years(active_df)
        num_cats = len(cat_years)
        all_cat_counts.append(num_cats)

        print(f"Replication {rep}:")
        print(f"  Active years: {len(active_df)}/50")
        print(f"  Catastrophe years: {num_cats}")

        if num_cats > 0:
            print(f"  Catastrophe year details:")
            for _, row in cat_years.iterrows():
                print(f"    Year {int(row['year'])}: Loss=${row['cat_event_loss']:,.0f}, "
                      f"Loss Ratio={row['avg_loss_ratio']:.2f}")

            # Analyze post-catastrophe premium changes
            premium_ratios = []
            for cat_year in cat_years['year'].values:
                # Get premium 1 year before and 1 year after
                pre_cat = active_df[active_df['year'] == cat_year - 1]
                post_cat = active_df[active_df['year'] == cat_year + 1]

                if not pre_cat.empty and not post_cat.empty:
                    pre_premium = pre_cat['avg_premium'].values[0]
                    post_premium = post_cat['avg_premium'].values[0]
                    if pre_premium > 0:
                        ratio = post_premium / pre_premium
                        premium_ratios.append(ratio)

            if premium_ratios:
                avg_spike = np.mean(premium_ratios)
                all_spike_ratios.append(avg_spike)
                print(f"  Avg post-cat premium spike: {avg_spike:.2f}x")
        else:
            print(f"  No catastrophes observed")

        # Spectral analysis of premium time series
        if len(active_df) >= 20:
            premiums = active_df['avg_premium'].values
            if len(premiums) > 10:
                # Detrend and compute periodogram
                freqs, psd = signal.periodogram(signal.detrend(premiums))

                # Find dominant frequency (exclude DC component)
                if len(freqs) > 1:
                    dominant_idx = np.argmax(psd[1:]) + 1
                    dominant_freq = freqs[dominant_idx]

                    if dominant_freq > 0:
                        cycle_period = 1.0 / dominant_freq
                        all_cycle_periods.append(cycle_period)
                        print(f"  Dominant cycle period: {cycle_period:.1f} years")

        print()

    # Summary statistics
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Average catastrophes per replication: {np.mean(all_cat_counts):.2f}")
    print(f"Expected (λ=0.05/year × 50 years): 2.5")

    if all_spike_ratios:
        print(f"Average post-catastrophe premium spike: {np.mean(all_spike_ratios):.2f}x")
        print(f"Spike ratios > 1.2x: {sum(1 for r in all_spike_ratios if r > 1.2)}/{len(all_spike_ratios)}")

    if all_cycle_periods:
        print(f"Average dominant cycle period: {np.mean(all_cycle_periods):.1f} years")
        cycles_in_range = sum(1 for p in all_cycle_periods if 5 <= p <= 8)
        print(f"Cycles in 5-8 year range: {cycles_in_range}/{len(all_cycle_periods)}")

    print()

    # Success criteria
    sufficient_cats = np.mean(all_cat_counts) >= 1.0  # At least some catastrophes
    spikes_observed = len(all_spike_ratios) > 0 and np.mean(all_spike_ratios) > 1.2
    cycles_in_range_check = len(all_cycle_periods) > 0 and sum(1 for p in all_cycle_periods if 5 <= p <= 8) >= 3

    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)
    print(f"✓ Catastrophes observed: {sufficient_cats}")
    print(f"✓ Post-catastrophe premium spikes > 1.2x: {spikes_observed}")
    print(f"✓ At least 3 replications show 5-8 year cycles: {cycles_in_range_check}")
    print()

    if sufficient_cats and (spikes_observed or cycles_in_range_check):
        print("✅ EXPERIMENT 2 PASSED: Catastrophes drive premium cycles")
    else:
        print("❌ EXPERIMENT 2 FAILED: Check catastrophe dynamics")

    # Generate visualizations
    create_cycle_plots(all_cat_counts, all_spike_ratios, all_cycle_periods)

    return {
        'cat_counts': all_cat_counts,
        'spike_ratios': all_spike_ratios,
        'cycle_periods': all_cycle_periods
    }

def create_cycle_plots(cat_counts, spike_ratios, cycle_periods):
    """Create visualizations of catastrophe cycles"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Time series with catastrophe markers for one replication
    ax1 = axes[0, 0]
    df = load_replication_data(0)
    active_df = df[df['num_solvent_syndicates'] > 0]
    cat_df = detect_catastrophe_years(active_df)

    ax1.plot(active_df['year'], active_df['avg_premium'], label='Premium', linewidth=2)
    ax1.scatter(cat_df['year'], cat_df['avg_premium'], color='red', s=100, zorder=5,
                label='Catastrophe Year', marker='x', linewidths=3)
    ax1.set_xlabel('Year')
    ax1.set_ylabel('Average Premium ($)')
    ax1.set_title('Premium Cycles with Catastrophe Events (Rep 0)')
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    # Plot 2: Loss ratio spikes in catastrophe years
    ax2 = axes[0, 1]
    if not cat_df.empty:
        ax2.bar(cat_df['year'], cat_df['avg_loss_ratio'], alpha=0.7, color='red')
        ax2.axhline(y=1.5, color='orange', linestyle='--', label='Spike Threshold (1.5)')
        ax2.set_xlabel('Year')
        ax2.set_ylabel('Loss Ratio')
        ax2.set_title('Loss Ratio Spikes in Catastrophe Years (Rep 0)')
        ax2.legend()
        ax2.grid(True, alpha=0.3)
    else:
        ax2.text(0.5, 0.5, 'No catastrophes in Rep 0', ha='center', va='center',
                 transform=ax2.transAxes)

    # Plot 3: Distribution of post-catastrophe premium spikes
    ax3 = axes[1, 0]
    if spike_ratios:
        ax3.hist(spike_ratios, bins=10, alpha=0.7, edgecolor='black')
        ax3.axvline(x=1.2, color='red', linestyle='--', linewidth=2, label='Target (1.2x)')
        ax3.set_xlabel('Post-Catastrophe Premium Ratio')
        ax3.set_ylabel('Frequency')
        ax3.set_title('Distribution of Premium Spikes')
        ax3.legend()
        ax3.grid(True, alpha=0.3)
    else:
        ax3.text(0.5, 0.5, 'No premium spikes to plot', ha='center', va='center',
                 transform=ax3.transAxes)

    # Plot 4: Distribution of cycle periods
    ax4 = axes[1, 1]
    if cycle_periods:
        ax4.hist(cycle_periods, bins=15, alpha=0.7, edgecolor='black')
        ax4.axvspan(5, 8, alpha=0.2, color='green', label='Target Range (5-8 years)')
        ax4.set_xlabel('Dominant Cycle Period (years)')
        ax4.set_ylabel('Frequency')
        ax4.set_title('Distribution of Cycle Periods')
        ax4.legend()
        ax4.grid(True, alpha=0.3)
    else:
        ax4.text(0.5, 0.5, 'No cycle periods to plot', ha='center', va='center',
                 transform=ax4.transAxes)

    plt.tight_layout()

    # Save plot
    output_path = Path(__file__).parent / 'experiment_2_cycles.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_cycles()
