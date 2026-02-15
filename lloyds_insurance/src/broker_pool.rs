use crate::{BrokerStats, Event, ModelConfig, Stats};
use des::{Agent, Response};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Poisson};
use std::collections::HashMap;

/// Manages multiple broker states internally to reduce event broadcast overhead
/// Replaces N individual Broker agents with a single BrokerPool agent
pub struct BrokerPool {
    config: ModelConfig,
    brokers: Vec<BrokerState>,
    risk_ownership: HashMap<usize, usize>, // risk_id -> broker_id
}

/// Internal state for each broker within the pool
struct BrokerState {
    broker_id: usize,
    rng: StdRng,
    next_risk_id: usize,
    stats: BrokerStats,
}

impl BrokerPool {
    pub fn new(num_brokers: usize, config: ModelConfig, base_seed: u64) -> Self {
        let brokers = (0..num_brokers)
            .map(|i| BrokerState {
                broker_id: i,
                rng: StdRng::seed_from_u64(base_seed + i as u64),
                next_risk_id: i * 100_000, // Ensure unique risk IDs across brokers
                stats: BrokerStats::new(i),
            })
            .collect();

        Self {
            config,
            brokers,
            risk_ownership: HashMap::new(),
        }
    }

    fn generate_risks_for_all_brokers(&mut self, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();
        let poisson = Poisson::new(self.config.risks_per_day).unwrap();

        for broker in &mut self.brokers {
            // Generate number of risks according to Poisson distribution
            let num_risks = poisson.sample(&mut broker.rng) as usize;

            for _ in 0..num_risks {
                let risk_id = broker.next_risk_id;
                broker.next_risk_id += 1;

                // Random peril region
                let peril_region = broker.rng.gen_range(0..self.config.num_peril_regions);

                // Risk limit is fixed in the base model
                let limit = self.config.risk_limit;

                // Track ownership
                self.risk_ownership.insert(risk_id, broker.broker_id);
                broker.stats.risks_generated += 1;

                // Broadcast risk
                events.push((
                    current_t,
                    Event::RiskBroadcasted {
                        risk_id,
                        peril_region,
                        limit,
                        broker_id: broker.broker_id,
                    },
                ));

                // Set quote deadlines (simplified timing)
                events.push((
                    current_t + 1,
                    Event::LeadQuoteConsolidationDeadline { risk_id },
                ));
                events.push((current_t + 2, Event::LeadQuoteSelectionDeadline { risk_id }));
                events.push((
                    current_t + 3,
                    Event::FollowQuoteConsolidationDeadline { risk_id },
                ));
                events.push((
                    current_t + 4,
                    Event::FollowQuoteSelectionDeadline { risk_id },
                ));
            }
        }

        events
    }

    fn handle_lead_quote_accepted(&mut self, risk_id: usize) {
        // O(1) HashMap lookup instead of O(N) event broadcast to N individual broker agents
        // With 25 brokers, this is 25x more efficient than the individual broker approach
        if let Some(&broker_id) = self.risk_ownership.get(&risk_id) {
            self.brokers[broker_id].stats.risks_bound += 1;
        }
    }
}

impl Agent<Event, Stats> for BrokerPool {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Day => Response::events(self.generate_risks_for_all_brokers(current_t)),
            Event::LeadQuoteAccepted { risk_id, .. } => {
                self.handle_lead_quote_accepted(*risk_id);
                Response::new()
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        // Return aggregated stats across all brokers in the pool
        let mut aggregated = BrokerStats::new(0); // ID 0 represents the pool

        for broker in &self.brokers {
            aggregated.risks_generated += broker.stats.risks_generated;
            aggregated.risks_bound += broker.stats.risks_bound;
        }

        Stats::BrokerStats(aggregated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broker_pool_initializes() {
        let config = ModelConfig::default();
        let pool = BrokerPool::new(25, config, 12345);
        assert_eq!(pool.brokers.len(), 25);
        assert_eq!(pool.risk_ownership.len(), 0);
    }

    #[test]
    fn test_broker_pool_generates_risks() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(25, config, 12345);

        let mut total_risks = 0;
        for _ in 0..100 {
            let resp = pool.act(0, &Event::Day);
            let risk_broadcasts = resp
                .events
                .iter()
                .filter(|(_, e)| matches!(e, Event::RiskBroadcasted { .. }))
                .count();
            total_risks += risk_broadcasts;
        }

        // With 25 brokers and λ=0.06 each, expect around 150 risks over 100 days
        assert!(total_risks > 50);
        assert!(total_risks < 500); // Very unlikely to get >500
    }

    #[test]
    fn test_broker_pool_tracks_ownership() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(5, config, 12345);

        // Generate some risks
        for _ in 0..10 {
            pool.act(0, &Event::Day);
        }

        // Should have tracked ownership for generated risks
        assert!(!pool.risk_ownership.is_empty());

        // Each risk should belong to a valid broker
        for &broker_id in pool.risk_ownership.values() {
            assert!(broker_id < 5);
        }
    }

    #[test]
    fn test_broker_pool_tracks_bound_risks() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(5, config, 12345);

        // Generate risks until we get at least one
        let mut risk_id = None;
        for _ in 0..100 {
            let resp = pool.act(0, &Event::Day);
            for (_, event) in resp.events {
                if let Event::RiskBroadcasted { risk_id: rid, .. } = event {
                    risk_id = Some(rid);
                    break;
                }
            }
            if risk_id.is_some() {
                break;
            }
        }

        let risk_id = risk_id.expect("Should have generated at least one risk");

        // Find which broker owns this risk
        let broker_id = pool.risk_ownership.get(&risk_id).copied().unwrap();
        let initial_bound = pool.brokers[broker_id].stats.risks_bound;

        // Simulate the risk being accepted
        pool.act(
            0,
            &Event::LeadQuoteAccepted {
                risk_id,
                syndicate_id: 0,
            },
        );

        // Should now show as bound for the owning broker
        assert_eq!(
            pool.brokers[broker_id].stats.risks_bound,
            initial_bound + 1,
            "BrokerPool should track when risks are bound"
        );
    }

    #[test]
    fn test_broker_pool_unique_risk_ids() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(25, config, 12345);

        // Generate many risks
        for _ in 0..100 {
            pool.act(0, &Event::Day);
        }

        // All risk IDs should be unique
        let risk_ids: Vec<_> = pool.risk_ownership.keys().copied().collect();
        let unique_count = risk_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(
            unique_count,
            risk_ids.len(),
            "All risk IDs should be unique"
        );
    }

    #[test]
    fn test_broker_pool_aggregates_stats() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(25, config, 12345);

        // Generate risks over multiple days
        for _ in 0..100 {
            pool.act(0, &Event::Day);
        }

        // Get stats
        let stats = pool.stats();
        if let Stats::BrokerStats(broker_stats) = stats {
            // Should aggregate across all 25 brokers
            let total_generated: usize = pool.brokers.iter().map(|b| b.stats.risks_generated).sum();
            assert_eq!(
                broker_stats.risks_generated, total_generated,
                "Stats should aggregate risks_generated across all brokers"
            );

            // With 25 brokers and λ=0.06 per day, expect ~150 total risks over 100 days
            assert!(broker_stats.risks_generated > 50);
            assert!(broker_stats.risks_generated < 500);
        } else {
            panic!("Expected BrokerStats");
        }
    }

    #[test]
    fn test_broker_pool_sets_deadlines() {
        let config = ModelConfig::default();
        let mut pool = BrokerPool::new(5, config, 12345);

        let resp = pool.act(0, &Event::Day);

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
}
