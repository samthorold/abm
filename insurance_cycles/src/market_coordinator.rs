//! Market coordinator agent implementation
//!
//! Orchestrates annual cycle:
//! - YearStart → PricingRequest (×20) → PriceSubmitted (×20) → MarketCleared → YearEnd
//!
//! Responsibilities:
//! - Collect prices from all insurers
//! - Execute market clearing (allocate customers to insurers)
//! - Aggregate industry statistics
//! - Track time series for cycle detection

use crate::helpers::circular_distance;
use crate::{Customer, Event, MarketStats, ModelConfig, Stats};
use des::{Agent, Response};
use std::collections::HashMap;

pub struct MarketCoordinator {
    config: ModelConfig,
    customers: Vec<Customer>,
    insurer_positions: HashMap<usize, f64>, // insurer_id -> position

    // Current year state
    current_year: usize,
    waiting_for_prices: bool,
    prices_received: HashMap<usize, f64>, // insurer_id -> market_price

    // Industry tracking
    industry_total_premiums: f64,
    industry_total_claims: f64,
    last_year_avg_claim: f64,

    // Time series for cycle detection
    loss_ratio_history: Vec<f64>,
    avg_claim_history: Vec<f64>,

    // Stats
    stats: MarketStats,
}

impl MarketCoordinator {
    pub fn new(
        config: ModelConfig,
        customers: Vec<Customer>,
        insurer_positions: HashMap<usize, f64>,
    ) -> Self {
        MarketCoordinator {
            config,
            customers,
            insurer_positions,
            current_year: 0,
            waiting_for_prices: false,
            prices_received: HashMap::new(),
            industry_total_premiums: 0.0,
            industry_total_claims: 0.0,
            last_year_avg_claim: 100.0, // Initialize to expected claim amount
            loss_ratio_history: Vec::new(),
            avg_claim_history: Vec::new(),
            stats: MarketStats {
                year: 0,
                total_premiums: 0.0,
                total_claims: 0.0,
                industry_loss_ratio: 0.0,
                industry_avg_claim: 0.0,
                num_solvent_insurers: 0,
                total_insurers: 0,
                min_price: 0.0,
                max_price: 0.0,
                avg_price: 0.0,
                loss_ratio_history: Vec::new(),
                avg_claim_history: Vec::new(),
            },
        }
    }

    /// Start a new year - request prices from all insurers
    fn start_year(&mut self, year: usize) -> Vec<(usize, Event)> {
        self.current_year = year;
        self.waiting_for_prices = true;
        self.prices_received.clear();

        // Request prices from all insurers
        let time = year * 365;
        let mut events = Vec::new();

        for insurer_id in 0..self.config.num_insurers {
            events.push((time, Event::PricingRequest { year, insurer_id }));
        }

        events
    }

    /// Receive price from an insurer
    fn receive_price(&mut self, insurer_id: usize, market_price: f64) {
        self.prices_received.insert(insurer_id, market_price);
    }

    /// Check if all prices have been received
    fn all_prices_received(&self) -> bool {
        self.prices_received.len() == self.config.num_insurers
    }

    /// Execute market clearing algorithm
    ///
    /// Greedy allocation: for each customer, find the insurer with lowest total cost
    /// Total cost = price + γ × circular_distance(customer, insurer)
    ///
    /// Returns: Vec<(customer_id, insurer_id)>
    fn clear_market(&mut self) -> Vec<(usize, usize)> {
        let mut allocations = Vec::new();
        let gamma = self.config.distance_cost;

        for customer in &self.customers {
            // Calculate total cost for each insurer
            let mut best_insurer: Option<usize> = None;
            let mut best_cost = f64::INFINITY;

            for (&insurer_id, &price) in &self.prices_received {
                let insurer_pos = self.insurer_positions[&insurer_id];
                let distance = circular_distance(customer.position, insurer_pos);
                let total_cost = price + gamma * distance;

                if total_cost < best_cost {
                    best_cost = total_cost;
                    best_insurer = Some(insurer_id);
                }
            }

            // Allocate customer to best insurer
            if let Some(insurer_id) = best_insurer {
                allocations.push((customer.id, insurer_id));
            }
        }

        allocations
    }

    /// Calculate industry average claim per customer from last year
    fn calculate_industry_avg_claim(&self) -> f64 {
        if self.customers.is_empty() {
            return self.config.gamma_mean;
        }

        // Use last year's average if available
        if self.industry_total_claims > 0.0 {
            self.industry_total_claims / self.customers.len() as f64
        } else {
            self.config.gamma_mean // Default to expected value
        }
    }

    /// Aggregate industry statistics from pricing round
    fn aggregate_statistics(&mut self, _allocations: &[(usize, usize)]) {
        let industry_avg_claim = self.calculate_industry_avg_claim();

        // Calculate price statistics
        let prices: Vec<f64> = self.prices_received.values().copied().collect();
        let min_price = prices.iter().copied().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let avg_price = prices.iter().sum::<f64>() / prices.len() as f64;

        // Calculate industry loss ratio
        let industry_loss_ratio = if self.industry_total_premiums > 0.0 {
            self.industry_total_claims / self.industry_total_premiums
        } else {
            0.0
        };

        // Update time series
        if industry_loss_ratio > 0.0 {
            self.loss_ratio_history.push(industry_loss_ratio);
        }
        self.avg_claim_history.push(industry_avg_claim);

        // Update stats
        self.stats = MarketStats {
            year: self.current_year,
            total_premiums: self.industry_total_premiums,
            total_claims: self.industry_total_claims,
            industry_loss_ratio,
            industry_avg_claim,
            num_solvent_insurers: self.prices_received.len(),
            total_insurers: self.config.num_insurers,
            min_price,
            max_price,
            avg_price,
            loss_ratio_history: self.loss_ratio_history.clone(),
            avg_claim_history: self.avg_claim_history.clone(),
        };

        // Reset year counters for next year
        self.industry_total_premiums = 0.0;
        self.industry_total_claims = 0.0;

        // Store for next year's actuarial pricing
        self.last_year_avg_claim = industry_avg_claim;
    }

    /// Handle price submitted event
    fn handle_price_submitted(
        &mut self,
        year: usize,
        insurer_id: usize,
        market_price: f64,
    ) -> Vec<(usize, Event)> {
        if year != self.current_year || !self.waiting_for_prices {
            return Vec::new();
        }

        self.receive_price(insurer_id, market_price);

        // Check if all prices received
        if self.all_prices_received() {
            self.waiting_for_prices = false;

            // Execute market clearing
            let allocations = self.clear_market();

            // Aggregate statistics
            self.aggregate_statistics(&allocations);

            // Broadcast MarketCleared event
            let time = year * 365;
            vec![(
                time,
                Event::MarketCleared {
                    year,
                    customer_allocations: allocations,
                    industry_avg_claim: self.last_year_avg_claim,
                },
            )]
        } else {
            Vec::new()
        }
    }

    /// Update industry totals from claims
    fn handle_claim_occurred(&mut self, amount: f64) {
        self.industry_total_claims += amount;
    }

    /// Calculate premium from allocation
    fn calculate_premium(&self, _customer_id: usize, insurer_id: usize) -> f64 {
        if let Some(&price) = self.prices_received.get(&insurer_id) {
            price
        } else {
            0.0
        }
    }

    /// Handle market cleared - calculate premiums
    fn handle_market_cleared(&mut self, allocations: &[(usize, usize)]) {
        for &(customer_id, insurer_id) in allocations {
            let premium = self.calculate_premium(customer_id, insurer_id);
            self.industry_total_premiums += premium;
        }
    }
}

impl Agent<Event, Stats> for MarketCoordinator {
    fn act(&mut self, _current_t: usize, event: &Event) -> Response<Event, Stats> {
        match event {
            Event::YearStart { year } => {
                let events = self.start_year(*year);
                Response::events(events)
            }

            Event::PriceSubmitted {
                year,
                insurer_id,
                market_price,
                ..
            } => {
                let events = self.handle_price_submitted(*year, *insurer_id, *market_price);
                Response::events(events)
            }

            Event::MarketCleared {
                customer_allocations,
                ..
            } => {
                self.handle_market_cleared(customer_allocations);
                Response::new()
            }

            Event::ClaimOccurred { amount, .. } => {
                self.handle_claim_occurred(*amount);
                Response::new()
            }

            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::Market(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_customers(n: usize) -> Vec<Customer> {
        (0..n).map(|i| Customer::new(i, (i as f64) * 0.1)).collect()
    }

    fn create_test_insurers(n: usize) -> HashMap<usize, f64> {
        (0..n).map(|i| (i, (i as f64) * 0.5)).collect()
    }

    #[test]
    fn test_coordinator_creation() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let coordinator = MarketCoordinator::new(config.clone(), customers, insurers);

        assert_eq!(coordinator.current_year, 0);
        assert!(!coordinator.waiting_for_prices);
        assert_eq!(coordinator.customers.len(), 10);
    }

    #[test]
    fn test_start_year_requests_prices() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 5;

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(5);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        let events = coordinator.start_year(1);

        assert_eq!(events.len(), 5); // One PricingRequest per insurer
        assert!(coordinator.waiting_for_prices);
        assert_eq!(coordinator.current_year, 1);
    }

    #[test]
    fn test_receive_price() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 3; // Match number of test insurers

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.start_year(1);

        coordinator.receive_price(0, 105.0);
        assert_eq!(coordinator.prices_received.get(&0), Some(&105.0));
        assert!(!coordinator.all_prices_received());

        coordinator.receive_price(1, 110.0);
        coordinator.receive_price(2, 108.0);
        assert!(coordinator.all_prices_received());
    }

    #[test]
    fn test_market_clearing_allocates_to_lowest_cost() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 3;
        config.distance_cost = 0.0; // No distance cost for simplicity

        let customers = vec![Customer::new(0, 0.0)];
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.start_year(1);

        // Insurer 1 has lowest price
        coordinator.receive_price(0, 110.0);
        coordinator.receive_price(1, 100.0);
        coordinator.receive_price(2, 105.0);

        let allocations = coordinator.clear_market();

        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0], (0, 1)); // Customer 0 → Insurer 1 (lowest price)
    }

    #[test]
    fn test_market_clearing_considers_distance() {
        use std::f64::consts::PI;

        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;
        config.distance_cost = 10.0; // High distance cost

        let customers = vec![Customer::new(0, 0.0)];
        let mut insurers = HashMap::new();
        insurers.insert(0, 0.0); // Same position as customer
        insurers.insert(1, PI); // Far from customer (π distance)

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.start_year(1);

        // Insurer 1 has lower price, but insurer 0 is closer
        coordinator.receive_price(0, 105.0); // Total cost = 105
        coordinator.receive_price(1, 100.0); // Total cost = 100 + 10×π ≈ 131.4

        let allocations = coordinator.clear_market();

        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0], (0, 0)); // Customer 0 → Insurer 0 (closer)
    }

    #[test]
    fn test_handle_price_submitted_triggers_market_clearing() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;

        let customers = create_test_customers(5);
        let insurers = create_test_insurers(2);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.start_year(1);

        // First price - no market clearing yet
        let events = coordinator.handle_price_submitted(1, 0, 105.0);
        assert!(events.is_empty());

        // Second price - triggers market clearing
        let events = coordinator.handle_price_submitted(1, 1, 110.0);
        assert_eq!(events.len(), 1);

        match &events[0].1 {
            Event::MarketCleared {
                year,
                customer_allocations,
                ..
            } => {
                assert_eq!(*year, 1);
                assert_eq!(customer_allocations.len(), 5); // All customers allocated
            }
            _ => panic!("Expected MarketCleared event"),
        }
    }

    #[test]
    fn test_aggregate_statistics() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 3;

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.start_year(1);

        coordinator.receive_price(0, 100.0);
        coordinator.receive_price(1, 110.0);
        coordinator.receive_price(2, 105.0);

        let allocations = coordinator.clear_market();
        coordinator.aggregate_statistics(&allocations);

        let stats = coordinator.stats();
        match stats {
            Stats::Market(s) => {
                assert_eq!(s.year, 1);
                assert_eq!(s.num_solvent_insurers, 3);
                assert_eq!(s.min_price, 100.0);
                assert_eq!(s.max_price, 110.0);
                assert!((s.avg_price - 105.0).abs() < 0.01);
            }
            _ => panic!("Expected Market stats"),
        }
    }

    #[test]
    fn test_claim_tracking() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);

        coordinator.handle_claim_occurred(100.0);
        coordinator.handle_claim_occurred(150.0);

        assert_eq!(coordinator.industry_total_claims, 250.0);
    }

    #[test]
    fn test_stats_projection() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers);
        coordinator.current_year = 5;
        coordinator.loss_ratio_history = vec![0.9, 1.0, 1.1, 1.0, 0.9];

        // Update stats to match current year
        coordinator.stats.year = 5;
        coordinator.stats.loss_ratio_history = coordinator.loss_ratio_history.clone();

        let stats = coordinator.stats();

        match stats {
            Stats::Market(s) => {
                assert_eq!(s.year, 5);
                assert_eq!(s.loss_ratio_history.len(), 5);
            }
            _ => panic!("Expected Market stats"),
        }
    }
}
