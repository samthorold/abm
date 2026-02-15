//! Claim generator agent implementation
//!
//! Generates stochastic claims:
//! - Bernoulli(b) × Gamma(μ, σ) claims
//! - Random timing during year

use crate::{Event, ModelConfig, Stats};
use des::{Agent, Response};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use rand_distr::{Bernoulli, Distribution, Gamma};

pub struct ClaimGenerator {
    config: ModelConfig,
    rng: StdRng,
    total_claims_generated: usize,
    total_claim_amount: f64,
}

impl ClaimGenerator {
    pub fn new(config: ModelConfig, seed: u64) -> Self {
        ClaimGenerator {
            config,
            rng: StdRng::seed_from_u64(seed),
            total_claims_generated: 0,
            total_claim_amount: 0.0,
        }
    }

    /// Generate claims for all customer allocations
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

        // Year time range: [year×365, (year+1)×365)
        let year_start = year * 365;
        let year_end = (year + 1) * 365;

        for &(customer_id, insurer_id) in customer_allocations {
            // Bernoulli trial: does a claim occur?
            if bernoulli.sample(&mut self.rng) {
                // Sample claim amount
                let amount = gamma.sample(&mut self.rng);

                // Schedule claim at random time during year
                let claim_time = self.rng.gen_range(year_start..year_end);

                events.push((
                    claim_time,
                    Event::ClaimOccurred {
                        year,
                        customer_id,
                        insurer_id,
                        amount,
                    },
                ));

                // Track statistics
                self.total_claims_generated += 1;
                self.total_claim_amount += amount;
            }
        }

        events
    }
}

impl Agent<Event, Stats> for ClaimGenerator {
    fn act(&mut self, _current_t: usize, event: &Event) -> Response<Event, Stats> {
        match event {
            Event::MarketCleared {
                year,
                customer_allocations,
                ..
            } => {
                let events = self.generate_claims_for_year(*year, customer_allocations);
                Response::events(events)
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::ClaimGenerator // No observable state needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ModelConfig {
        ModelConfig::baseline()
    }

    #[test]
    fn test_claim_generator_creation() {
        let config = test_config();
        let generator = ClaimGenerator::new(config, 42);

        assert_eq!(generator.total_claims_generated, 0);
        assert_eq!(generator.total_claim_amount, 0.0);
    }

    #[test]
    fn test_generates_claims_for_allocations() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        // Create 100 customer allocations
        let allocations: Vec<(usize, usize)> = (0..100).map(|i| (i, i % 5)).collect();

        let events = generator.generate_claims_for_year(1, &allocations);

        // With claim_frequency = 1.0, expect close to 100 claims
        assert!(events.len() >= 90);
        assert!(events.len() <= 100);
    }

    #[test]
    fn test_claim_amounts_follow_gamma() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config.clone(), 42);

        // Generate many claims
        let allocations: Vec<(usize, usize)> = (0..1000).map(|i| (i, i % 10)).collect();

        let events = generator.generate_claims_for_year(1, &allocations);

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
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        let allocations: Vec<(usize, usize)> = (0..100).map(|i| (i, i % 5)).collect();

        let events = generator.generate_claims_for_year(2, &allocations);

        // Year 2 time range: [730, 1095)
        let year_start = 2 * 365;
        let year_end = 3 * 365;

        for (time, _) in &events {
            assert!(*time >= year_start);
            assert!(*time < year_end);
        }
    }

    #[test]
    fn test_claims_track_customer_insurer() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        let allocations = vec![(10, 2), (11, 3), (12, 2)];

        let events = generator.generate_claims_for_year(1, &allocations);

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
    fn test_statistics_tracking() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        let allocations: Vec<(usize, usize)> = (0..100).map(|i| (i, i % 5)).collect();

        let initial_count = generator.total_claims_generated;
        let initial_amount = generator.total_claim_amount;

        let events = generator.generate_claims_for_year(1, &allocations);

        assert!(generator.total_claims_generated > initial_count);
        assert!(generator.total_claim_amount > initial_amount);
        assert_eq!(
            generator.total_claims_generated - initial_count,
            events.len()
        );
    }

    #[test]
    fn test_deterministic_with_seed() {
        let config = test_config();
        let mut gen1 = ClaimGenerator::new(config.clone(), 12345);
        let mut gen2 = ClaimGenerator::new(config, 12345);

        let allocations: Vec<(usize, usize)> = (0..50).map(|i| (i, i % 5)).collect();

        let events1 = gen1.generate_claims_for_year(1, &allocations);
        let events2 = gen2.generate_claims_for_year(1, &allocations);

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

    #[test]
    fn test_handles_market_cleared_event() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        let allocations = vec![(0, 0), (1, 1), (2, 0)];

        let event = Event::MarketCleared {
            year: 1,
            customer_allocations: allocations,
            industry_avg_claim: 100.0,
        };

        let response = generator.act(0, &event);

        assert!(!response.events.is_empty());
    }

    #[test]
    fn test_ignores_other_events() {
        let config = test_config();
        let mut generator = ClaimGenerator::new(config, 42);

        let event = Event::YearStart { year: 1 };
        let response = generator.act(0, &event);

        assert!(response.events.is_empty());
    }
}
