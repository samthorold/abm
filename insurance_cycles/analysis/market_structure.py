#!/usr/bin/env python3
"""
Market Structure Analysis

Analyzes market concentration and competition dynamics:
- Herfindahl-Hirschman Index (HHI) evolution
- Gini coefficient trends
- Market share distribution
- Dominant firm emergence
"""

import json
import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def load_insurer_snapshots(csv_path):
    """Load insurer snapshots from CSV"""
    return pd.read_csv(csv_path)


def calculate_herfindahl(market_shares):
    """
    Calculate Herfindahl-Hirschman Index (HHI)

    HHI = Σ(market_share²)
    Range: [1/N, 1] where N = number of firms
    - 1/N = perfect competition (equal shares)
    - 1 = monopoly
    - >0.25 = highly concentrated market
    """
    return np.sum(np.array(market_shares) ** 2)


def calculate_gini(market_shares):
    """
    Calculate Gini coefficient

    Measures inequality in distribution
    Range: [0, 1]
    - 0 = perfect equality
    - 1 = perfect inequality (monopoly)
    """
    shares = np.array(market_shares)
    n = len(shares)

    if n == 0 or np.sum(shares) == 0:
        return 0.0

    # Sort shares
    shares_sorted = np.sort(shares)

    # Calculate Gini
    cumsum = np.cumsum(shares_sorted)
    gini = (n + 1 - 2 * np.sum(cumsum) / cumsum[-1]) / n

    return gini


def market_share_evolution(insurer_snapshots, num_insurers=20):
    """
    Track market share evolution over time

    Returns DataFrame with columns: year, insurer_id, market_share, num_customers
    """
    # Group by year and calculate total customers
    yearly_totals = insurer_snapshots.groupby('year')['num_customers'].sum()

    # Calculate market shares
    shares = []
    for year in insurer_snapshots['year'].unique():
        year_data = insurer_snapshots[insurer_snapshots['year'] == year]
        total_customers = yearly_totals[year]

        for _, row in year_data.iterrows():
            share = row['num_customers'] / total_customers if total_customers > 0 else 0
            shares.append({
                'year': year,
                'insurer_id': row['insurer_id'],
                'market_share': share,
                'num_customers': row['num_customers'],
                'capital': row['capital'],
                'price': row['price'],
                'is_solvent': row['is_solvent']
            })

    return pd.DataFrame(shares)


def calculate_hhi_timeseries(share_evolution):
    """Calculate HHI for each year"""
    hhi_data = []

    for year in share_evolution['year'].unique():
        year_shares = share_evolution[share_evolution['year'] == year]['market_share'].values
        hhi = calculate_herfindahl(year_shares)
        gini = calculate_gini(year_shares)

        hhi_data.append({
            'year': year,
            'hhi': hhi,
            'gini': gini,
            'num_active': len(year_shares[year_shares > 0])
        })

    return pd.DataFrame(hhi_data)


def plot_market_concentration(hhi_timeseries, output_path=None, title="Market Concentration"):
    """Plot HHI and Gini over time"""
    fig, axes = plt.subplots(2, 1, figsize=(12, 8))

    # HHI plot
    ax = axes[0]
    ax.plot(hhi_timeseries['year'], hhi_timeseries['hhi'], linewidth=2, color='steelblue')
    ax.axhline(y=0.25, color='red', linestyle='--', label='High concentration (0.25)')
    ax.axhline(y=0.15, color='orange', linestyle='--', label='Moderate concentration (0.15)')
    ax.axhline(y=0.05, color='green', linestyle='--', label='Low concentration (1/20=0.05)')
    ax.set_xlabel('Year')
    ax.set_ylabel('HHI')
    ax.set_title('Herfindahl-Hirschman Index (Market Concentration)')
    ax.legend()
    ax.grid(True, alpha=0.3)

    # Gini plot
    ax = axes[1]
    ax.plot(hhi_timeseries['year'], hhi_timeseries['gini'], linewidth=2, color='coral')
    ax.axhline(y=0.5, color='red', linestyle='--', label='High inequality (0.5)')
    ax.set_xlabel('Year')
    ax.set_ylabel('Gini Coefficient')
    ax.set_title('Gini Coefficient (Market Share Inequality)')
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.suptitle(title, fontsize=14, fontweight='bold')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved concentration plot to {output_path}")
    else:
        plt.show()

    return fig


def plot_market_share_heatmap(share_evolution, output_path=None, title="Market Share Evolution"):
    """Create heatmap showing insurer_id × year market shares"""
    # Pivot to get insurer_id × year matrix
    pivot = share_evolution.pivot(index='insurer_id', columns='year', values='market_share')
    pivot = pivot.fillna(0)

    # Sort by average market share (descending)
    avg_shares = pivot.mean(axis=1).sort_values(ascending=False)
    pivot = pivot.loc[avg_shares.index]

    fig, ax = plt.subplots(figsize=(14, 8))

    sns.heatmap(pivot, cmap='YlOrRd', cbar_kws={'label': 'Market Share'},
                linewidths=0.5, linecolor='white', ax=ax)

    ax.set_xlabel('Year')
    ax.set_ylabel('Insurer ID')
    ax.set_title(title)

    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved heatmap to {output_path}")
    else:
        plt.show()

    return fig


def plot_top_firms(share_evolution, top_n=5, output_path=None):
    """Plot market share trajectories for top N firms"""
    # Calculate average market share per insurer
    avg_shares = share_evolution.groupby('insurer_id')['market_share'].mean()
    top_insurers = avg_shares.nlargest(top_n).index

    fig, ax = plt.subplots(figsize=(12, 6))

    for insurer_id in top_insurers:
        insurer_data = share_evolution[share_evolution['insurer_id'] == insurer_id]
        ax.plot(insurer_data['year'], insurer_data['market_share'],
                linewidth=2, label=f'Insurer {insurer_id}', marker='o', markersize=3)

    ax.set_xlabel('Year')
    ax.set_ylabel('Market Share')
    ax.set_title(f'Top {top_n} Firms by Average Market Share')
    ax.legend()
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved top firms plot to {output_path}")
    else:
        plt.show()

    return fig


def analyze_market_structure(experiment_dir):
    """Comprehensive market structure analysis"""
    exp_path = Path(experiment_dir)

    if not exp_path.exists():
        print(f"Error: {experiment_dir} not found")
        return

    print(f"\n=== Market Structure Analysis: {exp_path.name} ===\n")

    # Find first run with insurer snapshots
    run_dirs = sorted(exp_path.glob("run_*"))
    insurer_csv = None

    for run_dir in run_dirs:
        candidate = run_dir / "insurer_snapshots.csv"
        if candidate.exists():
            insurer_csv = candidate
            break

    if not insurer_csv:
        print("Error: No insurer snapshots found")
        return

    print(f"Analyzing: {insurer_csv.parent.name}")

    # Load data
    insurer_snapshots = load_insurer_snapshots(insurer_csv)

    # Calculate market share evolution
    share_evolution = market_share_evolution(insurer_snapshots)

    # Calculate HHI time series
    hhi_ts = calculate_hhi_timeseries(share_evolution)

    # Print summary statistics
    print("\n--- Summary Statistics ---\n")
    print(f"Time period: Year {hhi_ts['year'].min()} to {hhi_ts['year'].max()}")
    print(f"\nHerfindahl Index (HHI):")
    print(f"  Mean: {hhi_ts['hhi'].mean():.4f}")
    print(f"  Std:  {hhi_ts['hhi'].std():.4f}")
    print(f"  Min:  {hhi_ts['hhi'].min():.4f}")
    print(f"  Max:  {hhi_ts['hhi'].max():.4f}")

    print(f"\nGini Coefficient:")
    print(f"  Mean: {hhi_ts['gini'].mean():.4f}")
    print(f"  Std:  {hhi_ts['gini'].std():.4f}")

    # Concentration classification
    avg_hhi = hhi_ts['hhi'].mean()
    if avg_hhi > 0.25:
        concentration = "High (Oligopoly)"
    elif avg_hhi > 0.15:
        concentration = "Moderate"
    else:
        concentration = "Low (Competitive)"

    print(f"\nMarket Concentration: {concentration}")

    # Top firms analysis
    avg_shares = share_evolution.groupby('insurer_id')['market_share'].mean()
    top_5 = avg_shares.nlargest(5)

    print(f"\nTop 5 Firms (by average market share):")
    for insurer_id, share in top_5.items():
        print(f"  Insurer {insurer_id}: {share*100:.1f}%")

    # Check for dominant firm
    max_share = share_evolution.groupby('year')['market_share'].max().mean()
    if max_share > 0.4:
        print(f"\n⚠ Dominant firm detected: Average max share = {max_share*100:.1f}%")

    # Generate plots
    output_dir = exp_path
    print("\n--- Generating Plots ---\n")

    plot_market_concentration(hhi_ts,
                             output_path=output_dir / "market_concentration.png",
                             title=f"{exp_path.name} - Market Concentration")

    plot_market_share_heatmap(share_evolution,
                              output_path=output_dir / "market_share_heatmap.png",
                              title=f"{exp_path.name} - Market Share Evolution")

    plot_top_firms(share_evolution, top_n=5,
                   output_path=output_dir / "top_firms.png")

    # Save data
    hhi_ts.to_csv(output_dir / "hhi_timeseries.csv", index=False)
    share_evolution.to_csv(output_dir / "market_share_evolution.csv", index=False)

    print("\n--- Output Files ---")
    print(f"  HHI time series: {output_dir / 'hhi_timeseries.csv'}")
    print(f"  Market shares: {output_dir / 'market_share_evolution.csv'}")
    print(f"  Concentration plot: {output_dir / 'market_concentration.png'}")
    print(f"  Share heatmap: {output_dir / 'market_share_heatmap.png'}")
    print(f"  Top firms plot: {output_dir / 'top_firms.png'}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python market_structure.py <experiment_directory>")
        print("Example: python market_structure.py ../results/baseline_validation/")
        sys.exit(1)

    analyze_market_structure(sys.argv[1])
