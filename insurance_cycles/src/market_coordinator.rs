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
use crate::{Customer, Event, MarketStats, ModelConfig, Stats, DAYS_PER_YEAR};
use des::{Agent, Response};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use rand_distr::{Bernoulli, Distribution, Gamma};
use std::collections::HashMap;

pub struct MarketCoordinator {
    config: ModelConfig,
    customers: Vec<Customer>,
    insurer_positions: HashMap<usize, f64>, // insurer_id -> position

    // RNG for claim generation (moved from ClaimGenerator)
    claim_rng: StdRng,

    // RNG for allocation noise (reduces market concentration)
    allocation_rng: StdRng,

    // Shadow state for capacity tracking
    // Note: The DES framework uses broadcast semantics - agents cannot query each other's
    // state during act(). To check capacity during market clearing, the coordinator must
    // track insurer capital. This shadow state is updated from the same events insurers
    // receive (ClaimOccurred, premiums from MarketCleared), maintaining consistency.
    insurer_capital: HashMap<usize, f64>, // insurer_id -> current capital

    // Current year state
    current_year: usize,
    waiting_for_prices: bool,
    prices_received: HashMap<usize, f64>, // insurer_id -> market_price

    // Industry tracking (current year)
    // Note: Claims must be tracked via ClaimOccurred events because the DES framework
    // doesn't allow querying other agents' Stats during event processing.
    // Premiums are computed from allocations × prices to avoid duplicating insurer calculations.
    industry_total_premiums: f64,
    industry_total_claims: f64,
    last_year_avg_claim: f64,

    // Cumulative tracking (for shadow state consistency validation)
    cumulative_premiums: f64,
    cumulative_claims: f64,

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
        claim_seed: u64,
        allocation_seed: u64,
    ) -> Self {
        // Initialize shadow capital for all insurers
        let insurer_capital: HashMap<usize, f64> = (0..config.num_insurers)
            .map(|i| (i, config.initial_capital))
            .collect();

        MarketCoordinator {
            config,
            customers,
            insurer_positions,
            claim_rng: StdRng::seed_from_u64(claim_seed),
            allocation_rng: StdRng::seed_from_u64(allocation_seed),
            insurer_capital,
            current_year: 0,
            waiting_for_prices: false,
            prices_received: HashMap::new(),
            industry_total_premiums: 0.0,
            industry_total_claims: 0.0,
            last_year_avg_claim: 100.0, // Initialize to expected claim amount
            cumulative_premiums: 0.0,
            cumulative_claims: 0.0,
            loss_ratio_history: Vec::new(),
            avg_claim_history: Vec::new(),
            stats: MarketStats {
                year: 0,
                total_premiums: 0.0,
                total_claims: 0.0,
                industry_loss_ratio: 0.0,
                industry_avg_claim: 0.0,
                cumulative_premiums: 0.0,
                cumulative_claims: 0.0,
                num_solvent_insurers: 0,
                total_insurers: 0,
                min_price: 0.0,
                max_price: 0.0,
                avg_price: 0.0,
                herfindahl_index: 0.0,
                gini_coefficient: 0.0,
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

        // Request prices from all insurers at start of year
        let time = year * DAYS_PER_YEAR;
        let mut events = Vec::new();

        for insurer_id in 0..self.config.num_insurers {
            events.push((time, Event::PricingRequest { year, insurer_id }));
        }

        // Schedule YearEnd event at end of year (before next YearStart)
        // This gives time for all claims to be processed
        if year > 1 {
            // Emit YearEnd for previous year (starting from year 2)
            let year_end_time = year * DAYS_PER_YEAR - 1;
            events.push((
                year_end_time,
                Event::YearEnd {
                    year: year - 1,
                    industry_avg_claim: self.last_year_avg_claim,
                    industry_loss_ratio: if self.industry_total_premiums > 0.0 {
                        self.industry_total_claims / self.industry_total_premiums
                    } else {
                        0.0
                    },
                },
            ));
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

    /// Generate claims for all customer allocations (moved from ClaimGenerator)
    ///
    /// For each customer-insurer pair:
    /// 1. Bernoulli trial: does a claim occur? (probability = claim_frequency)
    /// 2. If yes, sample claim amount from Gamma(shape, scale)
    /// 3. Schedule ClaimOccurred event at random time during year
    fn generate_claims_for_year(
        &mut self,
        year: usize,
        customer_allocations: &[(usize, usize)],
    ) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Bernoulli distribution for claim occurrence
        let bernoulli = Bernoulli::new(self.config.claim_frequency).unwrap();

        // Gamma distribution for claim amounts
        let shape = self.config.gamma_shape();
        let scale = self.config.gamma_scale();
        let gamma = Gamma::new(shape, scale).unwrap();

        // Year time range: [year×DAYS_PER_YEAR, (year+1)×DAYS_PER_YEAR)
        let year_start = year * DAYS_PER_YEAR;
        let year_end = (year + 1) * DAYS_PER_YEAR;

        for &(customer_id, insurer_id) in customer_allocations {
            // Bernoulli trial: does a claim occur?
            if bernoulli.sample(&mut self.claim_rng) {
                // Sample claim amount
                let amount = gamma.sample(&mut self.claim_rng);

                // Schedule claim at random time during year
                let claim_time = self.claim_rng.gen_range(year_start..year_end);

                events.push((
                    claim_time,
                    Event::ClaimOccurred {
                        year,
                        customer_id,
                        insurer_id,
                        amount,
                    },
                ));
            }
        }

        events
    }

    /// Execute market clearing algorithm with capacity constraints
    ///
    /// For each customer:
    /// 1. Calculate total cost for all insurers (price + γ × distance)
    /// 2. Sort insurers by increasing cost
    /// 3. Allocate to first insurer with available capacity
    /// 4. If insurer at capacity, try next best option
    ///
    /// Capacity constraint: An insurer can accept a customer only if:
    ///   current_premium + new_premium <= capital × leverage_ratio
    ///
    /// This implements the paper's specification that customers choose their second-best
    /// option when their preferred insurer is at capacity, preventing winner-take-all
    /// dynamics and reducing market concentration.
    ///
    /// Returns: Vec<(customer_id, insurer_id)>
    fn clear_market(&mut self) -> Vec<(usize, usize)> {
        let mut allocations = Vec::new();
        let mut insurer_premium_totals: HashMap<usize, f64> = HashMap::new();
        let gamma = self.config.distance_cost;

        for customer in &self.customers {
            // Build sorted list of (insurer_id, total_cost) with noise
            let mut insurer_costs: Vec<(usize, f64)> = self
                .prices_received
                .iter()
                .map(|(&insurer_id, &price)| {
                    let insurer_pos = self.insurer_positions[&insurer_id];
                    let distance = circular_distance(customer.position, insurer_pos);
                    // Add noise to total cost (bounded rationality / decision error)
                    let base_cost = price + gamma * distance;
                    let noise_range = self.config.allocation_noise;
                    let noise =
                        self.allocation_rng.gen_range(-noise_range..noise_range) * base_cost;
                    let total_cost = base_cost + noise;
                    (insurer_id, total_cost)
                })
                .collect();

            // Sort by increasing cost (handle NaN by treating as infinite cost)
            insurer_costs.sort_by(|a, b| {
                a.1.partial_cmp(&b.1).unwrap_or_else(|| {
                    // If comparison fails (NaN), NaN values go to the end
                    if a.1.is_nan() && b.1.is_nan() {
                        std::cmp::Ordering::Equal
                    } else if a.1.is_nan() {
                        std::cmp::Ordering::Greater // NaN goes to end
                    } else {
                        std::cmp::Ordering::Less
                    }
                })
            });

            // Try insurers in order until one has capacity
            for (insurer_id, _) in insurer_costs {
                let price = self.prices_received[&insurer_id];
                let current_premium = *insurer_premium_totals.get(&insurer_id).unwrap_or(&0.0);
                let required_premium = current_premium + price;

                // Check capacity constraint (capital × leverage)
                let capital = self.insurer_capital[&insurer_id];
                let max_premium = capital * self.config.leverage_ratio;

                if capital > 0.0 && required_premium <= max_premium {
                    // Insurer has capacity - allocate customer
                    allocations.push((customer.id, insurer_id));
                    insurer_premium_totals.insert(insurer_id, required_premium);
                    break;
                }
                // Otherwise, try next best insurer
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
            cumulative_premiums: self.cumulative_premiums,
            cumulative_claims: self.cumulative_claims,
            num_solvent_insurers: self.prices_received.len(),
            total_insurers: self.config.num_insurers,
            min_price,
            max_price,
            avg_price,
            herfindahl_index: 0.0, // Will be calculated in stats() method
            gini_coefficient: 0.0, // Will be calculated in stats() method
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

            // Broadcast MarketCleared event after all prices submitted
            let time = year * DAYS_PER_YEAR + 2;
            let mut events = vec![(
                time,
                Event::MarketCleared {
                    year,
                    customer_allocations: allocations.clone(),
                    industry_avg_claim: self.last_year_avg_claim,
                },
            )];

            // Generate claims for the year (moved from ClaimGenerator)
            events.extend(self.generate_claims_for_year(year, &allocations));

            events
        } else {
            Vec::new()
        }
    }

    /// Update industry totals and shadow capital from claims
    fn handle_claim_occurred(&mut self, insurer_id: usize, amount: f64) {
        self.industry_total_claims += amount;
        self.cumulative_claims += amount;

        // Update shadow capital
        if let Some(capital) = self.insurer_capital.get_mut(&insurer_id) {
            *capital -= amount;
        }
    }

    /// Calculate premium from allocation
    fn calculate_premium(&self, _customer_id: usize, insurer_id: usize) -> f64 {
        if let Some(&price) = self.prices_received.get(&insurer_id) {
            price
        } else {
            0.0
        }
    }

    /// Handle market cleared - calculate premiums and update shadow capital
    fn handle_market_cleared(&mut self, allocations: &[(usize, usize)]) {
        // Calculate market shares for concentration metrics
        let mut customer_counts: HashMap<usize, usize> = HashMap::new();
        for &(_customer_id, insurer_id) in allocations {
            *customer_counts.entry(insurer_id).or_insert(0) += 1;
        }

        let total_customers = allocations.len() as f64;
        let market_shares: Vec<f64> = self
            .insurer_positions
            .keys()
            .map(|&insurer_id| {
                customer_counts.get(&insurer_id).copied().unwrap_or(0) as f64 / total_customers
            })
            .collect();

        // Update concentration metrics
        self.stats.herfindahl_index = MarketStats::calculate_herfindahl(&market_shares);
        self.stats.gini_coefficient = MarketStats::calculate_gini(&market_shares);

        // Calculate premiums and update shadow capital
        for &(customer_id, insurer_id) in allocations {
            let premium = self.calculate_premium(customer_id, insurer_id);
            self.industry_total_premiums += premium;
            self.cumulative_premiums += premium;

            // Update shadow capital
            if let Some(capital) = self.insurer_capital.get_mut(&insurer_id) {
                *capital += premium;
            }
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

            Event::ClaimOccurred {
                insurer_id, amount, ..
            } => {
                self.handle_claim_occurred(*insurer_id, *amount);
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

        let coordinator = MarketCoordinator::new(config.clone(), customers, insurers, 12345, 54321);

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

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);

        // First year (year 1) - only PricingRequests, no YearEnd yet
        let events = coordinator.start_year(1);
        assert_eq!(events.len(), 5); // One PricingRequest per insurer

        // Subsequent years include YearEnd for previous year
        coordinator.last_year_avg_claim = 100.0;
        coordinator.industry_total_premiums = 1000.0;
        coordinator.industry_total_claims = 1000.0;
        let events2 = coordinator.start_year(2);
        assert_eq!(events2.len(), 6); // 5 PricingRequests + 1 YearEnd

        assert!(coordinator.waiting_for_prices);
        assert_eq!(coordinator.current_year, 2);
    }

    #[test]
    fn test_receive_price() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 3; // Match number of test insurers

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
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

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
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

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
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

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
        coordinator.start_year(1);

        // First price - no market clearing yet
        let events = coordinator.handle_price_submitted(1, 0, 105.0);
        assert!(events.is_empty());

        // Second price - triggers market clearing and claim generation
        let events = coordinator.handle_price_submitted(1, 1, 110.0);
        // Should contain MarketCleared + ClaimOccurred events
        assert!(!events.is_empty());

        // First event should be MarketCleared
        match &events[0].1 {
            Event::MarketCleared {
                year,
                customer_allocations,
                ..
            } => {
                assert_eq!(*year, 1);
                assert_eq!(customer_allocations.len(), 5); // All customers allocated
            }
            _ => panic!("Expected MarketCleared event as first event"),
        }

        // Remaining events should be ClaimOccurred events
        for (_time, event) in &events[1..] {
            match event {
                Event::ClaimOccurred { .. } => {} // Expected
                _ => panic!("Expected ClaimOccurred events after MarketCleared"),
            }
        }
    }

    #[test]
    fn test_aggregate_statistics() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 3;

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
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

        let mut coordinator =
            MarketCoordinator::new(config.clone(), customers, insurers, 12345, 54321);

        let initial_capital = config.initial_capital;

        coordinator.handle_claim_occurred(0, 100.0);
        coordinator.handle_claim_occurred(1, 150.0);

        assert_eq!(coordinator.industry_total_claims, 250.0);

        // Verify shadow capital updated
        assert_eq!(coordinator.insurer_capital[&0], initial_capital - 100.0);
        assert_eq!(coordinator.insurer_capital[&1], initial_capital - 150.0);
        assert_eq!(coordinator.insurer_capital[&2], initial_capital); // Unchanged
    }

    #[test]
    fn test_stats_projection() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(10);
        let insurers = create_test_insurers(3);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
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

    // Tests for claim generation (moved from ClaimGenerator)

    #[test]
    fn test_generates_claims_for_allocations() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(100);
        let insurers = create_test_insurers(5);
        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 42, 24);

        // Create 100 customer allocations
        let allocations: Vec<(usize, usize)> = (0..100).map(|i| (i, i % 5)).collect();

        let events = coordinator.generate_claims_for_year(1, &allocations);

        // With claim_frequency = 1.0, expect close to 100 claims
        assert!(events.len() >= 90);
        assert!(events.len() <= 100);
    }

    #[test]
    fn test_claim_amounts_follow_gamma() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(1000);
        let insurers = create_test_insurers(10);
        let mut coordinator = MarketCoordinator::new(config.clone(), customers, insurers, 42, 24);

        // Generate many claims
        let allocations: Vec<(usize, usize)> = (0..1000).map(|i| (i, i % 10)).collect();

        let events = coordinator.generate_claims_for_year(1, &allocations);

        // Extract claim amounts
        let amounts: Vec<f64> = events
            .iter()
            .filter_map(|(_, event)| {
                if let Event::ClaimOccurred { amount, .. } = event {
                    Some(*amount)
                } else {
                    None
                }
            })
            .collect();

        assert!(!amounts.is_empty());

        // Check mean is close to gamma_mean (100.0)
        let mean = amounts.iter().sum::<f64>() / amounts.len() as f64;
        assert!((mean - config.gamma_mean).abs() < 10.0); // Within 10% tolerance

        // All amounts should be positive
        assert!(amounts.iter().all(|&a| a > 0.0));
    }

    #[test]
    fn test_claims_scheduled_during_year() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(100);
        let insurers = create_test_insurers(5);
        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 42, 24);

        let allocations: Vec<(usize, usize)> = (0..100).map(|i| (i, i % 5)).collect();

        let events = coordinator.generate_claims_for_year(2, &allocations);

        // Year 2 time range: [2×DAYS_PER_YEAR, 3×DAYS_PER_YEAR)
        let year_start = 2 * DAYS_PER_YEAR;
        let year_end = 3 * DAYS_PER_YEAR;

        for (time, _) in &events {
            assert!(*time >= year_start);
            assert!(*time < year_end);
        }
    }

    #[test]
    fn test_claims_track_customer_insurer() {
        let config = ModelConfig::baseline();
        let customers = create_test_customers(20);
        let insurers = create_test_insurers(5);
        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 42, 24);

        let allocations = vec![(10, 2), (11, 3), (12, 2)];

        let events = coordinator.generate_claims_for_year(1, &allocations);

        // Verify customer and insurer IDs are preserved
        for (_time, event) in &events {
            match event {
                Event::ClaimOccurred {
                    customer_id,
                    insurer_id,
                    ..
                } => {
                    // Check that this allocation exists
                    assert!(
                        allocations.contains(&(*customer_id, *insurer_id)),
                        "Claim for non-existent allocation"
                    );
                }
                _ => panic!("Expected ClaimOccurred event"),
            }
        }
    }

    #[test]
    fn test_deterministic_claim_generation_with_seed() {
        let config = ModelConfig::baseline();
        let customers1 = create_test_customers(50);
        let insurers1 = create_test_insurers(5);
        let mut coord1 =
            MarketCoordinator::new(config.clone(), customers1, insurers1, 12345, 54321);

        let customers2 = create_test_customers(50);
        let insurers2 = create_test_insurers(5);
        let mut coord2 = MarketCoordinator::new(config, customers2, insurers2, 12345, 54321);

        let allocations: Vec<(usize, usize)> = (0..50).map(|i| (i, i % 5)).collect();

        let events1 = coord1.generate_claims_for_year(1, &allocations);
        let events2 = coord2.generate_claims_for_year(1, &allocations);

        // Same seed should produce same number of claims
        assert_eq!(events1.len(), events2.len());

        // Same claim amounts and times
        for (e1, e2) in events1.iter().zip(events2.iter()) {
            assert_eq!(e1.0, e2.0); // Same time

            match (&e1.1, &e2.1) {
                (
                    Event::ClaimOccurred { amount: a1, .. },
                    Event::ClaimOccurred { amount: a2, .. },
                ) => {
                    assert!((a1 - a2).abs() < 1e-10); // Same amount
                }
                _ => panic!("Expected ClaimOccurred events"),
            }
        }
    }

    // Tests for capacity constraints (Sprint 2)

    #[test]
    fn test_capacity_constraint_enforced() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;
        config.initial_capital = 1000.0;
        config.leverage_ratio = 2.0; // Max premium = 2000.0

        // Create many customers that would exceed capacity
        let customers = create_test_customers(100);
        let insurers = create_test_insurers(2);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
        coordinator.start_year(1);

        // Both insurers submit same price (insurer 0 slightly cheaper)
        coordinator.receive_price(0, 100.0);
        coordinator.receive_price(1, 100.1);

        let allocations = coordinator.clear_market();

        // Calculate total premium for each insurer
        let mut insurer_0_premium = 0.0;
        let mut insurer_1_premium = 0.0;

        for &(_customer_id, insurer_id) in &allocations {
            if insurer_id == 0 {
                insurer_0_premium += 100.0;
            } else {
                insurer_1_premium += 100.1;
            }
        }

        // Verify no insurer exceeds capacity (capital × leverage = 2000.0)
        assert!(
            insurer_0_premium <= 2000.0,
            "Insurer 0 premium {} exceeds capacity 2000.0",
            insurer_0_premium
        );
        assert!(
            insurer_1_premium <= 2000.0,
            "Insurer 1 premium {} exceeds capacity 2000.0",
            insurer_1_premium
        );

        // Verify both insurers got customers (fallback to second-best worked)
        assert!(insurer_0_premium > 0.0);
        assert!(insurer_1_premium > 0.0);
    }

    #[test]
    fn test_fallback_to_second_best_when_at_capacity() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;
        config.initial_capital = 100.0;
        config.leverage_ratio = 2.0; // Max premium = 200.0
        config.distance_cost = 0.0; // Ignore distance

        // Only 3 customers
        let customers = vec![
            Customer::new(0, 0.0),
            Customer::new(1, 0.0),
            Customer::new(2, 0.0),
        ];
        let insurers = create_test_insurers(2);

        let mut coordinator = MarketCoordinator::new(config, customers, insurers, 12345, 54321);
        coordinator.start_year(1);

        // Insurer 0: lower price but can only fit 2 customers (2 × 100 = 200 = max)
        // Insurer 1: higher price
        coordinator.receive_price(0, 100.0);
        coordinator.receive_price(1, 110.0);

        let allocations = coordinator.clear_market();

        assert_eq!(allocations.len(), 3);

        // Count allocations per insurer
        let insurer_0_count = allocations.iter().filter(|(_, id)| *id == 0).count();
        let insurer_1_count = allocations.iter().filter(|(_, id)| *id == 1).count();

        // Insurer 0 should get 2 customers (at capacity)
        // Insurer 1 should get 1 customer (fallback)
        assert_eq!(insurer_0_count, 2);
        assert_eq!(insurer_1_count, 1);
    }

    // Tests for probabilistic allocation (Sprint 3)

    #[test]
    fn test_allocation_has_stochasticity() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;
        config.distance_cost = 0.0; // Simplify

        let customers = create_test_customers(100);
        let insurers = create_test_insurers(2);

        // Run allocation twice with different allocation seeds
        let mut coord1 = MarketCoordinator::new(
            config.clone(),
            customers.clone(),
            insurers.clone(),
            12345,
            11111,
        );
        let mut coord2 = MarketCoordinator::new(config, customers, insurers, 12345, 22222);

        coord1.start_year(1);
        coord2.start_year(1);

        // Same prices for both
        coord1.receive_price(0, 100.0);
        coord1.receive_price(1, 100.5);
        coord2.receive_price(0, 100.0);
        coord2.receive_price(1, 100.5);

        let allocations1 = coord1.clear_market();
        let allocations2 = coord2.clear_market();

        // Same number of customers allocated
        assert_eq!(allocations1.len(), allocations2.len());

        // But different distributions (due to noise)
        let insurer_0_count_1 = allocations1.iter().filter(|(_, id)| *id == 0).count();
        let insurer_0_count_2 = allocations2.iter().filter(|(_, id)| *id == 0).count();

        // With 100 customers and ±5% noise, distributions should differ
        // (This test may occasionally fail due to randomness, but very unlikely)
        assert_ne!(
            insurer_0_count_1, insurer_0_count_2,
            "Expected different allocations with different allocation seeds"
        );
    }

    #[test]
    fn test_shadow_capital_tracks_premiums_and_claims() {
        let mut config = ModelConfig::baseline();
        config.num_insurers = 2;
        config.initial_capital = 1000.0;

        let customers = create_test_customers(10);
        let insurers = create_test_insurers(2);

        let mut coordinator =
            MarketCoordinator::new(config.clone(), customers, insurers, 12345, 54321);
        coordinator.start_year(1);

        // Submit prices
        coordinator.receive_price(0, 100.0);
        coordinator.receive_price(1, 105.0);

        // Execute market clearing
        let allocations = coordinator.clear_market();

        // Simulate MarketCleared event handling (updates premiums and capital)
        coordinator.handle_market_cleared(&allocations);

        // Calculate expected premium for insurer 0
        let insurer_0_customers = allocations.iter().filter(|(_, id)| *id == 0).count();
        let expected_premium_0 = insurer_0_customers as f64 * 100.0;

        // Check shadow capital increased by premiums
        let expected_capital_0 = config.initial_capital + expected_premium_0;
        assert!(
            (coordinator.insurer_capital[&0] - expected_capital_0).abs() < 1e-6,
            "Expected capital {}, got {}",
            expected_capital_0,
            coordinator.insurer_capital[&0]
        );

        // Simulate claims
        coordinator.handle_claim_occurred(0, 50.0);
        coordinator.handle_claim_occurred(0, 30.0);

        // Check shadow capital decreased by claims
        let expected_capital_after_claims = expected_capital_0 - 80.0;
        assert!(
            (coordinator.insurer_capital[&0] - expected_capital_after_claims).abs() < 1e-6,
            "Expected capital {}, got {}",
            expected_capital_after_claims,
            coordinator.insurer_capital[&0]
        );
    }
}
