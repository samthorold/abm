#!/usr/bin/env python3
"""
Experiment 1: Fair Price Convergence

Validates that premiums converge to actuarially fair prices (~$150k per lead participation)
in the long run under attritional losses only (Scenario 1).

Success Criteria:
- Final 20-year average premium within ±20% of theoretical fair price ($150k)
- Premium variance decreases over time (market matures)
- Coefficient of variation decreases in later period vs early period
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from pathlib import Path

def load_replication_data(rep_num):
    """Load time series data for a single replication"""
    filepath = Path(__file__).parent.parent / f"exp1_rep{rep_num}_time_series.csv"
    return pd.read_csv(filepath)

def analyze_convergence():
    """Analyze premium convergence across all replications"""

    print("=" * 60)
    print("EXPERIMENT 1: FAIR PRICE CONVERGENCE")
    print("=" * 60)
    print()

    # Theoretical fair price parameters
    gamma_mean = 3_000_000.0
    yearly_claim_frequency = 0.1
    default_lead_line_size = 0.5
    volatility_weight = 0.2

    expected_loss_per_risk = gamma_mean * yearly_claim_frequency
    expected_lead_premium = expected_loss_per_risk * default_lead_line_size
    fair_price_with_loading = expected_lead_premium * (1.0 + volatility_weight)

    print(f"Theoretical Fair Price (with 20% loading): ${fair_price_with_loading:,.0f}")
    print(f"Acceptable range (±20%): ${fair_price_with_loading * 0.8:,.0f} - ${fair_price_with_loading * 1.2:,.0f}")
    print()

    # Analyze each replication
    convergence_results = []
    early_cvs = []
    late_cvs = []

    for rep in range(10):
        df = load_replication_data(rep)

        # Filter to active market years (where syndicates are solvent)
        active_df = df[df['num_solvent_syndicates'] > 0].copy()

        if len(active_df) < 30:
            print(f"⚠ Replication {rep}: Market collapsed early ({len(active_df)} years)")
            continue

        # Split into early and late periods
        midpoint = len(active_df) // 2
        early_period = active_df.iloc[:midpoint]
        late_period = active_df.iloc[midpoint:]

        # Calculate statistics
        early_avg = early_period['avg_premium'].mean()
        early_std = early_period['avg_premium'].std()
        early_cv = early_std / early_avg if early_avg > 0 else 0

        late_avg = late_period['avg_premium'].mean()
        late_std = late_period['avg_premium'].std()
        late_cv = late_std / late_avg if late_avg > 0 else 0

        # Final 20 years average (or as many as available)
        final_years = active_df.tail(min(20, len(active_df)))
        final_avg = final_years['avg_premium'].mean()

        # Check convergence criteria
        within_bounds = (fair_price_with_loading * 0.8) <= final_avg <= (fair_price_with_loading * 1.2)
        cv_decreased = late_cv < early_cv

        convergence_results.append({
            'rep': rep,
            'active_years': len(active_df),
            'early_avg': early_avg,
            'late_avg': late_avg,
            'final_20yr_avg': final_avg,
            'early_cv': early_cv,
            'late_cv': late_cv,
            'cv_decreased': cv_decreased,
            'within_bounds': within_bounds
        })

        early_cvs.append(early_cv)
        late_cvs.append(late_cv)

        print(f"Replication {rep}:")
        print(f"  Active years: {len(active_df)}/50")
        print(f"  Early avg: ${early_avg:,.0f} (CV: {early_cv:.3f})")
        print(f"  Late avg: ${late_avg:,.0f} (CV: {late_cv:.3f})")
        print(f"  Final 20-year avg: ${final_avg:,.0f}")
        print(f"  ✓ Within bounds: {within_bounds}")
        print(f"  ✓ CV decreased: {cv_decreased}")
        print()

    # Summary statistics
    results_df = pd.DataFrame(convergence_results)

    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Successful replications: {len(results_df)}/10")
    print(f"Replications within bounds: {results_df['within_bounds'].sum()}/{len(results_df)}")
    print(f"Replications with CV decrease: {results_df['cv_decreased'].sum()}/{len(results_df)}")
    print(f"Average final premium: ${results_df['final_20yr_avg'].mean():,.0f}")
    print(f"Average early CV: {np.mean(early_cvs):.3f}")
    print(f"Average late CV: {np.mean(late_cvs):.3f}")
    print()

    # Success criteria
    success = results_df['within_bounds'].sum() >= 7  # At least 7/10 within bounds
    cv_improvement = np.mean(late_cvs) < np.mean(early_cvs)

    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)
    print(f"✓ At least 7/10 within ±20% bounds: {success} ({results_df['within_bounds'].sum()}/10)")
    print(f"✓ Average CV decreased: {cv_improvement}")
    print()

    if success and cv_improvement:
        print("✅ EXPERIMENT 1 PASSED: Premiums converge to fair price")
    else:
        print("❌ EXPERIMENT 1 FAILED: Check convergence dynamics")

    # Generate visualization
    create_convergence_plot(results_df, fair_price_with_loading)

    return results_df

def create_convergence_plot(results_df, fair_price):
    """Create visualization of premium convergence"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Time series of premiums for all replications
    ax1 = axes[0, 0]
    for rep in range(10):
        try:
            df = load_replication_data(rep)
            active_df = df[df['num_solvent_syndicates'] > 0]
            ax1.plot(active_df['year'], active_df['avg_premium'], alpha=0.5, label=f'Rep {rep}')
        except:
            pass
    ax1.axhline(y=fair_price, color='r', linestyle='--', linewidth=2, label='Fair Price ($180k)')
    ax1.axhline(y=fair_price * 0.8, color='orange', linestyle=':', alpha=0.7, label='±20% bounds')
    ax1.axhline(y=fair_price * 1.2, color='orange', linestyle=':', alpha=0.7)
    ax1.set_xlabel('Year')
    ax1.set_ylabel('Average Premium ($)')
    ax1.set_title('Premium Evolution Over Time')
    ax1.legend(loc='upper right', fontsize=8)
    ax1.grid(True, alpha=0.3)

    # Plot 2: Final 20-year average vs fair price
    ax2 = axes[0, 1]
    ax2.bar(results_df['rep'], results_df['final_20yr_avg'], alpha=0.7)
    ax2.axhline(y=fair_price, color='r', linestyle='--', linewidth=2, label='Fair Price')
    ax2.axhline(y=fair_price * 0.8, color='orange', linestyle=':', alpha=0.7)
    ax2.axhline(y=fair_price * 1.2, color='orange', linestyle=':', alpha=0.7)
    ax2.set_xlabel('Replication')
    ax2.set_ylabel('Final 20-Year Avg Premium ($)')
    ax2.set_title('Convergence to Fair Price')
    ax2.legend()
    ax2.grid(True, alpha=0.3)

    # Plot 3: Coefficient of Variation comparison
    ax3 = axes[1, 0]
    x = np.arange(len(results_df))
    width = 0.35
    ax3.bar(x - width/2, results_df['early_cv'], width, label='Early Period', alpha=0.7)
    ax3.bar(x + width/2, results_df['late_cv'], width, label='Late Period', alpha=0.7)
    ax3.set_xlabel('Replication')
    ax3.set_ylabel('Coefficient of Variation')
    ax3.set_title('Premium Variability: Early vs Late Period')
    ax3.legend()
    ax3.grid(True, alpha=0.3)

    # Plot 4: Distribution of final premiums
    ax4 = axes[1, 1]
    ax4.hist(results_df['final_20yr_avg'], bins=10, alpha=0.7, edgecolor='black')
    ax4.axvline(x=fair_price, color='r', linestyle='--', linewidth=2, label='Fair Price')
    ax4.axvline(x=fair_price * 0.8, color='orange', linestyle=':', alpha=0.7)
    ax4.axvline(x=fair_price * 1.2, color='orange', linestyle=':', alpha=0.7)
    ax4.set_xlabel('Final 20-Year Avg Premium ($)')
    ax4.set_ylabel('Frequency')
    ax4.set_title('Distribution of Final Premiums')
    ax4.legend()
    ax4.grid(True, alpha=0.3)

    plt.tight_layout()

    # Save plot
    output_path = Path(__file__).parent / 'experiment_1_convergence.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_convergence()
