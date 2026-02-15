use crate::{BrokerStats, Event, ModelConfig, Stats};
use des::{Agent, Response};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Poisson};
use std::collections::HashSet;

/// Broker agent that generates new risks and manages quote deadlines
pub struct Broker {
    broker_id: usize,
    config: ModelConfig,
    rng: StdRng,
    next_risk_id: usize,
    our_risks: HashSet<usize>, // Track which risks this broker generated
    stats: BrokerStats,
}

impl Broker {
    pub fn new(broker_id: usize, config: ModelConfig, seed: u64) -> Self {
        Self {
            broker_id,
            config,
            rng: StdRng::seed_from_u64(seed),
            next_risk_id: broker_id * 100_000, // Ensure unique risk IDs across brokers
            our_risks: HashSet::new(),
            stats: BrokerStats::new(broker_id),
        }
    }

    fn generate_risks(&mut self, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Generate number of risks according to Poisson distribution
        let poisson = Poisson::new(self.config.risks_per_day).unwrap();
        let num_risks = poisson.sample(&mut self.rng) as usize;

        for _ in 0..num_risks {
            let risk_id = self.next_risk_id;
            self.next_risk_id += 1;

            // Random peril region
            let peril_region = self.rng.gen_range(0..self.config.num_peril_regions);

            // Risk limit is fixed in the base model
            let limit = self.config.risk_limit;

            // Track that we generated this risk
            self.our_risks.insert(risk_id);
            self.stats.risks_generated += 1;

            // Broadcast risk
            events.push((
                current_t,
                Event::RiskBroadcasted {
                    risk_id,
                    peril_region,
                    limit,
                    broker_id: self.broker_id,
                },
            ));

            // Set quote deadlines (simplified timing)
            // Lead quote consolidation: 1 day
            events.push((
                current_t + 1,
                Event::LeadQuoteConsolidationDeadline { risk_id },
            ));

            // Lead quote selection: 2 days
            events.push((current_t + 2, Event::LeadQuoteSelectionDeadline { risk_id }));

            // Follow quote consolidation: 3 days
            events.push((
                current_t + 3,
                Event::FollowQuoteConsolidationDeadline { risk_id },
            ));

            // Follow quote selection: 4 days
            events.push((
                current_t + 4,
                Event::FollowQuoteSelectionDeadline { risk_id },
            ));
        }

        events
    }
}

impl Agent<Event, Stats> for Broker {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Day => Response::events(self.generate_risks(current_t)),
            Event::LeadQuoteAccepted { risk_id, .. } => {
                // Check if this was our risk
                if self.our_risks.contains(risk_id) {
                    self.stats.risks_bound += 1;
                }
                Response::new()
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::BrokerStats(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_generates_risks() {
        let config = ModelConfig::default();
        let mut broker = Broker::new(0, config, 12345);

        let mut total_risks = 0;
        for _ in 0..100 {
            let resp = broker.act(0, &Event::Day);
            let risk_broadcasts = resp
                .events
                .iter()
                .filter(|(_, e)| matches!(e, Event::RiskBroadcasted { .. }))
                .count();
            total_risks += risk_broadcasts;
        }

        // With λ=0.06, expect around 6 risks over 100 days (with some variance)
        assert!(total_risks > 0);
        assert!(total_risks < 20); // Very unlikely to get >20 with λ=0.06
    }

    #[test]
    fn test_broker_sets_deadlines() {
        let config = ModelConfig::default();
        let mut broker = Broker::new(0, config, 12345);

        let resp = broker.act(0, &Event::Day);

        if !resp.events.is_empty() {
            // Check that deadlines are set for each risk
            let has_lead_consolidation = resp
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::LeadQuoteConsolidationDeadline { .. }));
            let has_lead_selection = resp
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::LeadQuoteSelectionDeadline { .. }));
            let has_follow_consolidation = resp
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::FollowQuoteConsolidationDeadline { .. }));
            let has_follow_selection = resp
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::FollowQuoteSelectionDeadline { .. }));

            if resp
                .events
                .iter()
                .any(|(_, e)| matches!(e, Event::RiskBroadcasted { .. }))
            {
                assert!(has_lead_consolidation);
                assert!(has_lead_selection);
                assert!(has_follow_consolidation);
                assert!(has_follow_selection);
            }
        }
    }

    #[test]
    fn test_broker_tracks_bound_risks() {
        // RED: This test should FAIL because brokers don't track bound risks yet
        let config = ModelConfig::default();
        let mut broker = Broker::new(0, config, 12345);

        // Keep trying until we generate at least one risk
        let mut risk_id = None;
        for _ in 0..100 {
            let initial_count = broker.stats.risks_generated;
            broker.act(0, &Event::Day);
            if broker.stats.risks_generated > initial_count {
                risk_id = Some(broker.next_risk_id - 1);
                break;
            }
        }

        let risk_id = risk_id.expect("Should have generated at least one risk");
        let risks_generated = broker.stats.risks_generated;

        assert!(risks_generated > 0);
        assert_eq!(broker.stats.risks_bound, 0); // Not bound yet

        // Simulate the risk being accepted
        broker.act(
            0,
            &Event::LeadQuoteAccepted {
                risk_id,
                syndicate_id: 0,
            },
        );

        // Should now show as bound
        assert_eq!(
            broker.stats.risks_bound, 1,
            "Broker should track when its risks are bound"
        );
    }
}
