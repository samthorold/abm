//! Insurance Industry Complex Social System ABM
//!
//! This module implements the Owadally et al. (2018) paper demonstrating how simple
//! individual-level behaviors generate complex industry-wide underwriting cycles.
//!
//! Key agents:
//! - Insurer: Two-stage pricing (actuarial + underwriter markup)
//! - MarketCoordinator: Orchestrates annual cycle, allocates customers
//! - ClaimGenerator: Generates stochastic claims
//!
//! Expected outcomes:
//! - Endogenous cycles with ~5.9 year period
//! - Loss ratio oscillations around mean ≈ 1.0
//! - β parameter controls cycle stability vs volatility

pub mod claim_generator;
pub mod helpers;
pub mod insurer;
pub mod market_coordinator;

/// All possible events in the insurance cycle simulation
#[derive(Debug, Clone)]
pub enum Event {
    /// Start of a new year - coordinator initiates pricing
    YearStart { year: usize },

    /// Coordinator requests price from specific insurer
    PricingRequest { year: usize, insurer_id: usize },

    /// Insurer submits pricing response
    PriceSubmitted {
        year: usize,
        insurer_id: usize,
        actuarial_price: f64,
        market_price: f64,
        markup: f64,
    },

    /// Market clearing complete - customers allocated to insurers
    MarketCleared {
        year: usize,
        customer_allocations: Vec<(usize, usize)>, // (customer_id, insurer_id)
        industry_avg_claim: f64,
    },

    /// Individual claim occurs
    ClaimOccurred {
        year: usize,
        customer_id: usize,
        insurer_id: usize,
        amount: f64,
    },

    /// End of year - industry statistics broadcast
    YearEnd {
        year: usize,
        industry_avg_claim: f64,
        industry_loss_ratio: f64,
    },
}

/// Observable state for an individual insurer
#[derive(Debug, Clone)]
pub struct InsurerStats {
    // Identity
    pub insurer_id: usize,
    pub position: f64, // Position on circular preference landscape [0, 2π)

    // Financial state (current)
    pub capital: f64,
    pub total_premiums: f64,
    pub total_claims: f64,
    pub loss_ratio: f64, // claims / premiums (if premiums > 0)

    // Pricing state (current)
    pub current_actuarial_price: f64,
    pub current_market_price: f64,
    pub current_markup: f64,
    pub num_customers: usize,

    // Historical data (for calculations)
    pub ewma_claim: f64,
    pub price_history: Vec<f64>,      // Last 2 years
    pub quantity_history: Vec<usize>, // Last 2 years

    // Cumulative metrics
    pub total_years: usize,
    pub years_solvent: usize,
}

impl InsurerStats {
    /// Check if insurer is solvent
    pub fn is_solvent(&self) -> bool {
        self.capital > 0.0
    }

    /// Calculate available capacity based on capital and leverage
    pub fn capacity(&self, leverage_ratio: f64) -> f64 {
        if !self.is_solvent() {
            return 0.0;
        }
        self.capital * leverage_ratio
    }

    /// Calculate arc price elasticity from last 2 years
    /// ε = (ΔQ/ΔP) × ((P1+P2)/(Q1+Q2))
    pub fn price_elasticity(&self) -> Option<f64> {
        if self.price_history.len() < 2 || self.quantity_history.len() < 2 {
            return None;
        }

        let p1 = self.price_history[self.price_history.len() - 2];
        let p2 = self.price_history[self.price_history.len() - 1];
        let q1 = self.quantity_history[self.quantity_history.len() - 2] as f64;
        let q2 = self.quantity_history[self.quantity_history.len() - 1] as f64;

        let delta_p = p2 - p1;
        let delta_q = q2 - q1;

        // Avoid division by zero
        if delta_p.abs() < 1e-10 || (q1 + q2).abs() < 1e-10 {
            return None;
        }

        let elasticity = (delta_q / delta_p) * ((p1 + p2) / (q1 + q2));
        Some(elasticity)
    }
}

/// Observable state for the overall market
#[derive(Debug, Clone)]
pub struct MarketStats {
    pub year: usize,

    // Industry aggregates (current year)
    pub total_premiums: f64,
    pub total_claims: f64,
    pub industry_loss_ratio: f64,
    pub industry_avg_claim: f64,

    // Market structure (current year)
    pub num_solvent_insurers: usize,
    pub total_insurers: usize,
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,

    // Time series (full history for cycle detection)
    pub loss_ratio_history: Vec<f64>,
    pub avg_claim_history: Vec<f64>,
}

impl MarketStats {
    /// Detect if loss ratio exhibits cyclical behavior
    /// Simple detection: check for sign changes in first differences
    pub fn has_cycles(&self) -> bool {
        if self.loss_ratio_history.len() < 10 {
            return false;
        }

        // Calculate first differences
        let mut sign_changes = 0;
        let mut prev_diff: Option<f64> = None;

        for i in 1..self.loss_ratio_history.len() {
            let diff = self.loss_ratio_history[i] - self.loss_ratio_history[i - 1];

            if let Some(prev) = prev_diff {
                // Sign change indicates turning point
                if (diff > 0.0 && prev < 0.0) || (diff < 0.0 && prev > 0.0) {
                    sign_changes += 1;
                }
            }
            prev_diff = Some(diff);
        }

        // Expect at least 3 sign changes (1.5 cycles) in data
        sign_changes >= 3
    }

    /// Estimate cycle period from peak-to-peak intervals
    /// Returns None if insufficient data or no cycles detected
    pub fn cycle_period(&self) -> Option<f64> {
        if self.loss_ratio_history.len() < 20 {
            return None;
        }

        // Find local maxima (peaks)
        let mut peaks = Vec::new();
        for i in 1..self.loss_ratio_history.len() - 1 {
            let prev = self.loss_ratio_history[i - 1];
            let curr = self.loss_ratio_history[i];
            let next = self.loss_ratio_history[i + 1];

            if curr > prev && curr > next {
                peaks.push(i);
            }
        }

        if peaks.len() < 2 {
            return None;
        }

        // Calculate average interval between peaks
        let intervals: Vec<f64> = peaks.windows(2).map(|w| (w[1] - w[0]) as f64).collect();

        let avg_period = intervals.iter().sum::<f64>() / intervals.len() as f64;
        Some(avg_period)
    }

    /// Calculate mean of loss ratio history
    pub fn mean_loss_ratio(&self) -> f64 {
        if self.loss_ratio_history.is_empty() {
            return 0.0;
        }
        self.loss_ratio_history.iter().sum::<f64>() / self.loss_ratio_history.len() as f64
    }

    /// Calculate standard deviation of loss ratio history
    pub fn std_loss_ratio(&self) -> f64 {
        if self.loss_ratio_history.len() < 2 {
            return 0.0;
        }

        let mean = self.mean_loss_ratio();
        let variance = self
            .loss_ratio_history
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / self.loss_ratio_history.len() as f64;

        variance.sqrt()
    }
}

/// Unified stats enum for all agents
#[derive(Debug, Clone)]
pub enum Stats {
    Insurer(InsurerStats),
    Market(MarketStats),
    ClaimGenerator, // No observable state for claim generator
}

/// Model configuration parameters
#[derive(Debug, Clone)]
pub struct ModelConfig {
    // Critical behavioral parameters (affect cycle dynamics)
    pub risk_loading_factor: f64,   // α - actuarial risk loading
    pub underwriter_smoothing: f64, // β - markup smoothing (CRITICAL: controls cycles)
    pub distance_cost: f64,         // γ - customer preference sensitivity

    // Actuarial parameters
    pub credibility_factor: f64, // z - weight on own experience vs industry
    pub ewma_smoothing: f64,     // w - EWMA decay for claim history

    // Claims distribution (Gamma)
    pub claim_frequency: f64, // Bernoulli parameter (usually 1.0 = certain)
    pub gamma_mean: f64,      // μ - mean claim amount
    pub gamma_std: f64,       // σ - std dev of claim amount

    // Market structure
    pub num_insurers: usize,  // N
    pub num_customers: usize, // M
    pub initial_capital: f64, // Starting capital per insurer
    pub leverage_ratio: f64,  // Max premium = capital × leverage
}

impl ModelConfig {
    /// Baseline configuration from paper (generates ~5.9 year cycles)
    pub fn baseline() -> Self {
        ModelConfig {
            risk_loading_factor: 0.001,
            underwriter_smoothing: 0.3, // CRITICAL parameter
            distance_cost: 0.08,
            credibility_factor: 0.2,
            ewma_smoothing: 0.2,
            claim_frequency: 1.0,
            gamma_mean: 100.0,
            gamma_std: 10.0,
            num_insurers: 20,
            num_customers: 1000,
            initial_capital: 10000.0,
            leverage_ratio: 2.0,
        }
    }

    /// Low beta configuration (more stable cycles, high autocorrelation)
    pub fn low_beta() -> Self {
        let mut config = Self::baseline();
        config.underwriter_smoothing = 0.2;
        config
    }

    /// High beta configuration (higher volatility, weaker cycles)
    pub fn high_beta() -> Self {
        let mut config = Self::baseline();
        config.underwriter_smoothing = 0.6;
        config
    }

    /// White noise configuration (β=1.0, no smoothing, cycles disappear)
    pub fn white_noise() -> Self {
        let mut config = Self::baseline();
        config.underwriter_smoothing = 1.0;
        config
    }

    /// Gamma distribution shape parameter (k)
    /// For Gamma: k = (μ/σ)²
    pub fn gamma_shape(&self) -> f64 {
        (self.gamma_mean / self.gamma_std).powi(2)
    }

    /// Gamma distribution scale parameter (θ)
    /// For Gamma: θ = σ²/μ
    pub fn gamma_scale(&self) -> f64 {
        self.gamma_std.powi(2) / self.gamma_mean
    }
}

/// Customer data structure (not an agent - simple decision-making)
#[derive(Debug, Clone)]
pub struct Customer {
    pub id: usize,
    pub position: f64, // Position on circular preference landscape [0, 2π)
}

impl Customer {
    pub fn new(id: usize, position: f64) -> Self {
        Customer { id, position }
    }

    /// Calculate total cost for this customer from given insurer
    /// Total cost = price + γ × distance
    pub fn total_cost(&self, insurer_position: f64, price: f64, gamma: f64) -> f64 {
        let distance = helpers::circular_distance(self.position, insurer_position);
        price + gamma * distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_insurer_stats_solvency() {
        let mut stats = InsurerStats {
            insurer_id: 0,
            position: 0.0,
            capital: 1000.0,
            total_premiums: 0.0,
            total_claims: 0.0,
            loss_ratio: 0.0,
            current_actuarial_price: 0.0,
            current_market_price: 0.0,
            current_markup: 0.0,
            num_customers: 0,
            ewma_claim: 0.0,
            price_history: vec![],
            quantity_history: vec![],
            total_years: 0,
            years_solvent: 0,
        };

        assert!(stats.is_solvent());
        assert_eq!(stats.capacity(2.0), 2000.0);

        stats.capital = -100.0;
        assert!(!stats.is_solvent());
        assert_eq!(stats.capacity(2.0), 0.0);
    }

    #[test]
    fn test_price_elasticity_calculation() {
        let stats = InsurerStats {
            insurer_id: 0,
            position: 0.0,
            capital: 1000.0,
            total_premiums: 0.0,
            total_claims: 0.0,
            loss_ratio: 0.0,
            current_actuarial_price: 0.0,
            current_market_price: 0.0,
            current_markup: 0.0,
            num_customers: 0,
            ewma_claim: 0.0,
            price_history: vec![100.0, 110.0],
            quantity_history: vec![50, 45],
            total_years: 0,
            years_solvent: 0,
        };

        let elasticity = stats.price_elasticity().unwrap();

        // ΔQ = 45 - 50 = -5
        // ΔP = 110 - 100 = 10
        // (P1+P2) = 210
        // (Q1+Q2) = 95
        // ε = (-5/10) × (210/95) = -0.5 × 2.21 ≈ -1.105
        assert!((elasticity + 1.105).abs() < 0.01);
    }

    #[test]
    fn test_price_elasticity_insufficient_data() {
        let stats = InsurerStats {
            insurer_id: 0,
            position: 0.0,
            capital: 1000.0,
            total_premiums: 0.0,
            total_claims: 0.0,
            loss_ratio: 0.0,
            current_actuarial_price: 0.0,
            current_market_price: 0.0,
            current_markup: 0.0,
            num_customers: 0,
            ewma_claim: 0.0,
            price_history: vec![100.0],
            quantity_history: vec![50],
            total_years: 0,
            years_solvent: 0,
        };

        assert!(stats.price_elasticity().is_none());
    }

    #[test]
    fn test_market_stats_mean_std() {
        let stats = MarketStats {
            year: 10,
            total_premiums: 0.0,
            total_claims: 0.0,
            industry_loss_ratio: 0.0,
            industry_avg_claim: 0.0,
            num_solvent_insurers: 20,
            total_insurers: 20,
            min_price: 0.0,
            max_price: 0.0,
            avg_price: 0.0,
            loss_ratio_history: vec![0.9, 1.0, 1.1, 1.0, 0.9],
            avg_claim_history: vec![],
        };

        assert!((stats.mean_loss_ratio() - 0.98).abs() < 1e-10);
        assert!((stats.std_loss_ratio() - 0.0748).abs() < 0.001);
    }

    #[test]
    fn test_model_config_gamma_params() {
        let config = ModelConfig::baseline();

        // μ = 100, σ = 10
        // k = (μ/σ)² = 100
        // θ = σ²/μ = 1.0
        assert_eq!(config.gamma_shape(), 100.0);
        assert_eq!(config.gamma_scale(), 1.0);
    }

    #[test]
    fn test_customer_total_cost() {
        let customer = Customer::new(0, 0.0);

        // Insurer at same position, zero distance cost
        let cost1 = customer.total_cost(0.0, 100.0, 0.08);
        assert_eq!(cost1, 100.0);

        // Insurer at π (maximum distance on circle)
        let cost2 = customer.total_cost(PI, 100.0, 0.08);
        assert!((cost2 - (100.0 + 0.08 * PI)).abs() < 0.01);
    }
}
