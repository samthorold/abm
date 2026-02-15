use crate::{AttritionalLossGeneratorStats, Event, ModelConfig, Stats};
use des::{Agent, Response};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Gamma, Poisson};

/// Generates attritional loss events for risks
pub struct AttritionalLossGenerator {
    config: ModelConfig,
    rng: StdRng,
    stats: AttritionalLossGeneratorStats,
}

impl AttritionalLossGenerator {
    pub fn new(config: ModelConfig, seed: u64) -> Self {
        Self {
            config,
            rng: StdRng::seed_from_u64(seed),
            stats: AttritionalLossGeneratorStats::new(),
        }
    }

    fn generate_losses_for_risk(
        &mut self,
        risk_id: usize,
        expiration_time: usize,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Generate number of claims according to Poisson distribution
        let poisson = Poisson::new(self.config.yearly_claim_frequency).unwrap();
        let num_claims = poisson.sample(&mut self.rng) as usize;

        // Gamma distribution for loss severity
        let shape = 1.0 / (self.config.gamma_cov * self.config.gamma_cov);
        let scale = self.config.gamma_mean * self.config.gamma_cov * self.config.gamma_cov;
        let gamma = Gamma::new(shape, scale).unwrap();

        for _ in 0..num_claims {
            let amount = gamma.sample(&mut self.rng);
            // Schedule claim at random time before expiration
            let claim_time = if expiration_time > current_t {
                current_t + (self.rng.gen_range(0..=(expiration_time - current_t)))
            } else {
                current_t
            };

            events.push((
                claim_time,
                Event::AttritionalLossOccurred { risk_id, amount },
            ));

            self.stats.total_losses_generated += 1;
            self.stats.total_loss_amount += amount;
        }

        events
    }
}

impl Agent<Event, Stats> for AttritionalLossGenerator {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::RiskBroadcasted { risk_id, .. } => {
                // Assume 1 year expiration (365 days from now)
                let expiration_time = current_t + 365;
                Response::events(self.generate_losses_for_risk(
                    *risk_id,
                    expiration_time,
                    current_t,
                ))
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::AttritionalLossGeneratorStats(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generates_losses() {
        let config = ModelConfig::default();
        let mut generator = AttritionalLossGenerator::new(config, 12345);

        // With yearly frequency 0.1, expect some losses (but could be 0 due to Poisson)
        // Run multiple times to check it works
        let mut total_losses = 0;
        for i in 0..100 {
            let resp = generator.act(
                0,
                &Event::RiskBroadcasted {
                    risk_id: i,
                    peril_region: 0,
                    limit: 10_000_000.0,
                    broker_id: 0,
                },
            );
            total_losses += resp.events.len();
        }

        // With 100 risks and λ=0.1, expect around 10 losses
        assert!(total_losses > 0);
        assert!(total_losses < 30);
    }

    #[test]
    fn test_loss_amounts_reasonable() {
        let config = ModelConfig::default();
        let mut generator = AttritionalLossGenerator::new(config.clone(), 12345);

        let mut total_amount = 0.0;
        let mut count = 0;

        for i in 0..1000 {
            let resp = generator.act(
                0,
                &Event::RiskBroadcasted {
                    risk_id: i,
                    peril_region: 0,
                    limit: 10_000_000.0,
                    broker_id: 0,
                },
            );

            for (_, event) in resp.events {
                if let Event::AttritionalLossOccurred { amount, .. } = event {
                    total_amount += amount;
                    count += 1;
                }
            }
        }

        if count > 0 {
            let avg = total_amount / count as f64;
            // Should be close to gamma mean ($3M)
            assert!(avg > 1_000_000.0);
            assert!(avg < 10_000_000.0);
        }
    }
}
