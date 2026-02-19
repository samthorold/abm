#!/usr/bin/env python3
"""
Plot paper figures from Lloyd's insurance simulation data

Recreates Figures 4a and 4b from:
Olmez, Ahmed, Kam, Feng, Tua (2024). "Exploring the Dynamics of the
Specialty Insurance Market Using a Novel Discrete Event Simulation Framework:
a Lloyd's of London Case Study"

Usage:
    python plot_paper_figures.py <scenario_num> <replication_num>

Example:
    python plot_paper_figures.py 1 0  # Plot Scenario 1, replication 0
"""

import sys
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np
from pathlib import Path

# Configure matplotlib for paper-quality plots
plt.rcParams['figure.figsize'] = (12, 5)
plt.rcParams['font.size'] = 10
plt.rcParams['axes.labelsize'] = 11
plt.rcParams['axes.titlesize'] = 12
plt.rcParams['legend.fontsize'] = 9
plt.rcParams['lines.linewidth'] = 1.5


def load_syndicate_data(scenario: int, replication: int) -> pd.DataFrame:
    """Load syndicate-level time series data"""
    filename = f"exp{scenario}_rep{replication}_syndicate_time_series.csv"
    filepath = Path(__file__).parent / filename

    if not filepath.exists():
        raise FileNotFoundError(
            f"Data file not found: {filepath}\n"
            f"Run simulation first: cargo run --release -p lloyds_insurance -- exp{scenario}"
        )

    df = pd.read_csv(filepath)
    return df


def load_market_data(scenario: int, replication: int) -> pd.DataFrame:
    """Load market-level time series data"""
    filename = f"exp{scenario}_rep{replication}_time_series.csv"
    filepath = Path(__file__).parent / filename

    if not filepath.exists():
        raise FileNotFoundError(
            f"Data file not found: {filepath}\n"
            f"Run simulation first: cargo run --release -p lloyds_insurance -- exp{scenario}"
        )

    df = pd.read_csv(filepath)
    return df


def plot_figure_4a(syndicate_df: pd.DataFrame, scenario: int, replication: int):
    """
    Recreate Figure 4a: Syndicate capital over time

    Shows capital depletion for each of 5 syndicates over 50 years.
    Paper shows some syndicates going bankrupt (capital -> 0) while others persist.
    """
    fig, ax = plt.subplots(figsize=(12, 6))

    # Color palette for 5 syndicates
    colors = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd']

    # Plot capital trajectory for each syndicate
    for syndicate_id in range(5):
        syndicate_data = syndicate_df[syndicate_df['syndicate_id'] == syndicate_id]

        # Sort by year to ensure proper line plotting
        syndicate_data = syndicate_data.sort_values('year')

        ax.plot(
            syndicate_data['year'],
            syndicate_data['capital'] / 1_000_000,  # Convert to millions
            color=colors[syndicate_id],
            label=f'Syndicate {syndicate_id}',
            linewidth=2
        )

    # Add horizontal line for initial capital
    initial_capital_millions = 10  # $10M from config
    ax.axhline(
        y=initial_capital_millions,
        color='gray',
        linestyle='--',
        alpha=0.5,
        linewidth=1,
        label='Initial Capital'
    )

    # Formatting
    ax.set_xlabel('Time (Years)', fontsize=12)
    ax.set_ylabel('Capital ($M)', fontsize=12)
    ax.set_title(
        f'Figure 4a: Syndicate Capital Over Time (Scenario {scenario}, Rep {replication})',
        fontsize=14,
        fontweight='bold'
    )
    ax.legend(loc='best', framealpha=0.9)
    ax.grid(True, alpha=0.3, linestyle=':')
    ax.set_xlim(0, 50)

    # Add zero line to highlight insolvencies
    ax.axhline(y=0, color='red', linestyle='-', alpha=0.3, linewidth=1)

    plt.tight_layout()
    output_path = Path(__file__).parent / f"figure_4a_scenario{scenario}_rep{replication}.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()


def plot_figure_4b(syndicate_df: pd.DataFrame, scenario: int, replication: int):
    """
    Recreate Figure 4b: Premium offered over time

    Shows box plots of premium distribution across syndicates at each time point.
    Paper shows premiums converging to a fair price (~$300k from the caption note).
    """
    fig, ax = plt.subplots(figsize=(14, 6))

    # Calculate average premium per policy for each syndicate-year
    syndicate_df['avg_premium_per_policy'] = np.where(
        syndicate_df['num_policies'] > 0,
        syndicate_df['annual_premiums'] / syndicate_df['num_policies'],
        np.nan
    )

    # Group by year and create box plot data
    years = sorted(syndicate_df['year'].unique())

    # Prepare data for box plots
    premium_data = []
    positions = []

    for year in years:
        year_data = syndicate_df[syndicate_df['year'] == year]['avg_premium_per_policy']
        # Filter out NaN and zero values (inactive syndicates)
        year_data = year_data[year_data.notna() & (year_data > 0)]

        if len(year_data) > 0:
            premium_data.append(year_data.values)
            positions.append(year)

    # Create box plots
    bp = ax.boxplot(
        premium_data,
        positions=positions,
        widths=0.8,
        patch_artist=True,
        showfliers=True,
        boxprops=dict(facecolor='lightblue', alpha=0.7),
        medianprops=dict(color='red', linewidth=2),
        whiskerprops=dict(color='gray', linewidth=1),
        capprops=dict(color='gray', linewidth=1),
        flierprops=dict(marker='o', markerfacecolor='gray', markersize=3, alpha=0.5)
    )

    # Add theoretical fair price reference
    # From Table 13: gamma_mean = $3M, yearly_claim_frequency = 0.1
    # Expected loss per risk = 0.1 × $3M = $300k
    # With lead line size 0.5 and 20% volatility loading:
    # Fair price ≈ $150k × 1.2 = $180k (but varies by syndicate participation)
    fair_price = 300_000  # Total risk fair price
    ax.axhline(
        y=fair_price / 1000,  # Convert to thousands
        color='green',
        linestyle='--',
        alpha=0.6,
        linewidth=2,
        label='Fair Price (~$300k per risk)'
    )

    # Formatting
    ax.set_xlabel('Time (Years)', fontsize=12)
    ax.set_ylabel('Premium ($1000s)', fontsize=12)
    ax.set_title(
        f'Figure 4b: Premium Distribution Over Time (Scenario {scenario}, Rep {replication})',
        fontsize=14,
        fontweight='bold'
    )
    ax.legend(loc='best', framealpha=0.9)
    ax.grid(True, alpha=0.3, linestyle=':', axis='y')
    ax.set_xlim(0, 51)

    # Format y-axis to show thousands
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, _: f'${int(x)}k'))

    plt.tight_layout()
    output_path = Path(__file__).parent / f"figure_4b_scenario{scenario}_rep{replication}.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()


def plot_combined_figure_4(syndicate_df: pd.DataFrame, scenario: int, replication: int):
    """Create combined figure with both 4a and 4b side-by-side"""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))

    # --- Panel (a): Capital over time ---
    colors = ['#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd']

    for syndicate_id in range(5):
        syndicate_data = syndicate_df[syndicate_df['syndicate_id'] == syndicate_id]
        syndicate_data = syndicate_data.sort_values('year')

        ax1.plot(
            syndicate_data['year'],
            syndicate_data['capital'] / 1_000_000,
            color=colors[syndicate_id],
            label=f'Syndicate {syndicate_id}',
            linewidth=2
        )

    ax1.axhline(y=10, color='gray', linestyle='--', alpha=0.5, linewidth=1)
    ax1.axhline(y=0, color='red', linestyle='-', alpha=0.3, linewidth=1)
    ax1.set_xlabel('Time (Years)', fontsize=11)
    ax1.set_ylabel('Capital ($M)', fontsize=11)
    ax1.set_title('(a) Syndicate Capital Depletion', fontsize=12, fontweight='bold')
    ax1.legend(loc='best', framealpha=0.9, fontsize=9)
    ax1.grid(True, alpha=0.3, linestyle=':')
    ax1.set_xlim(0, 50)

    # --- Panel (b): Premium distribution ---
    syndicate_df['avg_premium_per_policy'] = np.where(
        syndicate_df['num_policies'] > 0,
        syndicate_df['annual_premiums'] / syndicate_df['num_policies'],
        np.nan
    )

    years = sorted(syndicate_df['year'].unique())
    premium_data = []
    positions = []

    for year in years:
        year_data = syndicate_df[syndicate_df['year'] == year]['avg_premium_per_policy']
        year_data = year_data[year_data.notna() & (year_data > 0)]

        if len(year_data) > 0:
            premium_data.append(year_data.values)
            positions.append(year)

    bp = ax2.boxplot(
        premium_data,
        positions=positions,
        widths=0.8,
        patch_artist=True,
        showfliers=True,
        boxprops=dict(facecolor='lightblue', alpha=0.7),
        medianprops=dict(color='red', linewidth=2),
        whiskerprops=dict(color='gray', linewidth=1),
        capprops=dict(color='gray', linewidth=1),
        flierprops=dict(marker='o', markerfacecolor='gray', markersize=3, alpha=0.5)
    )

    fair_price = 300_000
    ax2.axhline(
        y=fair_price / 1000,
        color='green',
        linestyle='--',
        alpha=0.6,
        linewidth=2,
        label='Fair Price (~$300k)'
    )

    ax2.set_xlabel('Time (Years)', fontsize=11)
    ax2.set_ylabel('Premium ($1000s)', fontsize=11)
    ax2.set_title('(b) Premium Convergence', fontsize=12, fontweight='bold')
    ax2.legend(loc='best', framealpha=0.9, fontsize=9)
    ax2.grid(True, alpha=0.3, linestyle=':', axis='y')
    ax2.set_xlim(0, 51)
    ax2.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, _: f'${int(x)}k'))

    # Overall title
    fig.suptitle(
        f'Figure 4: Syndicate-Level Dynamics (Scenario {scenario}, Replication {replication})',
        fontsize=14,
        fontweight='bold',
        y=1.00
    )

    plt.tight_layout()
    output_path = Path(__file__).parent / f"figure_4_combined_scenario{scenario}_rep{replication}.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"Saved: {output_path}")
    plt.close()


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        print("\nUsage: python plot_paper_figures.py <scenario_num> <replication_num>")
        print("Example: python plot_paper_figures.py 1 0")
        sys.exit(1)

    try:
        scenario = int(sys.argv[1])
        replication = int(sys.argv[2])
    except ValueError:
        print("Error: scenario and replication must be integers")
        sys.exit(1)

    if scenario not in [1, 2, 3, 4]:
        print(f"Error: scenario must be 1-4, got {scenario}")
        sys.exit(1)

    print(f"Loading data for Scenario {scenario}, Replication {replication}...")

    try:
        syndicate_df = load_syndicate_data(scenario, replication)
        print(f"Loaded {len(syndicate_df)} syndicate-year records")

        print("\nCreating plots...")
        plot_figure_4a(syndicate_df, scenario, replication)
        plot_figure_4b(syndicate_df, scenario, replication)
        plot_combined_figure_4(syndicate_df, scenario, replication)

        print("\n✓ All plots created successfully!")

    except FileNotFoundError as e:
        print(f"\nError: {e}")
        print("\nTo generate data, run:")
        print(f"  cargo run --release -p lloyds_insurance -- exp{scenario}")
        sys.exit(1)
    except Exception as e:
        print(f"\nError creating plots: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
