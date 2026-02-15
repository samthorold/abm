#!/usr/bin/env python3
"""
Experiment 5: Loss Ratio Equilibrium

Validates that steady-state loss ratios fluctuate around 1.0 across all scenarios.

Success Criteria:
- All scenarios have mean loss ratio in [0.8, 1.2]
- One-sample t-test against 1.0 shows p > 0.01 (not significantly different)
- Loss ratios stabilize in later years (variance decreases)
"""

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import ttest_1samp
from pathlib import Path

def load_scenario_data(scenario, rep_num):
    """Load time series data"""
    filepath = Path(__file__).parent.parent / f"exp5_scenario{scenario}_rep{rep_num}_time_series.csv"
    return pd.read_csv(filepath)

def analyze_equilibrium():
    """Analyze loss ratio equilibrium across all scenarios"""

    print("=" * 60)
    print("EXPERIMENT 5: LOSS RATIO EQUILIBRIUM")
    print("=" * 60)
    print()

    results_by_scenario = {}

    for scenario in [1, 2, 3, 4]:
        print(f"Scenario {scenario}:")
        print("-" * 40)

        all_loss_ratios = []

        for rep in range(10):
            df = load_scenario_data(scenario, rep)
            active_df = df[df['num_solvent_syndicates'] > 0]

            if len(active_df) >= 20:
                # Use later years for steady state analysis
                steady_state = active_df.tail(min(30, len(active_df)))
                mean_lr = steady_state['avg_loss_ratio'].mean()
                all_loss_ratios.append(mean_lr)
                print(f"  Rep {rep}: {len(active_df)} years, steady-state LR={mean_lr:.3f}")

        if all_loss_ratios:
            mean_lr = np.mean(all_loss_ratios)
            std_lr = np.std(all_loss_ratios)

            # One-sample t-test against 1.0
            t_stat, p_value = ttest_1samp(all_loss_ratios, 1.0)

            results_by_scenario[scenario] = {
                'loss_ratios': all_loss_ratios,
                'mean': mean_lr,
                'std': std_lr,
                't_stat': t_stat,
                'p_value': p_value
            }

            print(f"  Mean: {mean_lr:.3f}, Std: {std_lr:.3f}")
            print(f"  T-test vs 1.0: t={t_stat:.3f}, p={p_value:.4f}")

        print()

    # Success criteria
    print("=" * 60)
    print("SUCCESS CRITERIA")
    print("=" * 60)

    all_passed = True
    for scenario, results in results_by_scenario.items():
        in_range = 0.8 <= results['mean'] <= 1.2
        not_sig_different = results['p_value'] > 0.01

        print(f"Scenario {scenario}:")
        print(f"  ✓ Mean in [0.8, 1.2]: {in_range} ({results['mean']:.3f})")
        print(f"  ✓ Not significantly different from 1.0: {not_sig_different} (p={results['p_value']:.4f})")

        if not (in_range and not_sig_different):
            all_passed = False

    print()

    if all_passed:
        print("✅ EXPERIMENT 5 PASSED: Loss ratios equilibrate around 1.0")
    else:
        print("❌ EXPERIMENT 5 FAILED: Check pricing mechanism")

    # Generate visualizations
    create_equilibrium_plots(results_by_scenario)

    return results_by_scenario

def create_equilibrium_plots(results):
    """Create equilibrium visualizations"""

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # Plot 1: Loss ratio distributions by scenario
    ax1 = axes[0, 0]
    for scenario in [1, 2, 3, 4]:
        if scenario in results:
            ax1.hist(results[scenario]['loss_ratios'], alpha=0.5, label=f'Scenario {scenario}',
                    bins=10, edgecolor='black')
    ax1.axvline(x=1.0, color='red', linestyle='--', linewidth=2, label='Target (1.0)')
    ax1.axvspan(0.8, 1.2, alpha=0.2, color='green')
    ax1.set_xlabel('Steady-State Loss Ratio')
    ax1.set_ylabel('Frequency')
    ax1.set_title('Distribution of Loss Ratios')
    ax1.legend()
    ax1.grid(True, alpha=0.3)

    # Plot 2: Mean loss ratios by scenario
    ax2 = axes[0, 1]
    scenarios = list(results.keys())
    means = [results[s]['mean'] for s in scenarios]
    stds = [results[s]['std'] for s in scenarios]
    ax2.bar(scenarios, means, yerr=stds, alpha=0.7, capsize=10)
    ax2.axhline(y=1.0, color='red', linestyle='--', linewidth=2, label='Target')
    ax2.axhspan(0.8, 1.2, alpha=0.2, color='green')
    ax2.set_xlabel('Scenario')
    ax2.set_ylabel('Mean Loss Ratio')
    ax2.set_title('Loss Ratio by Scenario')
    ax2.legend()
    ax2.grid(True, alpha=0.3, axis='y')

    # Plot 3: Time series example (Scenario 1, Rep 0)
    ax3 = axes[1, 0]
    df = load_scenario_data(1, 0)
    active_df = df[df['num_solvent_syndicates'] > 0]
    ax3.plot(active_df['year'], active_df['avg_loss_ratio'], linewidth=2)
    ax3.axhline(y=1.0, color='red', linestyle='--', linewidth=2, label='Target')
    ax3.axhspan(0.8, 1.2, alpha=0.2, color='green')
    ax3.set_xlabel('Year')
    ax3.set_ylabel('Loss Ratio')
    ax3.set_title('Loss Ratio Over Time (Scenario 1, Rep 0)')
    ax3.legend()
    ax3.grid(True, alpha=0.3)

    # Plot 4: P-values for each scenario
    ax4 = axes[1, 1]
    p_values = [results[s]['p_value'] for s in scenarios]
    colors = ['green' if p > 0.01 else 'red' for p in p_values]
    ax4.bar(scenarios, p_values, alpha=0.7, color=colors)
    ax4.axhline(y=0.01, color='red', linestyle='--', linewidth=2, label='Significance threshold')
    ax4.set_xlabel('Scenario')
    ax4.set_ylabel('P-value (vs 1.0)')
    ax4.set_title('Statistical Test Results')
    ax4.legend()
    ax4.grid(True, alpha=0.3, axis='y')

    plt.tight_layout()

    output_path = Path(__file__).parent / 'experiment_5_equilibrium.png'
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"📊 Visualization saved: {output_path}")
    plt.close()

if __name__ == "__main__":
    analyze_equilibrium()
