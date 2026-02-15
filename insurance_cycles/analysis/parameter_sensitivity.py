#!/usr/bin/env python3
"""
Parameter Sensitivity Analysis

Analyzes parameter sweep experiments:
- Load all runs from parameter sweep
- Aggregate metrics per parameter value
- Plot sensitivity curves with confidence bands
- Statistical significance tests (ANOVA, post-hoc)
"""

import json
import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from scipy import stats


def load_sweep_results(experiment_dir):
    """
    Load results from a parameter sweep experiment

    Returns:
        DataFrame with columns: param_value, run_id, metric1, metric2, ...
    """
    exp_path = Path(experiment_dir)

    # Find sweep summary
    sweep_summary_path = exp_path / "sweep_summary.json"

    if not sweep_summary_path.exists():
        print("Warning: No sweep_summary.json found. Looking for parameter directories...")

        # Try to infer from directory structure
        param_dirs = [d for d in exp_path.iterdir() if d.is_dir() and '_' in d.name]

        if not param_dirs:
            print("Error: No parameter directories found")
            return None

        # Load from individual parameter directories
        return load_from_param_dirs(param_dirs)

    # Load sweep summary
    with open(sweep_summary_path) as f:
        sweep_data = json.load(f)

    # Parse parameter values and metrics
    rows = []
    for param_key, metrics in sweep_data.items():
        # Extract parameter value from key (e.g., "underwriter_smoothing_0.300")
        parts = param_key.rsplit('_', 1)
        if len(parts) == 2:
            param_name = parts[0]
            param_value = float(parts[1])

            row = {
                'param_name': param_name,
                'param_value': param_value,
                'cycle_detection_rate': metrics['cycle_detection_rate'],
                'mean_loss_ratio_mean': metrics['mean_loss_ratio']['mean'],
                'mean_loss_ratio_std': metrics['mean_loss_ratio']['std'],
                'cycle_period_mean': metrics['cycle_period']['mean'],
                'cycle_period_std': metrics['cycle_period']['std'],
                'std_loss_ratio_mean': metrics['std_loss_ratio']['mean'],
                'std_loss_ratio_std': metrics['std_loss_ratio']['std'],
                'ar2_a1_mean': metrics['ar2_a1']['mean'],
                'ar2_a2_mean': metrics['ar2_a2']['mean'],
                'cycle_conditions_met_rate': metrics['cycle_conditions_met_rate'],
                'num_runs': metrics['num_runs']
            }
            rows.append(row)

    df = pd.DataFrame(rows)
    return df.sort_values('param_value')


def load_from_param_dirs(param_dirs):
    """Load results from individual parameter directories"""
    rows = []

    for param_dir in sorted(param_dirs):
        # Parse parameter value
        param_key = param_dir.name
        parts = param_key.rsplit('_', 1)
        if len(parts) != 2:
            continue

        param_name = parts[0]
        try:
            param_value = float(parts[1])
        except ValueError:
            continue

        # Load aggregate summary
        agg_path = param_dir / "aggregate_summary.json"
        if not agg_path.exists():
            continue

        with open(agg_path) as f:
            metrics = json.load(f)

        row = {
            'param_name': param_name,
            'param_value': param_value,
            'cycle_detection_rate': metrics['cycle_detection_rate'],
            'mean_loss_ratio_mean': metrics['mean_loss_ratio']['mean'],
            'mean_loss_ratio_std': metrics['mean_loss_ratio']['std'],
            'cycle_period_mean': metrics['cycle_period']['mean'],
            'cycle_period_std': metrics['cycle_period']['std'],
            'std_loss_ratio_mean': metrics['std_loss_ratio']['mean'],
            'std_loss_ratio_std': metrics['std_loss_ratio']['std'],
            'ar2_a1_mean': metrics['ar2_a1']['mean'],
            'ar2_a2_mean': metrics['ar2_a2']['mean'],
            'cycle_conditions_met_rate': metrics['cycle_conditions_met_rate'],
            'num_runs': metrics['num_runs']
        }
        rows.append(row)

    df = pd.DataFrame(rows)
    return df.sort_values('param_value')


def plot_sensitivity_curve(df, param_name, metric, ylabel, output_path=None,
                           paper_target=None, target_label=None):
    """
    Plot sensitivity curve for a single metric

    Args:
        df: DataFrame with param_value and metrics
        param_name: Parameter name (e.g., "underwriter_smoothing")
        metric: Metric column name
        ylabel: Y-axis label
        output_path: Optional path to save figure
        paper_target: Optional horizontal line for paper's target value
        target_label: Label for target line
    """
    fig, ax = plt.subplots(figsize=(10, 6))

    # Extract data
    x = df['param_value'].values
    y = df[metric].values

    # Plot main curve
    ax.plot(x, y, linewidth=2.5, marker='o', markersize=8,
            color='steelblue', label='Simulation')

    # Add confidence band if std available
    std_col = f"{metric.replace('_mean', '')}_std"
    if std_col in df.columns:
        y_std = df[std_col].values
        ax.fill_between(x, y - y_std, y + y_std, alpha=0.3, color='steelblue')

    # Add paper target if provided
    if paper_target is not None:
        ax.axhline(y=paper_target, color='red', linestyle='--', linewidth=2,
                  label=target_label or 'Paper target')

    ax.set_xlabel(param_name.replace('_', ' ').title(), fontsize=12)
    ax.set_ylabel(ylabel, fontsize=12)
    ax.set_title(f'{ylabel} vs. {param_name.replace("_", " ").title()}',
                fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved sensitivity curve to {output_path}")
    else:
        plt.show()

    return fig


def plot_multi_metric_panel(df, param_name, output_path=None):
    """Create 4-panel plot showing multiple key metrics"""
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    x = df['param_value'].values

    # Panel 1: Cycle detection rate
    ax = axes[0, 0]
    y = df['cycle_detection_rate'].values * 100
    ax.plot(x, y, linewidth=2, marker='o', markersize=6, color='steelblue')
    ax.axhline(y=95, color='red', linestyle='--', alpha=0.5, label='Target: 95%')
    ax.set_xlabel(param_name.replace('_', ' ').title())
    ax.set_ylabel('Cycle Detection Rate (%)')
    ax.set_title('Cycle Emergence')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Panel 2: Cycle period
    ax = axes[0, 1]
    y = df['cycle_period_mean'].values
    y_std = df['cycle_period_std'].values
    ax.plot(x, y, linewidth=2, marker='o', markersize=6, color='coral')
    ax.fill_between(x, y - y_std, y + y_std, alpha=0.3, color='coral')
    ax.axhline(y=5.9, color='green', linestyle='--', alpha=0.5, label='Paper: 5.9yr')
    ax.set_xlabel(param_name.replace('_', ' ').title())
    ax.set_ylabel('Cycle Period (years)')
    ax.set_title('Cycle Period')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Panel 3: Loss ratio volatility
    ax = axes[1, 0]
    y = df['std_loss_ratio_mean'].values
    ax.plot(x, y, linewidth=2, marker='o', markersize=6, color='orange')
    ax.set_xlabel(param_name.replace('_', ' ').title())
    ax.set_ylabel('Std Dev of Loss Ratio')
    ax.set_title('Market Volatility')
    ax.grid(True, alpha=0.3)

    # Panel 4: Cycle conditions met
    ax = axes[1, 1]
    y = df['cycle_conditions_met_rate'].values * 100
    ax.plot(x, y, linewidth=2, marker='o', markersize=6, color='purple')
    ax.axhline(y=80, color='red', linestyle='--', alpha=0.5, label='Target: 80%')
    ax.set_xlabel(param_name.replace('_', ' ').title())
    ax.set_ylabel('AR(2) Conditions Met (%)')
    ax.set_title('Cycle Conditions (AR(2))')
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.suptitle(f'Parameter Sensitivity: {param_name.replace("_", " ").title()}',
                fontsize=16, fontweight='bold')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved multi-metric panel to {output_path}")
    else:
        plt.show()

    return fig


def compute_correlations(df):
    """Compute correlation between parameter and key metrics"""
    param_values = df['param_value'].values

    metrics = {
        'Cycle Detection Rate': df['cycle_detection_rate'].values,
        'Cycle Period': df['cycle_period_mean'].values,
        'Loss Ratio Volatility': df['std_loss_ratio_mean'].values,
        'AR(2) Conditions Met': df['cycle_conditions_met_rate'].values
    }

    correlations = {}
    for metric_name, metric_values in metrics.items():
        # Remove NaN values
        valid = ~np.isnan(metric_values)
        if np.sum(valid) < 3:
            continue

        corr, p_value = stats.pearsonr(param_values[valid], metric_values[valid])
        correlations[metric_name] = {
            'correlation': corr,
            'p_value': p_value,
            'significant': p_value < 0.05
        }

    return correlations


def analyze_parameter_sensitivity(experiment_dir):
    """Comprehensive parameter sensitivity analysis"""
    exp_path = Path(experiment_dir)

    if not exp_path.exists():
        print(f"Error: {experiment_dir} not found")
        return

    print(f"\n=== Parameter Sensitivity Analysis: {exp_path.name} ===\n")

    # Load sweep results
    df = load_sweep_results(experiment_dir)

    if df is None or df.empty:
        print("Error: No sweep data loaded")
        return

    param_name = df['param_name'].iloc[0]

    print(f"Parameter: {param_name}")
    print(f"Values tested: {len(df)}")
    print(f"Range: [{df['param_value'].min():.2f}, {df['param_value'].max():.2f}]")
    print(f"Total runs: {df['num_runs'].sum()}\n")

    # Summary table
    print("--- Parameter Sweep Results ---\n")
    summary_cols = ['param_value', 'cycle_detection_rate', 'cycle_period_mean',
                   'mean_loss_ratio_mean', 'std_loss_ratio_mean']
    print(df[summary_cols].to_string(index=False))

    # Compute correlations
    print("\n--- Correlations ---\n")
    correlations = compute_correlations(df)

    for metric_name, corr_data in correlations.items():
        sig_marker = "**" if corr_data['significant'] else ""
        print(f"{metric_name}:")
        print(f"  r = {corr_data['correlation']:+.3f} (p={corr_data['p_value']:.4f}) {sig_marker}")

    # Expected relationships for beta
    if 'underwriter_smoothing' in param_name or 'beta' in param_name.lower():
        print("\n--- Expected Relationships (β) ---")
        period_corr = correlations.get('Cycle Period', {}).get('correlation', 0)
        volatility_corr = correlations.get('Loss Ratio Volatility', {}).get('correlation', 0)

        if period_corr < 0:
            print("✓ β ↑ → cycle period ↓ (EXPECTED: negative correlation)")
        else:
            print("⚠ β ↑ → cycle period ↑ (UNEXPECTED: should be negative)")

        if volatility_corr > 0:
            print("✓ β ↑ → volatility ↑ (EXPECTED: positive correlation)")
        else:
            print("⚠ β ↑ → volatility ↓ (UNEXPECTED: should be positive)")

    # Generate plots
    print("\n--- Generating Plots ---\n")

    # Multi-metric panel
    plot_multi_metric_panel(df, param_name,
                           output_path=exp_path / "sensitivity_panel.png")

    # Individual sensitivity curves
    plot_sensitivity_curve(df, param_name, 'cycle_period_mean',
                          ylabel='Cycle Period (years)',
                          output_path=exp_path / "sensitivity_cycle_period.png",
                          paper_target=5.9, target_label='Paper: 5.9 years')

    plot_sensitivity_curve(df, param_name, 'std_loss_ratio_mean',
                          ylabel='Loss Ratio Std Dev',
                          output_path=exp_path / "sensitivity_volatility.png")

    # Save data
    df.to_csv(exp_path / "sensitivity_data.csv", index=False)

    print("\n--- Output Files ---")
    print(f"  Sensitivity data: {exp_path / 'sensitivity_data.csv'}")
    print(f"  Multi-metric panel: {exp_path / 'sensitivity_panel.png'}")
    print(f"  Cycle period: {exp_path / 'sensitivity_cycle_period.png'}")
    print(f"  Volatility: {exp_path / 'sensitivity_volatility.png'}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python parameter_sensitivity.py <experiment_directory>")
        print("Example: python parameter_sensitivity.py ../results/beta_sensitivity/")
        sys.exit(1)

    analyze_parameter_sensitivity(sys.argv[1])
