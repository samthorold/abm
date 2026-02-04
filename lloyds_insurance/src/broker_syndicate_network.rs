use des::{Agent, Response};
use rand::Rng;
use rand::rngs::StdRng;
use rand::SeedableRng;
use crate::{Event, Stats, BrokerStats, ModelConfig};

/// Broker-Syndicate Network that connects risks to syndicates for quoting
pub struct BrokerSyndicateNetwork {
    config: ModelConfig,
    num_syndicates: usize,
    rng: StdRng,
}

impl BrokerSyndicateNetwork {
    pub fn new(config: ModelConfig, num_syndicates: usize, seed: u64) -> Self {
        Self {
            config,
            num_syndicates,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn select_syndicates_for_lead(&mut self, _risk_id: usize) -> Vec<usize> {
        // Random topology: randomly select top_k syndicates for lead quotes
        let mut syndicates: Vec<usize> = (0..self.num_syndicates).collect();

        // Shuffle and take top_k
        for i in 0..syndicates.len() {
            let j = self.rng.gen_range(i..syndicates.len());
            syndicates.swap(i, j);
        }

        syndicates.into_iter().take(self.config.lead_top_k).collect()
    }

    fn select_syndicates_for_follow(&mut self, _risk_id: usize, _lead_syndicate: usize) -> Vec<usize> {
        // Random topology: randomly select top_k syndicates for follow quotes
        // Exclude the lead syndicate
        let mut syndicates: Vec<usize> = (0..self.num_syndicates).collect();

        // Shuffle and take top_k
        for i in 0..syndicates.len() {
            let j = self.rng.gen_range(i..syndicates.len());
            syndicates.swap(i, j);
        }

        syndicates.into_iter().take(self.config.follow_top_k).collect()
    }
}

impl Agent<Event, Stats> for BrokerSyndicateNetwork {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::RiskBroadcasted { risk_id, .. } => {
                let mut events = Vec::new();

                // Request lead quotes from selected syndicates
                let lead_syndicates = self.select_syndicates_for_lead(*risk_id);
                for syndicate_id in lead_syndicates {
                    events.push((
                        current_t,
                        Event::LeadQuoteRequested {
                            risk_id: *risk_id,
                            syndicate_id,
                        },
                    ));
                }

                Response::events(events)
            }
            Event::LeadQuoteAccepted { risk_id, syndicate_id } => {
                // Once lead is selected, request follow quotes
                let mut events = Vec::new();

                let follow_syndicates = self.select_syndicates_for_follow(*risk_id, *syndicate_id);
                for follower_id in follow_syndicates {
                    if follower_id != *syndicate_id {
                        // Don't ask lead to follow
                        events.push((
                            current_t,
                            Event::FollowQuoteRequested {
                                risk_id: *risk_id,
                                syndicate_id: follower_id,
                                lead_price: 0.0, // Will be filled by repository
                            },
                        ));
                    }
                }

                Response::events(events)
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        // BrokerSyndicateNetwork doesn't produce meaningful stats
        Stats::BrokerStats(BrokerStats::new(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selects_lead_syndicates() {
        let config = ModelConfig::default();
        let mut network = BrokerSyndicateNetwork::new(config.clone(), 5, 12345);

        let selected = network.select_syndicates_for_lead(1);
        assert_eq!(selected.len(), config.lead_top_k);
    }

    #[test]
    fn test_responds_to_risk_broadcast() {
        let config = ModelConfig::default();
        let mut network = BrokerSyndicateNetwork::new(config, 5, 12345);

        let resp = network.act(0, &Event::RiskBroadcasted {
            risk_id: 1,
            peril_region: 0,
            limit: 10_000_000.0,
            broker_id: 0,
        });

        // Should emit LeadQuoteRequested events
        assert!(!resp.events.is_empty());
        assert!(resp.events.iter().all(|(_, e)| matches!(e, Event::LeadQuoteRequested { .. })));
    }
}
