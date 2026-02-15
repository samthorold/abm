#!/usr/bin/env python3
"""
Spectral Analysis Visualization

Power spectral density plots:
- Periodogram showing dominant frequencies
- Comparison to paper's target (5.9 years = 0.17 cycles/year)
- Multi-run aggregation
"""

import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def compute_periodogram(series, dt=1.0):
    """
    Compute periodogram (power spectral density)

    Args:
        series: Time series data
        dt: Time step (default 1 year)

    Returns:
        (freqs, power) arrays
    """
    n = len(series)
    mean = np.mean(series)

    # Frequency range: 0.05 to 0.5 cycles/year (periods 2-20 years)
    freqs = np.arange(0.05, 0.51, 0.01)
    power = np.zeros_like(freqs)

    for i, freq in enumerate(freqs):
        cos_sum = 0.0
        sin_sum = 0.0

        for t in range(n):
            angle = 2 * np.pi * freq * t
            deviation = series[t] - mean
            cos_sum += deviation * np.cos(angle)
            sin_sum += deviation * np.sin(angle)

        power[i] = (cos_sum**2 + sin_sum**2) / n

    return freqs, power


def plot_single_periodogram(series, output_path=None, title="Periodogram"):
    """Plot periodogram for a single run"""
    freqs, power = compute_periodogram(series)

    # Find dominant frequency
    dominant_idx = np.argmax(power)
    dominant_freq = freqs[dominant_idx]
    dominant_period = 1.0 / dominant_freq if dominant_freq > 0 else np.inf

    fig, ax = plt.subplots(figsize=(12, 6))

    # Plot periodogram
    ax.plot(freqs, power, linewidth=2, color='steelblue', label='Power Spectrum')
    ax.fill_between(freqs, power, alpha=0.3, color='steelblue')

    # Mark dominant frequency
    ax.axvline(x=dominant_freq, color='red', linestyle='--', linewidth=2,
              label=f'Dominant: {dominant_period:.1f} years')

    # Mark paper's target
    paper_freq = 1.0 / 5.9
    ax.axvline(x=paper_freq, color='green', linestyle='--', linewidth=2,
              label='Paper target: 5.9 years')

    # Add annotations
    ax.annotate(f'{dominant_period:.1f}yr', xy=(dominant_freq, power[dominant_idx]),
               xytext=(dominant_freq + 0.05, power[dominant_idx] * 1.1),
               arrowprops=dict(arrowstyle='->', color='red'),
               fontsize=10, color='red')

    ax.set_xlabel('Frequency (cycles/year)', fontsize=12)
    ax.set_ylabel('Power', fontsize=12)
    ax.set_title(title, fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved periodogram to {output_path}")
    else:
        plt.show()

    return fig, dominant_freq, dominant_period


def plot_multi_run_periodogram(experiment_dir, output_path=None):
    """Plot aggregated periodogram across multiple runs"""
    exp_path = Path(experiment_dir)
    run_dirs = sorted(exp_path.glob("run_*"))

    if not run_dirs:
        print("Error: No run directories found")
        return

    # Collect periodograms from all runs
    all_freqs = None
    all_powers = []
    dominant_freqs = []

    for run_dir in run_dirs:
        ts_path = run_dir / "market_timeseries.csv"
        if not ts_path.exists():
            continue

        df = load_timeseries(ts_path)
        loss_ratios = df['loss_ratio'].values

        freqs, power = compute_periodogram(loss_ratios)

        if all_freqs is None:
            all_freqs = freqs

        all_powers.append(power)

        # Find dominant frequency
        dominant_idx = np.argmax(power)
        dominant_freqs.append(freqs[dominant_idx])

    if not all_powers:
        print("Error: No periodograms computed")
        return

    all_powers = np.array(all_powers)
    mean_power = np.mean(all_powers, axis=0)
    std_power = np.std(all_powers, axis=0)

    # Statistics on dominant frequencies
    mean_dominant_freq = np.mean(dominant_freqs)
    std_dominant_freq = np.std(dominant_freqs)
    mean_period = 1.0 / mean_dominant_freq if mean_dominant_freq > 0 else np.inf

    fig, ax = plt.subplots(figsize=(12, 7))

    # Plot individual periodograms (faint)
    for power in all_powers:
        ax.plot(all_freqs, power, color='steelblue', alpha=0.1, linewidth=0.8)

    # Plot mean periodogram (bold)
    ax.plot(all_freqs, mean_power, linewidth=2.5, color='darkblue',
           label=f'Mean power ({len(all_powers)} runs)')

    # Add confidence band
    ax.fill_between(all_freqs, mean_power - std_power, mean_power + std_power,
                   color='steelblue', alpha=0.3, label='±1 std dev')

    # Mark mean dominant frequency
    ax.axvline(x=mean_dominant_freq, color='red', linestyle='--', linewidth=2,
              label=f'Mean dominant: {mean_period:.1f}±{std_dominant_freq*mean_period**2:.1f} years')

    # Mark paper's target
    paper_freq = 1.0 / 5.9
    ax.axvline(x=paper_freq, color='green', linestyle='--', linewidth=2,
              label='Paper target: 5.9 years')

    ax.set_xlabel('Frequency (cycles/year)', fontsize=12)
    ax.set_ylabel('Power', fontsize=12)
    ax.set_title(f'{exp_path.name}: Spectral Analysis',
                fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved multi-run periodogram to {output_path}")
    else:
        plt.show()

    # Print summary
    print(f"\n--- Spectral Analysis Summary ---")
    print(f"Runs analyzed: {len(all_powers)}")
    print(f"Mean dominant frequency: {mean_dominant_freq:.3f} ± {std_dominant_freq:.3f} cycles/year")
    print(f"Mean cycle period: {mean_period:.1f} years")
    print(f"Paper target: 5.9 years (0.17 cycles/year)")
    print(f"Difference: {mean_period - 5.9:.1f} years ({(mean_period/5.9 - 1)*100:.1f}%)")

    return fig


def plot_period_histogram(experiment_dir, output_path=None):
    """Plot histogram of detected cycle periods"""
    exp_path = Path(experiment_dir)
    run_dirs = sorted(exp_path.glob("run_*"))

    periods = []

    for run_dir in run_dirs:
        ts_path = run_dir / "market_timeseries.csv"
        if not ts_path.exists():
            continue

        df = load_timeseries(ts_path)
        loss_ratios = df['loss_ratio'].values

        freqs, power = compute_periodogram(loss_ratios)
        dominant_idx = np.argmax(power)
        dominant_freq = freqs[dominant_idx]

        if dominant_freq > 0:
            period = 1.0 / dominant_freq
            periods.append(period)

    if not periods:
        print("Error: No periods detected")
        return

    fig, ax = plt.subplots(figsize=(10, 6))

    # Plot histogram
    ax.hist(periods, bins=15, color='steelblue', alpha=0.7, edgecolor='black')

    # Add vertical lines for mean and paper target
    mean_period = np.mean(periods)
    ax.axvline(x=mean_period, color='red', linestyle='--', linewidth=2,
              label=f'Mean: {mean_period:.1f} years')
    ax.axvline(x=5.9, color='green', linestyle='--', linewidth=2,
              label='Paper: 5.9 years')

    ax.set_xlabel('Cycle Period (years)', fontsize=12)
    ax.set_ylabel('Frequency', fontsize=12)
    ax.set_title(f'{exp_path.name}: Distribution of Cycle Periods',
                fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3, axis='y')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved period histogram to {output_path}")
    else:
        plt.show()

    return fig


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python plot_spectral.py <experiment_directory>")
        print("Example: python plot_spectral.py ../../results/baseline_validation/")
        sys.exit(1)

    experiment_dir = sys.argv[1]
    exp_path = Path(experiment_dir)

    print(f"\n=== Spectral Analysis: {exp_path.name} ===\n")

    # Multi-run periodogram
    plot_multi_run_periodogram(experiment_dir,
                               output_path=exp_path / "spectral_periodogram.png")

    # Period histogram
    plot_period_histogram(experiment_dir,
                         output_path=exp_path / "spectral_period_histogram.png")

    print("\n--- Output Files ---")
    print(f"  Periodogram: {exp_path / 'spectral_periodogram.png'}")
    print(f"  Period histogram: {exp_path / 'spectral_period_histogram.png'}")

    print("\nDone!")
