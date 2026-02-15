#!/usr/bin/env python3
"""
Cycle Detection and Statistical Validation

Analyzes insurance market time series for cyclical behavior using:
- Peak detection
- AR(2) model fitting (Yule-Walker)
- Spectral analysis (periodogram)
- Autocorrelation functions
"""

import json
import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from scipy import signal
from statsmodels.tsa.stattools import acf, pacf
from statsmodels.graphics.tsaplots import plot_acf, plot_pacf


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def load_summary(json_path):
    """Load summary statistics from JSON"""
    with open(json_path) as f:
        return json.load(f)


def detect_cycles_peaks(series, min_distance=2):
    """Detect cycles using peak detection"""
    peaks, _ = signal.find_peaks(series, distance=min_distance)

    if len(peaks) < 2:
        return False, None

    # Calculate average period between peaks
    periods = np.diff(peaks)
    avg_period = np.mean(periods)

    return len(peaks) >= 2, avg_period


def fit_ar2_yule_walker(series):
    """
    Fit AR(2) model using Yule-Walker equations

    Returns (a0, a1, a2) where x_t = a0 + a1·x_{t-1} + a2·x_{t-2}
    """
    if len(series) < 10:
        return None

    # Calculate autocorrelations
    acf_vals = acf(series, nlags=2, fft=False)
    rho1, rho2 = acf_vals[1], acf_vals[2]

    # Yule-Walker equations
    denom = 1 - rho1**2
    if abs(denom) < 1e-10:
        return None

    a1 = rho1 * (1 - rho2) / denom
    a2 = (rho2 - rho1**2) / denom
    a0 = np.mean(series) * (1 - a1 - a2)

    return (a0, a1, a2)


def check_cycle_conditions(a1, a2):
    """
    Check AR(2) cycle conditions from paper:
    1. a1 > 0 (positive feedback)
    2. -1 < a2 < 0 (damped oscillation)
    3. a1² + 4a2 < 0 (complex roots → cycles)
    """
    cond1 = a1 > 0
    cond2 = -1 < a2 < 0
    cond3 = a1**2 + 4*a2 < 0

    return cond1 and cond2 and cond3, (cond1, cond2, cond3)


def periodogram(series, dt=1.0):
    """Compute periodogram (power spectral density)"""
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


def plot_cycle_diagnostics(series, output_path=None, title="Cycle Diagnostics"):
    """Create 4-panel diagnostic plot"""
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))

    # 1. Time series with detected peaks
    ax = axes[0, 0]
    ax.plot(series, linewidth=1.5, label='Loss Ratio')
    peaks, _ = signal.find_peaks(series, distance=2)
    ax.plot(peaks, series[peaks], 'ro', markersize=8, label='Peaks')
    ax.axhline(y=1.0, color='gray', linestyle='--', alpha=0.5, label='Target=1.0')
    ax.set_xlabel('Year')
    ax.set_ylabel('Loss Ratio')
    ax.set_title('Time Series with Cycle Peaks')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # 2. ACF
    ax = axes[0, 1]
    plot_acf(series, lags=20, ax=ax, alpha=0.05)
    ax.set_title('Autocorrelation Function (ACF)')
    ax.grid(True, alpha=0.3)

    # 3. PACF
    ax = axes[1, 0]
    plot_pacf(series, lags=20, ax=ax, alpha=0.05, method='ywm')
    ax.set_title('Partial Autocorrelation Function (PACF)')
    ax.grid(True, alpha=0.3)

    # 4. Periodogram
    ax = axes[1, 1]
    freqs, power = periodogram(series)
    ax.plot(freqs, power, linewidth=1.5)

    # Mark dominant frequency
    dominant_idx = np.argmax(power)
    dominant_freq = freqs[dominant_idx]
    dominant_period = 1.0 / dominant_freq if dominant_freq > 0 else 0

    ax.axvline(x=dominant_freq, color='red', linestyle='--',
               label=f'Dominant: {dominant_period:.1f}yr')
    ax.axvline(x=0.17, color='green', linestyle='--', alpha=0.5,
               label='Paper target: 5.9yr')

    ax.set_xlabel('Frequency (cycles/year)')
    ax.set_ylabel('Power')
    ax.set_title('Periodogram')
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.suptitle(title, fontsize=14, fontweight='bold')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved diagnostic plot to {output_path}")
    else:
        plt.show()

    return fig


def analyze_experiment(experiment_dir):
    """Analyze a complete experiment directory"""
    exp_path = Path(experiment_dir)

    if not exp_path.exists():
        print(f"Error: {experiment_dir} not found")
        return

    print(f"\n=== Analyzing: {exp_path.name} ===\n")

    # Find all run directories
    run_dirs = sorted(exp_path.glob("run_*"))

    if not run_dirs:
        print("No run directories found")
        return

    print(f"Found {len(run_dirs)} runs\n")

    # Aggregate metrics
    all_has_cycles = []
    all_periods = []
    all_ar2_results = []

    for run_dir in run_dirs:
        summary_path = run_dir / "summary.json"
        if not summary_path.exists():
            continue

        summary = load_summary(summary_path)
        cycle_metrics = summary['cycle_metrics']

        all_has_cycles.append(cycle_metrics['has_cycles'])

        if cycle_metrics['cycle_period']:
            all_periods.append(cycle_metrics['cycle_period'])

        if cycle_metrics['ar2_coefficients']:
            all_ar2_results.append(cycle_metrics['ar2_coefficients'])

    # Print summary
    cycle_rate = np.mean(all_has_cycles) * 100
    print(f"Cycle Detection Rate: {cycle_rate:.1f}%")

    if all_periods:
        print(f"Cycle Period: {np.mean(all_periods):.2f} ± {np.std(all_periods):.2f} years")
        print(f"  Range: [{np.min(all_periods):.1f}, {np.max(all_periods):.1f}] years")
        print(f"  Paper target: 5.9 years")

    if all_ar2_results:
        a0s = [r[0] for r in all_ar2_results]
        a1s = [r[1] for r in all_ar2_results]
        a2s = [r[2] for r in all_ar2_results]

        print(f"\nAR(2) Coefficients:")
        print(f"  a0: {np.mean(a0s):.3f} ± {np.std(a0s):.3f}")
        print(f"  a1: {np.mean(a1s):.3f} ± {np.std(a1s):.3f} (paper: ~0.467)")
        print(f"  a2: {np.mean(a2s):.3f} ± {np.std(a2s):.3f} (paper: ~-0.100)")

        # Check conditions
        conditions_met = [check_cycle_conditions(a1, a2)[0] for _, a1, a2 in all_ar2_results]
        print(f"  Cycle conditions met: {np.mean(conditions_met)*100:.1f}%")

    # Plot diagnostics for first run
    if run_dirs:
        first_run = run_dirs[0]
        ts_path = first_run / "market_timeseries.csv"

        if ts_path.exists():
            df = load_timeseries(ts_path)
            loss_ratios = df['loss_ratio'].values

            output_plot = exp_path / "cycle_diagnostics.png"
            plot_cycle_diagnostics(loss_ratios, output_path=output_plot,
                                 title=f"{exp_path.name} - Run 0")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python cycle_analysis.py <experiment_directory>")
        print("Example: python cycle_analysis.py ../results/baseline_validation/")
        sys.exit(1)

    analyze_experiment(sys.argv[1])
