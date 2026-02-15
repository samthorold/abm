#!/usr/bin/env python3
"""
Phase Diagram Visualization

Creates phase space plots to visualize cycle dynamics:
- x_t vs x_{t-1} (state space portrait)
- x_t vs dx/dt (velocity field)
"""

import sys
from pathlib import Path
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.patches import Ellipse


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def plot_state_space(series, output_path=None, title="State Space Portrait"):
    """
    Plot x_t vs x_{t-1}

    For stable cycles, should see elliptical trajectory
    """
    x_t = series[1:]
    x_t_minus_1 = series[:-1]

    fig, ax = plt.subplots(figsize=(10, 10))

    # Plot trajectory
    ax.plot(x_t_minus_1, x_t, 'o-', color='steelblue', markersize=3,
           alpha=0.6, linewidth=0.5)

    # Mark start and end
    ax.plot(x_t_minus_1[0], x_t[0], 'go', markersize=10, label='Start', zorder=5)
    ax.plot(x_t_minus_1[-1], x_t[-1], 'ro', markersize=10, label='End', zorder=5)

    # Add diagonal line (x_t = x_{t-1})
    ax.plot([x_t.min(), x_t.max()], [x_t.min(), x_t.max()],
           'k--', alpha=0.3, linewidth=1, label='x_t = x_{t-1}')

    # Calculate and plot mean attractor
    mean_x = np.mean(series)
    ax.plot(mean_x, mean_x, 'r*', markersize=15, label=f'Mean ({mean_x:.3f})', zorder=10)

    ax.set_xlabel('Loss Ratio at t-1', fontsize=12)
    ax.set_ylabel('Loss Ratio at t', fontsize=12)
    ax.set_title(title, fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    ax.set_aspect('equal', adjustable='box')
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved state space plot to {output_path}")
    else:
        plt.show()

    return fig


def plot_velocity_field(series, output_path=None, title="Velocity Field"):
    """
    Plot x_t vs dx/dt

    dx/dt ≈ x_t - x_{t-1}
    """
    x_t = series[1:]
    dx_dt = np.diff(series)

    fig, ax = plt.subplots(figsize=(10, 8))

    # Scatter plot with color gradient by time
    scatter = ax.scatter(x_t, dx_dt, c=np.arange(len(x_t)),
                        cmap='viridis', alpha=0.6, s=30)

    # Add horizontal line at dx/dt = 0 (equilibrium)
    ax.axhline(y=0, color='red', linestyle='--', linewidth=2,
              label='Equilibrium (dx/dt=0)')

    # Mark mean
    mean_x = np.mean(x_t)
    ax.axvline(x=mean_x, color='green', linestyle='--', alpha=0.5,
              label=f'Mean x ({mean_x:.3f})')

    # Add colorbar
    cbar = plt.colorbar(scatter, ax=ax)
    cbar.set_label('Time', rotation=270, labelpad=20)

    ax.set_xlabel('Loss Ratio (x_t)', fontsize=12)
    ax.set_ylabel('Rate of Change (dx/dt)', fontsize=12)
    ax.set_title(title, fontsize=14, fontweight='bold')
    ax.legend()
    ax.grid(True, alpha=0.3)
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved velocity field plot to {output_path}")
    else:
        plt.show()

    return fig


def plot_3d_trajectory(series, output_path=None, title="3D Phase Space"):
    """
    Plot x_t vs x_{t-1} vs x_{t-2}

    Requires 3D plotting
    """
    from mpl_toolkits.mplot3d import Axes3D

    if len(series) < 3:
        print("Error: Need at least 3 data points for 3D plot")
        return

    x_t = series[2:]
    x_t_minus_1 = series[1:-1]
    x_t_minus_2 = series[:-2]

    fig = plt.figure(figsize=(12, 10))
    ax = fig.add_subplot(111, projection='3d')

    # Plot trajectory
    ax.plot(x_t_minus_2, x_t_minus_1, x_t, color='steelblue',
           linewidth=1.5, alpha=0.7)

    # Mark start and end
    ax.scatter(x_t_minus_2[0], x_t_minus_1[0], x_t[0],
              color='green', s=100, label='Start', zorder=5)
    ax.scatter(x_t_minus_2[-1], x_t_minus_1[-1], x_t[-1],
              color='red', s=100, label='End', zorder=5)

    ax.set_xlabel('x_{t-2}', fontsize=12)
    ax.set_ylabel('x_{t-1}', fontsize=12)
    ax.set_zlabel('x_t', fontsize=12)
    ax.set_title(title, fontsize=14, fontweight='bold')
    ax.legend()
    plt.tight_layout()

    if output_path:
        plt.savefig(output_path, dpi=150, bbox_inches='tight')
        print(f"Saved 3D trajectory to {output_path}")
    else:
        plt.show()

    return fig


def analyze_phase_space(experiment_dir):
    """Generate all phase diagrams for experiment"""
    exp_path = Path(experiment_dir)

    # Use first run
    run_dirs = sorted(exp_path.glob("run_*"))
    if not run_dirs:
        print("Error: No run directories found")
        return

    first_run = run_dirs[0] / "market_timeseries.csv"
    if not first_run.exists():
        print("Error: No market time series found")
        return

    df = load_timeseries(first_run)
    loss_ratios = df['loss_ratio'].values

    print(f"\n=== Phase Diagrams: {exp_path.name} ===\n")
    print(f"Using: {run_dirs[0].name}")
    print(f"Data points: {len(loss_ratios)}\n")

    # Generate plots
    plot_state_space(loss_ratios,
                    output_path=exp_path / "phase_state_space.png",
                    title=f"{exp_path.name} - State Space (x_t vs x_{{t-1}})")

    plot_velocity_field(loss_ratios,
                       output_path=exp_path / "phase_velocity.png",
                       title=f"{exp_path.name} - Velocity Field")

    plot_3d_trajectory(loss_ratios,
                      output_path=exp_path / "phase_3d.png",
                      title=f"{exp_path.name} - 3D Phase Space")

    print("\n--- Output Files ---")
    print(f"  State space: {exp_path / 'phase_state_space.png'}")
    print(f"  Velocity field: {exp_path / 'phase_velocity.png'}")
    print(f"  3D trajectory: {exp_path / 'phase_3d.png'}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python plot_phase_diagrams.py <experiment_directory>")
        print("Example: python plot_phase_diagrams.py ../../results/baseline_validation/")
        sys.exit(1)

    analyze_phase_space(sys.argv[1])
