//! Insurer agent implementation
//!
//! Insurers implement two-stage pricing:
//! 1. Actuarial pricing: credibility blending + risk loading
//! 2. Underwriter markup: elasticity-based with smoothing
//!
//! Based on Owadally et al. (2018) insurance cycle model

use crate::{Event, InsurerStats, ModelConfig, Stats, DAYS_PER_YEAR};
use des::{Agent, Response};
use rand::rngs::StdRng;
use rand::SeedableRng;

pub struct Insurer {
    insurer_id: usize,
    position: f64, // Position on circular preference landscape [0, 2π)
    config: ModelConfig,

    // Financial state
    capital: f64,

    // Claims experience (EWMA)
    ewma_claim: f64,
    total_premiums: f64,
    total_claims: f64,

    // Pricing history (bounded to last 2 years for elasticity calculation)
    price_history: Vec<f64>,
    quantity_history: Vec<usize>,

    // Current year state
    current_actuarial_price: f64,
    current_market_price: f64,
    current_markup: f64,
    current_customers: usize,
    current_year_premiums: f64,
    current_year_claims: f64,

    // Historical tracking
    total_years: usize,
    years_solvent: usize,

    // Internal calculations
    recent_claim_std: f64, // Standard deviation of recent claims for risk loading
    industry_avg_claim: f64, // Industry average claim from last year

    // RNG for stochastic elements
    #[allow(dead_code)]
    rng: StdRng,
}

impl Insurer {
    pub fn new(insurer_id: usize, position: f64, config: ModelConfig, seed: u64) -> Self {
        Insurer {
            insurer_id,
            position,
            config: config.clone(),
            capital: config.initial_capital,
            ewma_claim: config.gamma_mean, // Initialize to expected claim amount
            total_premiums: 0.0,
            total_claims: 0.0,
            price_history: Vec::new(),
            quantity_history: Vec::new(),
            current_actuarial_price: 0.0,
            current_market_price: 0.0,
            current_markup: 0.0, // Start with no markup (exp(0) = 1.0)
            current_customers: 0,
            current_year_premiums: 0.0,
            current_year_claims: 0.0,
            total_years: 0,
            years_solvent: 0,
            recent_claim_std: config.gamma_std, // Initialize to expected std dev
            industry_avg_claim: config.gamma_mean, // Initialize to expected claim
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Calculate actuarial price using credibility blending
    ///
    /// Formula: P_actuarial = (z × ewma_claim + (1-z) × industry_avg) + α × σ_claims
    ///
    /// Where:
    /// - z = credibility factor (weight on own experience)
    /// - α = risk loading factor
    /// - σ_claims = standard deviation of recent claims
    fn calculate_actuarial_price(&self, industry_avg_claim: f64) -> f64 {
        let z = self.config.credibility_factor;
        let alpha = self.config.risk_loading_factor;

        // Credibility-weighted blend of own experience and industry average
        let blended_claim = z * self.ewma_claim + (1.0 - z) * industry_avg_claim;

        // Add risk loading based on claim volatility
        let risk_loading = alpha * self.recent_claim_std;

        blended_claim + risk_loading
    }

    /// Calculate optimal markup based on price elasticity
    ///
    /// Formula: m_hat = -1 / (1 + ε)  if ε < -1, else 0
    ///
    /// Then smooth: m_t = β × m_hat + (1-β) × m_{t-1}
    ///
    /// Where:
    /// - ε = arc price elasticity from last 2 years
    /// - β = underwriter smoothing parameter (CRITICAL for cycles)
    fn calculate_underwriter_markup(&mut self) -> f64 {
        let beta = self.config.underwriter_smoothing;

        // Calculate optimal markup from elasticity
        let m_hat = if let Some(elasticity) = self.calculate_price_elasticity() {
            if elasticity < -1.0 {
                // Elastic demand: optimal markup
                -1.0 / (1.0 + elasticity)
            } else {
                // Inelastic or positive elasticity: no markup
                0.0
            }
        } else {
            // No history yet: no markup
            0.0
        };

        // Smooth update: m_t = β × m_hat + (1-β) × m_{t-1}
        beta * m_hat + (1.0 - beta) * self.current_markup
    }

    /// Calculate arc price elasticity from last 2 years
    ///
    /// Formula: ε = (ΔQ/ΔP) × ((P1+P2)/(Q1+Q2))
    fn calculate_price_elasticity(&self) -> Option<f64> {
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

    /// Update EWMA claim estimate with new claim
    ///
    /// Formula: ewma_t = w × new_claim + (1-w) × ewma_{t-1}
    fn update_ewma_claim(&mut self, claim_amount: f64) {
        let w = self.config.ewma_smoothing;
        self.ewma_claim = w * claim_amount + (1.0 - w) * self.ewma_claim;
    }

    /// Update claim standard deviation estimate (simple running calculation)
    fn update_claim_std(&mut self, claim_amount: f64) {
        // Simplified: use EWMA approach for variance as well
        let w = self.config.ewma_smoothing;
        let deviation = claim_amount - self.ewma_claim;
        let variance = w * deviation.powi(2) + (1.0 - w) * self.recent_claim_std.powi(2);
        self.recent_claim_std = variance.sqrt();
    }

    /// Handle pricing request event
    fn handle_pricing_request(
        &mut self,
        year: usize,
        industry_avg_claim: f64,
    ) -> Vec<(usize, Event)> {
        // Stage 1: Actuarial pricing
        let actuarial_price = self.calculate_actuarial_price(industry_avg_claim);

        // Stage 2: Underwriter markup
        let new_markup = self.calculate_underwriter_markup();
        self.current_markup = new_markup;

        // Apply markup: market_price = actuarial_price × exp(markup)
        let market_price = actuarial_price * self.current_markup.exp();

        // Store current prices
        self.current_actuarial_price = actuarial_price;
        self.current_market_price = market_price;

        // Schedule PriceSubmitted event after pricing calculation
        let time = year * DAYS_PER_YEAR + 1; // Day 1 of year (after PricingRequest at day 0)
        vec![(
            time,
            Event::PriceSubmitted {
                year,
                insurer_id: self.insurer_id,
                actuarial_price,
                market_price,
                markup: new_markup,
            },
        )]
    }

    /// Handle market cleared event - update history and reset year counters
    fn handle_market_cleared(&mut self, _year: usize, num_customers: usize) {
        self.current_customers = num_customers;
        self.current_year_premiums = self.current_market_price * num_customers as f64;
        self.current_year_claims = 0.0; // Reset for new year

        // Collect premium
        self.capital += self.current_year_premiums;
        self.total_premiums += self.current_year_premiums;

        // Update price/quantity history (keep only last 2 years)
        self.price_history.push(self.current_market_price);
        self.quantity_history.push(num_customers);

        if self.price_history.len() > 2 {
            self.price_history.remove(0);
        }
        if self.quantity_history.len() > 2 {
            self.quantity_history.remove(0);
        }

        // Track years
        self.total_years += 1;
        if self.capital > 0.0 {
            self.years_solvent += 1;
        }
    }

    /// Handle claim occurred event
    fn handle_claim_occurred(&mut self, amount: f64) {
        // Pay claim
        self.capital -= amount;
        self.total_claims += amount;
        self.current_year_claims += amount;

        // Update EWMA estimates
        self.update_ewma_claim(amount);
        self.update_claim_std(amount);
    }

    /// Check if insurer has capacity for customers
    pub fn has_capacity(&self, num_customers: usize) -> bool {
        if !self.is_solvent() {
            return false;
        }

        let required_premium = self.current_market_price * num_customers as f64;
        let max_premium = self.capital * self.config.leverage_ratio;

        required_premium <= max_premium
    }

    fn is_solvent(&self) -> bool {
        self.capital > 0.0
    }
}

impl Agent<Event, Stats> for Insurer {
    fn act(&mut self, _current_t: usize, event: &Event) -> Response<Event, Stats> {
        match event {
            Event::PricingRequest { year, insurer_id } if *insurer_id == self.insurer_id => {
                // Use industry average from last year (stored from MarketCleared)
                let events = self.handle_pricing_request(*year, self.industry_avg_claim);
                Response::events(events)
            }

            Event::MarketCleared {
                year,
                customer_allocations,
                industry_avg_claim,
            } => {
                // Store industry average for next year's pricing
                self.industry_avg_claim = *industry_avg_claim;
                // Count customers allocated to this insurer
                let num_customers = customer_allocations
                    .iter()
                    .filter(|(_, iid)| *iid == self.insurer_id)
                    .count();

                self.handle_market_cleared(*year, num_customers);
                Response::new()
            }

            Event::ClaimOccurred {
                insurer_id, amount, ..
            } if *insurer_id == self.insurer_id => {
                self.handle_claim_occurred(*amount);
                Response::new()
            }

            Event::YearEnd { .. } => {
                // Could update industry average here, but handled in pricing request
                Response::new()
            }

            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        let loss_ratio = if self.total_premiums > 0.0 {
            self.total_claims / self.total_premiums
        } else {
            0.0
        };

        Stats::Insurer(InsurerStats {
            insurer_id: self.insurer_id,
            position: self.position,
            capital: self.capital,
            total_premiums: self.total_premiums,
            total_claims: self.total_claims,
            loss_ratio,
            current_actuarial_price: self.current_actuarial_price,
            current_market_price: self.current_market_price,
            current_markup: self.current_markup,
            num_customers: self.current_customers,
            ewma_claim: self.ewma_claim,
            price_history: self.price_history.clone(),
            quantity_history: self.quantity_history.clone(),
            total_years: self.total_years,
            years_solvent: self.years_solvent,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ModelConfig {
        ModelConfig::baseline()
    }

    #[test]
    fn test_insurer_creation() {
        let config = test_config();
        let insurer = Insurer::new(0, 1.5, config.clone(), 42);

        assert_eq!(insurer.insurer_id, 0);
        assert_eq!(insurer.position, 1.5);
        assert_eq!(insurer.capital, config.initial_capital);
        assert_eq!(insurer.ewma_claim, config.gamma_mean);
    }

    #[test]
    fn test_actuarial_pricing_credibility_blend() {
        let config = test_config();
        let insurer = Insurer::new(0, 0.0, config.clone(), 42);

        // z=0.2, industry=100, own=120
        // blended = 0.2 × 120 + 0.8 × 100 = 24 + 80 = 104
        // + risk loading: 0.001 × 10 = 0.01
        // total = 104.01
        let mut insurer_with_history = insurer;
        insurer_with_history.ewma_claim = 120.0;
        insurer_with_history.recent_claim_std = 10.0;

        let price = insurer_with_history.calculate_actuarial_price(100.0);
        assert!((price - 104.01).abs() < 0.01);
    }

    #[test]
    fn test_underwriter_markup_no_history() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        let markup = insurer.calculate_underwriter_markup();
        assert_eq!(markup, 0.0); // No history = no markup
    }

    #[test]
    fn test_underwriter_markup_with_elasticity() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        // Set up history with elastic demand (ε = -2.0)
        insurer.price_history = vec![100.0, 110.0];
        insurer.quantity_history = vec![100, 80]; // Price up 10%, quantity down 20%

        // ΔQ = -20, ΔP = 10
        // (P1+P2) = 210, (Q1+Q2) = 180
        // ε = (-20/10) × (210/180) = -2.0 × 1.167 = -2.33
        // m_hat = -1/(1-2.33) = -1/(-1.33) = 0.75
        // With β=0.3: m_t = 0.3 × 0.75 + 0.7 × 0 = 0.225

        let markup = insurer.calculate_underwriter_markup();
        assert!((markup - 0.225).abs() < 0.01);
    }

    #[test]
    fn test_ewma_claim_update() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        insurer.ewma_claim = 100.0;
        insurer.update_ewma_claim(120.0);

        // w=0.2: ewma = 0.2 × 120 + 0.8 × 100 = 24 + 80 = 104
        assert!((insurer.ewma_claim - 104.0).abs() < 0.01);
    }

    #[test]
    fn test_handle_claim_reduces_capital() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        let initial_capital = insurer.capital;
        insurer.handle_claim_occurred(500.0);

        assert_eq!(insurer.capital, initial_capital - 500.0);
        assert_eq!(insurer.total_claims, 500.0);
        assert_eq!(insurer.current_year_claims, 500.0);
    }

    #[test]
    fn test_handle_market_cleared_updates_history() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        insurer.current_market_price = 105.0;
        insurer.handle_market_cleared(1, 50);

        assert_eq!(insurer.current_customers, 50);
        assert_eq!(insurer.price_history.len(), 1);
        assert_eq!(insurer.quantity_history.len(), 1);
        assert_eq!(insurer.price_history[0], 105.0);
        assert_eq!(insurer.quantity_history[0], 50);
        assert_eq!(insurer.current_year_premiums, 105.0 * 50.0);
    }

    #[test]
    fn test_history_bounded_to_two_years() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        insurer.current_market_price = 100.0;
        insurer.handle_market_cleared(1, 50);
        insurer.current_market_price = 105.0;
        insurer.handle_market_cleared(2, 55);
        insurer.current_market_price = 110.0;
        insurer.handle_market_cleared(3, 60);

        assert_eq!(insurer.price_history.len(), 2);
        assert_eq!(insurer.quantity_history.len(), 2);
        assert_eq!(insurer.price_history[0], 105.0); // Year 2
        assert_eq!(insurer.price_history[1], 110.0); // Year 3
    }

    #[test]
    fn test_capacity_constraint() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config.clone(), 42);

        insurer.current_market_price = 100.0;
        insurer.capital = 1000.0;

        // Max premium = 1000 × 2.0 = 2000
        // Price = 100, so max customers = 2000/100 = 20
        assert!(insurer.has_capacity(20));
        assert!(!insurer.has_capacity(21));
    }

    #[test]
    fn test_insolvent_insurer_has_no_capacity() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 0.0, config, 42);

        insurer.capital = -100.0; // Insolvent
        insurer.current_market_price = 100.0;

        assert!(!insurer.has_capacity(1));
    }

    #[test]
    fn test_stats_projection() {
        let config = test_config();
        let mut insurer = Insurer::new(0, 1.5, config, 42);

        insurer.capital = 12000.0;
        insurer.total_premiums = 5000.0;
        insurer.total_claims = 4500.0;

        let stats = insurer.stats();

        match stats {
            Stats::Insurer(s) => {
                assert_eq!(s.insurer_id, 0);
                assert_eq!(s.position, 1.5);
                assert_eq!(s.capital, 12000.0);
                assert_eq!(s.total_premiums, 5000.0);
                assert_eq!(s.total_claims, 4500.0);
                assert!((s.loss_ratio - 0.9).abs() < 0.01);
            }
            _ => panic!("Expected Insurer stats"),
        }
    }
}
