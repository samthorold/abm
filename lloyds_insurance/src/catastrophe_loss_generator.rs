use crate::{CatastropheLossGeneratorStats, Event, ModelConfig, Stats};
use des::{Agent, Response};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Poisson};

/// Generates catastrophe events using Poisson distribution for frequency
/// and Pareto distribution for severity
pub struct CatastropheLossGenerator {
    scheduled_catastrophes: Vec<ScheduledCatastrophe>,
    stats: CatastropheLossGeneratorStats,
}

#[derive(Debug, Clone)]
struct ScheduledCatastrophe {
    time: usize,
    peril_region: usize,
    total_loss: f64,
}

impl CatastropheLossGenerator {
    pub fn new(config: ModelConfig, sim_years: usize, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut stats = CatastropheLossGeneratorStats::new();

        // Pre-generate catastrophe events
        let scheduled_catastrophes =
            Self::generate_catastrophes(&config, sim_years, &mut rng, &mut stats);

        Self {
            scheduled_catastrophes,
            stats,
        }
    }

    fn generate_catastrophes(
        config: &ModelConfig,
        sim_years: usize,
        rng: &mut StdRng,
        stats: &mut CatastropheLossGeneratorStats,
    ) -> Vec<ScheduledCatastrophe> {
        let mut catastrophes = Vec::new();

        // Determine number of catastrophes using Poisson distribution
        let lambda = config.mean_cat_events_per_year * sim_years as f64;

        // Handle zero lambda case (Poisson::new requires lambda > 0)
        if lambda == 0.0 {
            return catastrophes;
        }

        let poisson = Poisson::new(lambda).unwrap();
        let num_catastrophes = poisson.sample(rng) as usize;

        stats.total_catastrophes = num_catastrophes;

        let sim_days = sim_years * 365;

        for _ in 0..num_catastrophes {
            // Random time within simulation period
            let time = rng.gen_range(0..sim_days);

            // Random peril region
            let peril_region = rng.gen_range(0..config.num_peril_regions);

            // Loss amount using Pareto distribution (simplified for now)
            // TODO: Implement truncated Pareto properly
            let total_loss = Self::sample_pareto_loss(config, rng);

            *stats
                .catastrophes_by_region
                .entry(peril_region)
                .or_insert(0) += 1;
            stats.total_catastrophe_loss += total_loss;

            catastrophes.push(ScheduledCatastrophe {
                time,
                peril_region,
                total_loss,
            });
        }

        // Sort by time for efficient processing
        catastrophes.sort_by_key(|c| c.time);

        catastrophes
    }

    fn sample_pareto_loss(config: &ModelConfig, rng: &mut StdRng) -> f64 {
        // Simplified Pareto sampling
        // Full implementation would use truncated Pareto with min_cat_damage_fraction
        // For now, use a simple power law distribution

        let alpha = config.pareto_shape;
        let x_min = config.risk_limit * config.min_cat_damage_fraction;

        // Pareto: P(X > x) = (x_min / x)^alpha
        // Inverse CDF: X = x_min / U^(1/alpha) where U ~ Uniform(0,1)
        let u: f64 = rng.gen_range(0.0..1.0);
        let loss = x_min / u.powf(1.0 / alpha);

        // Cap at some reasonable maximum (e.g., 100x minimum)
        loss.min(x_min * 100.0)
    }
}

impl Agent<Event, Stats> for CatastropheLossGenerator {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Day => {
                // Check if any catastrophes should fire today
                let mut events = Vec::new();

                // Process all catastrophes scheduled for current_t
                // (they're sorted by time, so we can stop when we hit a future one)
                while let Some(cat) = self.scheduled_catastrophes.first() {
                    if cat.time == current_t {
                        events.push((
                            current_t,
                            Event::CatastropheLossOccurred {
                                peril_region: cat.peril_region,
                                total_loss: cat.total_loss,
                            },
                        ));
                        self.scheduled_catastrophes.remove(0);
                    } else if cat.time > current_t {
                        break; // Future catastrophes
                    } else {
                        // This shouldn't happen (past catastrophes should have been removed)
                        self.scheduled_catastrophes.remove(0);
                    }
                }

                Response::events(events)
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::CatastropheLossGeneratorStats(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generates_catastrophes() {
        // RED: This test should initially guide implementation
        let config = ModelConfig {
            mean_cat_events_per_year: 0.05, // ~2.5 events over 50 years
            ..Default::default()
        };

        let generator = CatastropheLossGenerator::new(config.clone(), 50, 12345);

        // Should have pre-generated some catastrophes
        assert!(
            generator.stats.total_catastrophes > 0,
            "Should generate at least some catastrophes over 50 years"
        );

        // Number should be reasonable (within 3 std deviations)
        // Expected: λ = 0.05 * 50 = 2.5, std = sqrt(2.5) ≈ 1.58
        // Range: [0, 2.5 + 3*1.58] ≈ [0, 7]
        assert!(
            generator.stats.total_catastrophes < 10,
            "Should not generate unreasonably many catastrophes"
        );
    }

    #[test]
    fn test_catastrophe_timing() {
        let config = ModelConfig {
            mean_cat_events_per_year: 1.0, // High rate for testing
            ..Default::default()
        };

        let mut generator = CatastropheLossGenerator::new(config.clone(), 10, 54321);

        let mut fired_catastrophes = 0;
        let sim_days = 10 * 365;

        // Run simulation and count catastrophes
        for day in 0..sim_days {
            let resp = generator.act(day, &Event::Day);
            for (_, event) in resp.events {
                if matches!(event, Event::CatastropheLossOccurred { .. }) {
                    fired_catastrophes += 1;
                }
            }
        }

        // Should fire the pre-generated catastrophes
        assert_eq!(
            fired_catastrophes, generator.stats.total_catastrophes,
            "All pre-generated catastrophes should fire during simulation"
        );
    }

    #[test]
    fn test_catastrophe_loss_amounts() {
        let config = ModelConfig::default();
        let generator = CatastropheLossGenerator::new(config.clone(), 50, 99999);

        if generator.stats.total_catastrophes > 0 {
            let avg_loss =
                generator.stats.total_catastrophe_loss / generator.stats.total_catastrophes as f64;

            let min_expected = config.risk_limit * config.min_cat_damage_fraction;

            // Average loss should be >= minimum
            assert!(
                avg_loss >= min_expected,
                "Average catastrophe loss ${:.0} should be >= minimum ${:.0}",
                avg_loss,
                min_expected
            );
        }
    }

    #[test]
    fn test_no_catastrophes_with_zero_rate() {
        let config = ModelConfig {
            mean_cat_events_per_year: 0.0,
            ..Default::default()
        };

        let generator = CatastropheLossGenerator::new(config, 50, 11111);

        assert_eq!(
            generator.stats.total_catastrophes, 0,
            "Should generate no catastrophes when rate is 0"
        );
    }
}
