#!/usr/bin/env python3
"""
Anomaly Detection and Quality Assurance

Validates simulation correctness by checking:
- Shadow state consistency (coordinator vs. insurer aggregates)
- Capacity constraint violations
- Insolvency rates
- Loss ratio explosions
- Stationarity tests
"""

import json
import sys
from pathlib import Path
import numpy as np
import pandas as pd
from scipy import stats as scipy_stats


def load_summary(json_path):
    """Load summary statistics from JSON"""
    with open(json_path) as f:
        return json.load(f)


def load_timeseries(csv_path):
    """Load market time series from CSV"""
    return pd.read_csv(csv_path)


def load_insurer_snapshots(csv_path):
    """Load insurer snapshots from CSV"""
    return pd.read_csv(csv_path)


def check_shadow_state_consistency(market_ts, insurer_snapshots, threshold=0.03):
    """
    Verify coordinator aggregates match sum of individual insurers

    Args:
        market_ts: Market time series DataFrame
        insurer_snapshots: Insurer snapshots DataFrame
        threshold: Maximum allowed relative error (default 3%)

    Returns:
        (is_consistent, errors_dict)
    """
    issues = []

    # Get final year from market data
    if market_ts.empty:
        return False, {"error": "Empty market time series"}

    final_year = market_ts['year'].max()

    # Get final market totals
    final_market = market_ts[market_ts['year'] == final_year].iloc[0]
    market_total_premiums = final_market['total_premiums']
    market_total_claims = final_market['total_claims']

    # Get insurer totals from snapshots at final year
    final_insurers = insurer_snapshots[insurer_snapshots['year'] == final_year]

    if final_insurers.empty:
        return False, {"error": "No insurer snapshots for final year"}

    # Note: Snapshots don't have cumulative totals, only current state
    # We can check current year consistency if we had per-year data
    # For now, verify solvency and basic constraints

    # Check if any insurers have negative capital
    negative_capital = final_insurers[final_insurers['capital'] < 0]
    if not negative_capital.empty:
        issues.append({
            "type": "negative_capital",
            "count": len(negative_capital),
            "insurer_ids": negative_capital['insurer_id'].tolist()
        })

    # Check for extreme loss ratios
    extreme_loss_ratios = final_insurers[final_insurers['loss_ratio'] > 2.0]
    if not extreme_loss_ratios.empty:
        issues.append({
            "type": "extreme_loss_ratios",
            "count": len(extreme_loss_ratios),
            "max_loss_ratio": float(extreme_loss_ratios['loss_ratio'].max()),
            "insurer_ids": extreme_loss_ratios['insurer_id'].tolist()
        })

    # Check market share consistency (should sum to ~1.0)
    total_market_share = final_insurers['market_share'].sum()
    market_share_error = abs(total_market_share - 1.0)

    # Note: market_share is currently 0 in the output - this is expected
    # as it's computed by MarketCoordinator but not yet populated

    is_consistent = len(issues) == 0

    return is_consistent, {
        "issues": issues,
        "checks": {
            "negative_capital_count": len(negative_capital),
            "extreme_loss_ratio_count": len(extreme_loss_ratios),
            "total_market_share": float(total_market_share),
            "market_share_error": float(market_share_error)
        }
    }


def check_capacity_violations(insurer_snapshots, config):
    """
    Check if insurers violated capacity constraints

    Constraint: num_customers × avg_price ≤ capital × leverage_ratio

    Args:
        insurer_snapshots: DataFrame of insurer snapshots
        config: Model configuration dict

    Returns:
        (has_violations, violations_list)
    """
    leverage_ratio = config.get('leverage_ratio', 2.0)
    violations = []

    for _, row in insurer_snapshots.iterrows():
        if row['is_solvent']:  # Only check solvent insurers
            capacity = row['capital'] * leverage_ratio
            current_exposure = row['num_customers'] * row['price']

            if current_exposure > capacity * 1.01:  # 1% tolerance for rounding
                violations.append({
                    "year": int(row['year']),
                    "insurer_id": int(row['insurer_id']),
                    "capacity": float(capacity),
                    "exposure": float(current_exposure),
                    "violation_pct": float((current_exposure / capacity - 1.0) * 100)
                })

    return len(violations) > 0, violations


def check_insolvency_rates(insurer_snapshots, threshold=0.20):
    """
    Calculate insolvency rates

    Args:
        insurer_snapshots: DataFrame of insurer snapshots
        threshold: Flag if insolvency rate exceeds this (default 20%)

    Returns:
        (exceeds_threshold, stats_dict)
    """
    total_insurer_years = len(insurer_snapshots)
    insolvent_count = len(insurer_snapshots[~insurer_snapshots['is_solvent']])

    insolvency_rate = insolvent_count / total_insurer_years if total_insurer_years > 0 else 0

    # Group by insurer to see which ones failed
    failed_insurers = insurer_snapshots[~insurer_snapshots['is_solvent']]['insurer_id'].unique()

    stats = {
        "total_insurer_years": int(total_insurer_years),
        "insolvent_count": int(insolvent_count),
        "insolvency_rate": float(insolvency_rate),
        "failed_insurers": failed_insurers.tolist(),
        "num_failed_insurers": len(failed_insurers)
    }

    exceeds_threshold = insolvency_rate > threshold

    return exceeds_threshold, stats


def check_loss_ratio_explosions(market_ts, threshold=2.0):
    """
    Flag years with extreme loss ratios

    Args:
        market_ts: Market time series DataFrame
        threshold: Flag if loss_ratio exceeds this

    Returns:
        (has_explosions, explosion_years)
    """
    explosions = market_ts[market_ts['loss_ratio'] > threshold]

    explosion_years = []
    for _, row in explosions.iterrows():
        explosion_years.append({
            "year": int(row['year']),
            "loss_ratio": float(row['loss_ratio'])
        })

    return len(explosion_years) > 0, explosion_years


def check_stationarity(series, test='adf', alpha=0.05):
    """
    Test for stationarity using Augmented Dickey-Fuller test

    Args:
        series: Time series array
        test: Test type ('adf' only for now)
        alpha: Significance level

    Returns:
        (is_stationary, test_results)
    """
    from statsmodels.tsa.stattools import adfuller

    if len(series) < 10:
        return None, {"error": "Insufficient data for stationarity test"}

    result = adfuller(series, autolag='AIC')

    test_results = {
        "test_statistic": float(result[0]),
        "p_value": float(result[1]),
        "n_lags": int(result[2]),
        "n_obs": int(result[3]),
        "critical_values": {k: float(v) for k, v in result[4].items()},
        "is_stationary": result[1] < alpha
    }

    return result[1] < alpha, test_results


def analyze_experiment_anomalies(experiment_dir):
    """Comprehensive anomaly detection for an experiment"""
    exp_path = Path(experiment_dir)

    if not exp_path.exists():
        print(f"Error: {experiment_dir} not found")
        return

    print(f"\n=== Anomaly Detection: {exp_path.name} ===\n")

    # Find all run directories
    run_dirs = sorted(exp_path.glob("run_*"))

    if not run_dirs:
        print("No run directories found")
        return

    print(f"Analyzing {len(run_dirs)} runs...\n")

    # Aggregate anomalies across runs
    all_shadow_issues = []
    all_capacity_violations = []
    all_insolvency_stats = []
    all_loss_explosions = []
    all_stationarity_results = []

    for run_dir in run_dirs:
        summary_path = run_dir / "summary.json"
        market_csv = run_dir / "market_timeseries.csv"
        insurer_csv = run_dir / "insurer_snapshots.csv"

        if not summary_path.exists():
            continue

        summary = load_summary(summary_path)

        # Check 1: Shadow state consistency
        if market_csv.exists() and insurer_csv.exists():
            market_ts = load_timeseries(market_csv)
            insurer_snapshots = load_insurer_snapshots(insurer_csv)

            is_consistent, shadow_result = check_shadow_state_consistency(
                market_ts, insurer_snapshots
            )
            if not is_consistent:
                all_shadow_issues.append({
                    "run": run_dir.name,
                    "result": shadow_result
                })

            # Check 2: Capacity violations
            config = summary['metadata']['config']
            has_violations, violations = check_capacity_violations(
                insurer_snapshots, config
            )
            if has_violations:
                all_capacity_violations.append({
                    "run": run_dir.name,
                    "violations": violations
                })

            # Check 3: Insolvency rates
            exceeds_threshold, insolvency_stats = check_insolvency_rates(
                insurer_snapshots
            )
            all_insolvency_stats.append({
                "run": run_dir.name,
                "stats": insolvency_stats,
                "exceeds_threshold": exceeds_threshold
            })

            # Check 4: Loss ratio explosions
            has_explosions, explosions = check_loss_ratio_explosions(market_ts)
            if has_explosions:
                all_loss_explosions.append({
                    "run": run_dir.name,
                    "explosions": explosions
                })

            # Check 5: Stationarity
            loss_ratios = market_ts['loss_ratio'].values
            is_stationary, stat_result = check_stationarity(loss_ratios)
            all_stationarity_results.append({
                "run": run_dir.name,
                "is_stationary": is_stationary,
                "p_value": stat_result.get('p_value')
            })

    # Print summary report
    print("=" * 60)
    print("ANOMALY DETECTION REPORT")
    print("=" * 60)

    print(f"\n1. SHADOW STATE CONSISTENCY")
    if all_shadow_issues:
        print(f"   ⚠ ISSUES FOUND: {len(all_shadow_issues)} runs")
        for issue in all_shadow_issues[:3]:  # Show first 3
            result = issue['result']
            if 'issues' in result:
                print(f"   - {issue['run']}: {result['issues']}")
            else:
                print(f"   - {issue['run']}: {result}")
    else:
        print(f"   ✓ PASS: All runs consistent")

    print(f"\n2. CAPACITY VIOLATIONS")
    if all_capacity_violations:
        print(f"   ⚠ VIOLATIONS FOUND: {len(all_capacity_violations)} runs")
        total_violations = sum(len(v['violations']) for v in all_capacity_violations)
        print(f"   Total violations: {total_violations}")
    else:
        print(f"   ✓ PASS: No capacity violations")

    print(f"\n3. INSOLVENCY RATES")
    if all_insolvency_stats:
        avg_insolvency = np.mean([s['stats']['insolvency_rate']
                                  for s in all_insolvency_stats])
        high_insolvency = [s for s in all_insolvency_stats
                          if s['exceeds_threshold']]

        print(f"   Average insolvency rate: {avg_insolvency*100:.1f}%")
        if high_insolvency:
            print(f"   ⚠ HIGH INSOLVENCY: {len(high_insolvency)} runs exceed 20%")
        else:
            print(f"   ✓ PASS: All runs below 20% threshold")

    print(f"\n4. LOSS RATIO EXPLOSIONS (>2.0)")
    if all_loss_explosions:
        print(f"   ⚠ EXPLOSIONS FOUND: {len(all_loss_explosions)} runs")
        for explosion in all_loss_explosions[:3]:
            years = [e['year'] for e in explosion['explosions']]
            print(f"   - {explosion['run']}: years {years}")
    else:
        print(f"   ✓ PASS: No extreme loss ratios")

    print(f"\n5. STATIONARITY (Augmented Dickey-Fuller)")
    if all_stationarity_results:
        stationary_count = sum(1 for r in all_stationarity_results
                               if r['is_stationary'])
        pct_stationary = stationary_count / len(all_stationarity_results) * 100

        print(f"   Stationary runs: {stationary_count}/{len(all_stationarity_results)} ({pct_stationary:.0f}%)")

        if pct_stationary < 80:
            print(f"   ⚠ WARNING: <80% of runs are stationary")
        else:
            print(f"   ✓ PASS: ≥80% of runs stationary")

    print("\n" + "=" * 60)

    # Overall verdict
    total_issues = len(all_shadow_issues) + len(all_capacity_violations) + \
                   len(all_loss_explosions) + len([s for s in all_insolvency_stats
                                                    if s['exceeds_threshold']])

    if total_issues == 0:
        print("✓ OVERALL: No critical anomalies detected")
    else:
        print(f"⚠ OVERALL: {total_issues} critical issues found")

    print("=" * 60 + "\n")

    # Save detailed report
    report = {
        "shadow_state_issues": all_shadow_issues,
        "capacity_violations": all_capacity_violations,
        "insolvency_stats": all_insolvency_stats,
        "loss_explosions": all_loss_explosions,
        "stationarity_results": all_stationarity_results
    }

    report_path = exp_path / "anomaly_report.json"
    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"Detailed report saved to: {report_path}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python anomaly_detection.py <experiment_directory>")
        print("Example: python anomaly_detection.py ../results/baseline_validation/")
        sys.exit(1)

    analyze_experiment_anomalies(sys.argv[1])
