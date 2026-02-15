#!/usr/bin/env python3
"""
Time Series Visualization

Multi-run overlay plots with:
- Faint lines for individual runs
- Bold line for mean across runs
- Shaded region for paper's target range
- Annotated cycle peaks/troughs
"""

import json
import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from scipy import signal


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def plot_multi_run_overlay(experiment_dir, metric='loss_ratio',
                           target_range=None, output_path=None):
    """
    Create overlay plot showing all runs

    Args:
        experiment_dir: Path to experiment directory
        metric: Column name to plot ('loss_ratio', 'avg_claim', etc.)
        target_range: Tuple (min, max) for paper's target range
        output_path: Optional path to save figure
    """
    exp_path = Path(experiment_dir)
    run_dirs = sorted(exp_path.glob("run_*"))

    if not run_dirs:
        print("Error: No run directories found")
        return

    # Load all runs
    all_series = []
    years = None

    for run_dir in run_dirs:
        ts_path = run_dir / "market_timeseries.csv"
        if not ts_path.exists():
            continue

        df = load_timeseries(ts_path)
        if years is None:
            years = df['year'].values

        all_series.append(df[metric].values)

    if not all_series:
        print("Error: No time series data found")
        return

    all_series = np.array(all_series)

    # Compute mean and std
    mean_series = np.mean(all_series, axis=0)
    std_series = np.std(all_series, axis=0)

    # Create plot
    fig, ax = plt.subplots(figsize=(14, 7))

    # Plot individual runs (faint)
    for series in all_series:
        ax.plot(years, series, color='steelblue', alpha=0.15, linewidth=0.8)

    # Plot mean (bold)
    ax.plot(years, mean_series, color='darkblue', linewidth=2.5, label='Mean across runs')

    # Add confidence band (±1 std)
    ax.fill_between(years, mean_series - std_series, mean_series + std_series,
                    color='steelblue', alpha=0.3, label='±1 std dev')

    # Add target range if provided
    if target_range:
        ax.axhspan(target_range[0], target_range[1], color='green',
                  alpha=0.15, label=f'Paper target: [{target_range[0]:.2f}, {target_range[1]:.2f}]')

    # Detect and annotate peaks on mean series
    peaks, _ = signal.find_peaks(mean_series, distance=2)
    troughs, _ = signal.find_peaks(-mean_series, distance=2)

    if len(peaks) > 0:
        ax.plot(years[peaks], mean_series[peaks], 'ro', markersize=8,
               label=f'Peaks (n={len(peaks)})')

    if len(troughs) > 0:
        ax.plot(years[troughs], mean_series[troughs], 'go', markersize=8,
               label=f'Troughs (n={len(troughs)})')

    # Calculate average period
    if len(peaks) >= 2:
        periods = np.diff(years[peaks])
        avg_period = np.mean(periods)
        ax.text(0.02, 0.98, f'Avg cycle period: {avg_period:.1f} years',
               transform=ax.transAxes, verticalalignment='top',
               bbox=dict(boxstyle='round', facecolor='white', alpha=0.8))

    ax.set_xlabel('Year', fontsize=12)
    ax.set_ylabel(metric.replace('_', ' ').title(), fontsize=12)
    ax.set_title(f'{exp_path.name}: {metric.replace("_", " ").title()} Evolution',
                fontsize=14, fontweight='bold')
    ax.legend(loc='upper right')
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved time series plot to {output_path}")
    else:
        plt.show()

    return fig


def plot_multiple_metrics(experiment_dir, output_path=None):
    """Create 3-panel plot showing loss ratio, claims, and prices"""
    exp_path = Path(experiment_dir)
    run_dirs = sorted(exp_path.glob("run_*"))

    if not run_dirs:
        print("Error: No run directories found")
        return

    # Load first run for structure
    first_run = run_dirs[0] / "market_timeseries.csv"
    df_first = load_timeseries(first_run)
    years = df_first['year'].values

    # Collect all runs
    loss_ratios = []
    avg_claims = []
    avg_prices = []

    for run_dir in run_dirs:
        ts_path = run_dir / "market_timeseries.csv"
        if not ts_path.exists():
            continue

        df = load_timeseries(ts_path)
        loss_ratios.append(df['loss_ratio'].values)
        avg_claims.append(df['avg_claim'].values)
        avg_prices.append(df['avg_price'].values)

    loss_ratios = np.array(loss_ratios)
    avg_claims = np.array(avg_claims)
    avg_prices = np.array(avg_prices)

    # Create 3-panel figure
    fig, axes = plt.subplots(3, 1, figsize=(14, 12))

    # Panel 1: Loss Ratios
    ax = axes[0]
    mean_lr = np.mean(loss_ratios, axis=0)
    std_lr = np.std(loss_ratios, axis=0)

    for lr in loss_ratios:
        ax.plot(years, lr, color='steelblue', alpha=0.15, linewidth=0.8)
    ax.plot(years, mean_lr, color='darkblue', linewidth=2.5, label='Mean')
    ax.fill_between(years, mean_lr - std_lr, mean_lr + std_lr,
                    color='steelblue', alpha=0.3)
    ax.axhspan(0.9, 1.1, color='green', alpha=0.15, label='Target: [0.9, 1.1]')
    ax.axhline(y=1.0, color='red', linestyle='--', alpha=0.5)
    ax.set_ylabel('Loss Ratio')
    ax.set_title('Loss Ratio Evolution')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Panel 2: Average Claims
    ax = axes[1]
    mean_claims = np.mean(avg_claims, axis=0)
    std_claims = np.std(avg_claims, axis=0)

    for claims in avg_claims:
        ax.plot(years, claims, color='coral', alpha=0.15, linewidth=0.8)
    ax.plot(years, mean_claims, color='darkred', linewidth=2.5, label='Mean')
    ax.fill_between(years, mean_claims - std_claims, mean_claims + std_claims,
                    color='coral', alpha=0.3)
    ax.set_ylabel('Average Claim ($)')
    ax.set_title('Average Claim Evolution')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Panel 3: Average Prices
    ax = axes[2]
    mean_prices = np.mean(avg_prices, axis=0)
    std_prices = np.std(avg_prices, axis=0)

    for prices in avg_prices:
        ax.plot(years, prices, color='orange', alpha=0.15, linewidth=0.8)
    ax.plot(years, mean_prices, color='darkorange', linewidth=2.5, label='Mean')
    ax.fill_between(years, mean_prices - std_prices, mean_prices + std_prices,
                    color='orange', alpha=0.3)
    ax.set_xlabel('Year')
    ax.set_ylabel('Average Price ($)')
    ax.set_title('Average Price Evolution')
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.suptitle(f'{exp_path.name}: Market Dynamics', fontsize=16, fontweight='bold')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved multi-metric plot to {output_path}")
    else:
        plt.show()

    return fig


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python plot_timeseries.py <experiment_directory> [metric]")
        print("Example: python plot_timeseries.py ../../results/baseline_validation/")
        print("Metrics: loss_ratio (default), avg_claim, avg_price")
        sys.exit(1)

    experiment_dir = sys.argv[1]
    metric = sys.argv[2] if len(sys.argv) > 2 else 'loss_ratio'

    exp_path = Path(experiment_dir)

    # Single metric plot
    output = exp_path / f"timeseries_{metric}.png"
    target_range = (0.9, 1.1) if metric == 'loss_ratio' else None
    plot_multi_run_overlay(experiment_dir, metric=metric,
                          target_range=target_range, output_path=output)

    # Multi-metric panel
    output_multi = exp_path / "timeseries_panel.png"
    plot_multiple_metrics(experiment_dir, output_path=output_multi)

    print("\nDone!")
