use crate::SystemStats;

/// Compute realized shortfall (expected shortfall / CVaR) from a series of returns
///
/// Realized shortfall is the average loss in the worst q fraction of returns.
/// This is a coherent risk measure used in Basel III.
///
/// Arguments:
/// - returns: Vector of log returns
/// - q: Quantile level (e.g., 0.05 for 5% worst returns)
///
/// Returns: Average loss magnitude in the tail (positive value = loss)
pub fn realized_shortfall(returns: &[f64], q: f64) -> f64 {
    if returns.is_empty() || q <= 0.0 || q >= 1.0 {
        return 0.0;
    }

    // Filter out NaN values
    let mut sorted: Vec<f64> = returns.iter().filter(|x| x.is_finite()).copied().collect();
    if sorted.is_empty() {
        return 0.0;
    }

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let cutoff_idx = ((q * sorted.len() as f64).floor() as usize).max(1);
    let worst_returns = &sorted[..cutoff_idx];

    if worst_returns.is_empty() {
        return 0.0;
    }

    // Return negative of average (so losses are positive)
    -worst_returns.iter().sum::<f64>() / worst_returns.len() as f64
}

/// Compute Value-at-Risk from a series of returns
///
/// VaR is the threshold loss at quantile q.
///
/// Arguments:
/// - returns: Vector of log returns
/// - q: Quantile level (e.g., 0.05 for 5% VaR)
///
/// Returns: Loss threshold (positive value = loss)
pub fn value_at_risk(returns: &[f64], q: f64) -> f64 {
    if returns.is_empty() || q <= 0.0 || q >= 1.0 {
        return 0.0;
    }

    // Filter out NaN values
    let mut sorted: Vec<f64> = returns.iter().filter(|x| x.is_finite()).copied().collect();
    if sorted.is_empty() {
        return 0.0;
    }

    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let idx = ((q * sorted.len() as f64).floor() as usize).min(sorted.len() - 1);
    -sorted[idx]
}

/// Compute standard deviation of a series
pub fn std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    variance.sqrt()
}

/// Compute mean of a series
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Compute range (max - min) of a series
pub fn range(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    max - min
}

/// Augment SystemStats with computed risk metrics
pub fn compute_risk_metrics(stats: &mut SystemStats, quantile: f64) {
    if !stats.equity_return_history.is_empty() {
        stats.realized_shortfall = Some(realized_shortfall(&stats.equity_return_history, quantile));
    }
}

/// Summary of stability analysis
#[derive(Debug, Clone)]
pub struct StabilityAnalysis {
    /// Price range over simulation
    pub price_range: f64,
    /// Leverage range over simulation
    pub leverage_range: f64,
    /// Price coefficient of variation (std/mean)
    pub price_cv: f64,
    /// Leverage coefficient of variation
    pub leverage_cv: f64,
    /// Whether price variance suggests convergence
    pub is_converged: bool,
    /// Whether leverage range suggests cycles
    pub has_cycles: bool,
    /// 5% VaR of equity returns
    pub var_5: f64,
    /// 5% CVaR (realized shortfall) of equity returns
    pub cvar_5: f64,
}

impl StabilityAnalysis {
    /// Analyze stability from system stats
    pub fn from_stats(stats: &SystemStats) -> Self {
        let price_range = range(&stats.price_history);
        let leverage_range = range(&stats.leverage_history);

        let price_cv = if stats.price_mean.abs() > 1e-10 {
            stats.price_std / stats.price_mean
        } else {
            0.0
        };

        let leverage_cv = if stats.leverage_mean.abs() > 1e-10 {
            stats.leverage_std / stats.leverage_mean
        } else {
            0.0
        };

        let is_converged = stats.price_std < 0.1;
        let has_cycles = leverage_range > 1.0;

        let var_5 = value_at_risk(&stats.equity_return_history, 0.05);
        let cvar_5 = realized_shortfall(&stats.equity_return_history, 0.05);

        StabilityAnalysis {
            price_range,
            leverage_range,
            price_cv,
            leverage_cv,
            is_converged,
            has_cycles,
            var_5,
            cvar_5,
        }
    }

    /// Print summary
    pub fn print_summary(&self) {
        println!("Stability Analysis:");
        println!("  Price range: {:.2}", self.price_range);
        println!("  Leverage range: {:.2}", self.leverage_range);
        println!("  Price CV: {:.4}", self.price_cv);
        println!("  Leverage CV: {:.4}", self.leverage_cv);
        println!(
            "  Converged: {}, Has cycles: {}",
            self.is_converged, self.has_cycles
        );
        println!("  5% VaR: {:.4}", self.var_5);
        println!("  5% CVaR: {:.4}", self.cvar_5);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realized_shortfall_basic() {
        // Returns: -5, -3, -1, 1, 3, 5
        let returns = vec![-5.0, -3.0, -1.0, 1.0, 3.0, 5.0];

        // 50% tail: worst 3 returns are -5, -3, -1
        // Average = (-5 + -3 + -1) / 3 = -3
        // Shortfall = -(-3) = 3
        let shortfall = realized_shortfall(&returns, 0.5);
        assert!((shortfall - 3.0).abs() < 0.01);
    }

    #[test]
    fn realized_shortfall_empty() {
        let returns: Vec<f64> = vec![];
        assert_eq!(realized_shortfall(&returns, 0.05), 0.0);
    }

    #[test]
    fn var_basic() {
        let returns = vec![-5.0, -3.0, -1.0, 1.0, 3.0, 5.0];

        // 33% VaR: return at ~33% quantile
        // Sorted: -5, -3, -1, 1, 3, 5
        // 33% of 6 = 2, so index 2, value = -1
        // VaR = -(-1) = 1
        let var = value_at_risk(&returns, 0.34);
        assert!((var - 1.0).abs() < 0.01);

        // 16% VaR: worst ~16%
        // 16% of 6 = 1, so index 1, value = -3
        // VaR = -(-3) = 3
        let var_16 = value_at_risk(&returns, 0.17);
        assert!((var_16 - 3.0).abs() < 0.01);
    }

    #[test]
    fn std_dev_basic() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        // Mean = 5, Variance = 4, StdDev = 2
        assert!((sd - 2.0).abs() < 0.01);
    }

    #[test]
    fn range_basic() {
        let values = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0];
        let r = range(&values);
        assert!((r - 8.0).abs() < 0.01); // 9 - 1 = 8
    }
}
