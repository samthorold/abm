use crate::{CoordinatorStats, Transaction};
use std::collections::HashMap;

/// Results from a single trading period
#[derive(Debug, Clone)]
pub struct PeriodResults {
    pub period: usize,
    pub num_transactions: usize,
    pub efficiency: f64,
    pub price_rmsd: f64,
    pub convergence_slope: Option<f64>,
    pub convergence_r_squared: Option<f64>,
    pub total_surplus: i32,
    pub max_surplus: i32,
}

impl PeriodResults {
    pub fn from_coordinator_stats(stats: &CoordinatorStats) -> Self {
        let (slope, r_squared) =
            calculate_convergence(&stats.transactions, stats.equilibrium_price);

        PeriodResults {
            period: stats.period,
            num_transactions: stats.num_transactions(),
            efficiency: stats.efficiency(),
            price_rmsd: stats.price_rmsd(),
            convergence_slope: slope,
            convergence_r_squared: r_squared,
            total_surplus: stats.total_surplus,
            max_surplus: stats.max_possible_surplus,
        }
    }
}

/// Results from a complete session (multiple periods)
#[derive(Debug, Clone)]
pub struct SessionResults {
    pub session_id: usize,
    pub periods: Vec<PeriodResults>,
    pub mean_efficiency: f64,
    pub mean_convergence_slope: f64,
    pub total_transactions: usize,
}

impl SessionResults {
    pub fn from_periods(session_id: usize, periods: Vec<PeriodResults>) -> Self {
        let mean_efficiency =
            periods.iter().map(|p| p.efficiency).sum::<f64>() / periods.len() as f64;

        let slopes: Vec<f64> = periods.iter().filter_map(|p| p.convergence_slope).collect();
        let mean_convergence_slope = if slopes.is_empty() {
            0.0
        } else {
            slopes.iter().sum::<f64>() / slopes.len() as f64
        };

        let total_transactions = periods.iter().map(|p| p.num_transactions).sum();

        SessionResults {
            session_id,
            periods,
            mean_efficiency,
            mean_convergence_slope,
            total_transactions,
        }
    }
}

/// Aggregate results across multiple sessions
#[derive(Debug, Clone)]
pub struct AggregateResults {
    pub num_sessions: usize,
    pub mean_efficiency: f64,
    pub std_efficiency: f64,
    pub min_efficiency: f64,
    pub max_efficiency: f64,
    pub mean_convergence_slope: f64,
    pub std_convergence_slope: f64,
    pub mean_transactions_per_session: f64,
}

impl AggregateResults {
    pub fn from_sessions(sessions: &[SessionResults]) -> Self {
        let efficiencies: Vec<f64> = sessions.iter().map(|s| s.mean_efficiency).collect();
        let slopes: Vec<f64> = sessions.iter().map(|s| s.mean_convergence_slope).collect();
        let transactions: Vec<usize> = sessions.iter().map(|s| s.total_transactions).collect();

        let mean_efficiency = mean(&efficiencies);
        let std_efficiency = std_dev(&efficiencies, mean_efficiency);
        let min_efficiency = efficiencies.iter().copied().fold(f64::INFINITY, f64::min);
        let max_efficiency = efficiencies
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let mean_convergence_slope = mean(&slopes);
        let std_convergence_slope = std_dev(&slopes, mean_convergence_slope);

        let mean_transactions_per_session =
            transactions.iter().sum::<usize>() as f64 / transactions.len() as f64;

        AggregateResults {
            num_sessions: sessions.len(),
            mean_efficiency,
            std_efficiency,
            min_efficiency,
            max_efficiency,
            mean_convergence_slope,
            std_convergence_slope,
            mean_transactions_per_session,
        }
    }

    pub fn print_summary(&self, market_name: &str, trader_type: &str) {
        println!("\n{} - {}", market_name, trader_type);
        println!("  Sessions: {}", self.num_sessions);
        println!(
            "  Efficiency: {:.2}% (±{:.2}%) [{:.2}%, {:.2}%]",
            self.mean_efficiency, self.std_efficiency, self.min_efficiency, self.max_efficiency
        );
        println!(
            "  Convergence slope: {:.4} (±{:.4})",
            self.mean_convergence_slope, self.std_convergence_slope
        );
        println!(
            "  Avg transactions: {:.1}",
            self.mean_transactions_per_session
        );
    }
}

/// Calculate price convergence using linear regression
/// Returns (slope, R²) where negative slope indicates convergence
/// Returns None for both if insufficient data points
pub fn calculate_convergence(
    transactions: &[Transaction],
    equilibrium_price: usize,
) -> (Option<f64>, Option<f64>) {
    if transactions.len() < 2 {
        return (None, None);
    }

    // Prepare data: x = sequence number, y = price deviation from equilibrium
    let x_values: Vec<f64> = transactions.iter().map(|t| t.sequence as f64).collect();
    let y_values: Vec<f64> = transactions
        .iter()
        .map(|t| t.price_deviation(equilibrium_price) as f64)
        .collect();

    // Calculate means
    let x_mean = mean(&x_values);
    let y_mean = mean(&y_values);

    // Calculate slope (β) and intercept (α)
    let numerator: f64 = x_values
        .iter()
        .zip(y_values.iter())
        .map(|(x, y)| (x - x_mean) * (y - y_mean))
        .sum();

    let denominator: f64 = x_values.iter().map(|x| (x - x_mean).powi(2)).sum();

    if denominator == 0.0 {
        return (None, None);
    }

    let slope = numerator / denominator;

    // Calculate R²
    let ss_tot: f64 = y_values.iter().map(|y| (y - y_mean).powi(2)).sum();
    let ss_res: f64 = x_values
        .iter()
        .zip(y_values.iter())
        .map(|(x, y)| {
            let y_pred = slope * (x - x_mean) + y_mean;
            (y - y_pred).powi(2)
        })
        .sum();

    let r_squared = if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - (ss_res / ss_tot)
    };

    (Some(slope), Some(r_squared))
}

/// Calculate profit dispersion (RMSD between actual and equilibrium profits)
/// equilibrium_profits should be the theoretical profits at competitive equilibrium
pub fn calculate_profit_dispersion(
    actual_profits: &HashMap<usize, i32>,
    equilibrium_profits: &HashMap<usize, i32>,
) -> f64 {
    if actual_profits.is_empty() {
        return 0.0;
    }

    let sum_sq_diff: f64 = actual_profits
        .iter()
        .map(|(trader_id, &actual)| {
            let equilibrium = equilibrium_profits.get(trader_id).copied().unwrap_or(0);
            let diff = actual - equilibrium;
            (diff * diff) as f64
        })
        .sum();

    (sum_sq_diff / actual_profits.len() as f64).sqrt()
}

/// Calculate mean of a slice
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Calculate standard deviation
fn std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() <= 1 {
        return 0.0;
    }

    let variance =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convergence_negative_slope() {
        // Prices converging toward equilibrium of 100
        let transactions = vec![
            Transaction {
                sequence: 0,
                buyer_id: 0,
                seller_id: 1,
                price: 120,
                buyer_value: 150,
                seller_cost: 80,
            },
            Transaction {
                sequence: 1,
                buyer_id: 0,
                seller_id: 1,
                price: 110,
                buyer_value: 150,
                seller_cost: 80,
            },
            Transaction {
                sequence: 2,
                buyer_id: 0,
                seller_id: 1,
                price: 105,
                buyer_value: 150,
                seller_cost: 80,
            },
            Transaction {
                sequence: 3,
                buyer_id: 0,
                seller_id: 1,
                price: 102,
                buyer_value: 150,
                seller_cost: 80,
            },
        ];

        let (slope, r_squared) = calculate_convergence(&transactions, 100);

        assert!(slope.is_some());
        assert!(r_squared.is_some());

        // Slope should be negative (prices moving toward equilibrium)
        assert!(
            slope.unwrap() < 0.0,
            "Slope should be negative for convergence"
        );

        // R² should be high for this linear relationship
        assert!(
            r_squared.unwrap() > 0.8,
            "R² should be high for strong linear trend"
        );
    }

    #[test]
    fn test_convergence_insufficient_data() {
        let transactions = vec![Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 100,
            buyer_value: 150,
            seller_cost: 80,
        }];

        let (slope, r_squared) = calculate_convergence(&transactions, 100);
        assert!(slope.is_none());
        assert!(r_squared.is_none());
    }

    #[test]
    fn test_profit_dispersion() {
        let mut actual = HashMap::new();
        actual.insert(0, 100);
        actual.insert(1, 150);
        actual.insert(2, 80);

        let mut equilibrium = HashMap::new();
        equilibrium.insert(0, 90);
        equilibrium.insert(1, 140);
        equilibrium.insert(2, 90);

        let rmsd = calculate_profit_dispersion(&actual, &equilibrium);

        // RMSD = sqrt(((100-90)^2 + (150-140)^2 + (80-90)^2) / 3)
        // = sqrt((100 + 100 + 100) / 3) = sqrt(100) = 10
        assert!((rmsd - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_mean_and_std_dev() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let m = mean(&values);
        assert!((m - 3.0).abs() < 0.01);

        let std = std_dev(&values, m);
        // Sample std dev of 1,2,3,4,5 is sqrt(2.5) ≈ 1.58
        assert!((std - 1.58).abs() < 0.01);
    }

    #[test]
    fn test_surplus_calculation_with_marginal_units() {
        // Transaction with very small surplus
        let txn = Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 131,
            buyer_value: 131,
            seller_cost: 130,
        };

        let surplus = txn.total_surplus();
        // (131-131) + (131-130) = 0 + 1 = 1
        assert_eq!(
            surplus, 1,
            "Small positive surplus should be calculated correctly"
        );
    }

    #[test]
    fn test_surplus_calculation_with_extramarginal_trade() {
        // Welfare-destroying trade (ZI-U can make these)
        let txn = Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 105,
            buyer_value: 100,
            seller_cost: 110,
        };

        let surplus = txn.total_surplus();
        // (100-105) + (105-110) = -5 + -5 = -10
        assert_eq!(surplus, -10, "Negative surplus for losing trade");
    }

    #[test]
    fn test_efficiency_when_no_transactions() {
        use crate::CoordinatorStats;

        let stats = CoordinatorStats::new(0, 1, 100, 10, 500);
        // No transactions recorded
        assert_eq!(stats.num_transactions(), 0);
        assert_eq!(stats.efficiency(), 0.0);
    }

    #[test]
    fn test_efficiency_with_partial_equilibrium() {
        use crate::CoordinatorStats;

        let mut stats = CoordinatorStats::new(0, 1, 100, 10, 500);

        // Add 3 transactions with surplus of 50 each
        for i in 0..3 {
            stats.transactions.push(Transaction {
                sequence: i,
                buyer_id: 0,
                seller_id: 1,
                price: 100,
                buyer_value: 125,
                seller_cost: 75,
            });
            stats.total_surplus += 50;
        }

        let efficiency = stats.efficiency();
        // 3 * 50 / 500 = 150 / 500 = 30%
        assert!((efficiency - 30.0).abs() < 0.01);
        assert!(efficiency > 0.0 && efficiency < 100.0);
    }

    #[test]
    fn test_price_rmsd_with_no_transactions() {
        use crate::CoordinatorStats;

        let stats = CoordinatorStats::new(0, 1, 100, 10, 500);
        assert_eq!(stats.price_rmsd(), 0.0);
    }

    #[test]
    fn test_convergence_with_single_transaction() {
        let transactions = vec![Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 100,
            buyer_value: 120,
            seller_cost: 80,
        }];

        let (slope, r_squared) = calculate_convergence(&transactions, 100);
        // Not enough data points for regression
        assert!(slope.is_none());
        assert!(r_squared.is_none());
    }

    #[test]
    fn test_transaction_price_deviation() {
        let txn = Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 110,
            buyer_value: 120,
            seller_cost: 90,
        };

        assert_eq!(txn.price_deviation(100), 10);
        assert_eq!(txn.price_deviation(110), 0);
        assert_eq!(txn.price_deviation(120), -10);
    }
}
