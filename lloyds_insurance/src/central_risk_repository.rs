use crate::{CentralRiskRepositoryStats, Event, ModelConfig, Policy, Quote, Risk, Stats};
use des::{Agent, Response};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;

/// Central repository that tracks all risks, quotes, and policies
/// Also handles syndicate selection for quote requests (previously in BrokerSyndicateNetwork)
pub struct CentralRiskRepository {
    risks: HashMap<usize, Risk>,
    lead_quotes: HashMap<usize, Vec<Quote>>, // risk_id -> quotes
    follow_quotes: HashMap<usize, Vec<Quote>>, // risk_id -> quotes
    policies: HashMap<usize, Policy>,        // risk_id -> policy
    stats: CentralRiskRepositoryStats,

    // Syndicate selection (folded from BrokerSyndicateNetwork)
    config: ModelConfig,
    num_syndicates: usize,
    rng: StdRng,
}

impl CentralRiskRepository {
    pub fn new(config: ModelConfig, num_syndicates: usize, seed: u64) -> Self {
        Self {
            risks: HashMap::new(),
            lead_quotes: HashMap::new(),
            follow_quotes: HashMap::new(),
            policies: HashMap::new(),
            stats: CentralRiskRepositoryStats::new(),
            config,
            num_syndicates,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Select syndicates for lead quote requests (random topology)
    fn select_syndicates_for_lead(&mut self, _risk_id: usize) -> Vec<usize> {
        let mut syndicates: Vec<usize> = (0..self.num_syndicates).collect();

        // Shuffle and take top_k
        for i in 0..syndicates.len() {
            let j = self.rng.gen_range(i..syndicates.len());
            syndicates.swap(i, j);
        }

        syndicates
            .into_iter()
            .take(self.config.lead_top_k)
            .collect()
    }

    /// Select syndicates for follow quote requests (random topology, excluding lead)
    fn select_syndicates_for_follow(
        &mut self,
        _risk_id: usize,
        _lead_syndicate: usize,
    ) -> Vec<usize> {
        let mut syndicates: Vec<usize> = (0..self.num_syndicates).collect();

        // Shuffle and take top_k
        for i in 0..syndicates.len() {
            let j = self.rng.gen_range(i..syndicates.len());
            syndicates.swap(i, j);
        }

        syndicates
            .into_iter()
            .take(self.config.follow_top_k)
            .collect()
    }

    fn register_risk(&mut self, risk_id: usize, peril_region: usize, limit: f64, broker_id: usize) {
        let risk = Risk {
            id: risk_id,
            peril_region,
            limit,
            expiration_time: 0, // Will be set by broker
            broker_id,
        };
        self.risks.insert(risk_id, risk);
        self.stats.total_risks += 1;
    }

    fn register_lead_quote(
        &mut self,
        risk_id: usize,
        syndicate_id: usize,
        price: f64,
        line_size: f64,
    ) {
        let quote = Quote {
            syndicate_id,
            price,
            line_size,
        };
        self.lead_quotes.entry(risk_id).or_default().push(quote);
        self.stats.total_lead_quotes += 1;
    }

    fn register_follow_quote(&mut self, risk_id: usize, syndicate_id: usize, line_size: f64) {
        let quote = Quote {
            syndicate_id,
            price: 0.0, // Followers accept lead price
            line_size,
        };
        self.follow_quotes.entry(risk_id).or_default().push(quote);
        self.stats.total_follow_quotes += 1;
    }

    fn select_lead(&mut self, risk_id: usize, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        if let Some(quotes) = self.lead_quotes.get(&risk_id)
            && !quotes.is_empty()
        {
            // Select cheapest quote
            let best_quote = quotes
                .iter()
                .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
                .unwrap();

            // Notify winning syndicate
            let risk = self.risks.get(&risk_id).expect("Risk should exist");
            events.push((
                current_t,
                Event::LeadQuoteAccepted {
                    risk_id,
                    syndicate_id: best_quote.syndicate_id,
                    peril_region: risk.peril_region,
                    risk_limit: risk.limit,
                },
            ));

            // Create policy (will be completed with followers later)
            let policy = Policy {
                risk_id,
                lead_syndicate_id: best_quote.syndicate_id,
                lead_price: best_quote.price,
                lead_line_size: best_quote.line_size,
                followers: Vec::new(),
            };
            self.policies.insert(risk_id, policy);
            self.stats.total_policies += 1;
        }

        events
    }

    fn select_followers(&mut self, risk_id: usize, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Check if policy exists (lead selected)
        if let Some(policy) = self.policies.get(&risk_id) {
            let mut remaining_line = 1.0 - policy.lead_line_size;

            if let Some(quotes) = self.follow_quotes.get(&risk_id) {
                // Sort by line size descending (prioritize larger followers)
                let mut sorted_quotes = quotes.clone();
                sorted_quotes.sort_by(|a, b| b.line_size.partial_cmp(&a.line_size).unwrap());

                for quote in sorted_quotes {
                    if remaining_line <= 0.0 {
                        break;
                    }

                    let allocated_line = quote.line_size.min(remaining_line);
                    remaining_line -= allocated_line;

                    // Notify follower
                    let risk = self.risks.get(&risk_id).expect("Risk should exist");
                    events.push((
                        current_t,
                        Event::FollowQuoteAccepted {
                            risk_id,
                            syndicate_id: quote.syndicate_id,
                            line_size: allocated_line,
                            peril_region: risk.peril_region,
                            risk_limit: risk.limit,
                        },
                    ));

                    // Add to policy
                    if let Some(policy) = self.policies.get_mut(&risk_id) {
                        policy.followers.push((quote.syndicate_id, allocated_line));
                    }
                }
            }
        }

        events
    }

    fn apply_attritional_loss(
        &self,
        risk_id: usize,
        amount: f64,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        if let Some(policy) = self.policies.get(&risk_id) {
            // Apply loss to lead
            let lead_loss = amount * policy.lead_line_size;
            events.push((
                current_t,
                Event::ClaimReceived {
                    risk_id,
                    syndicate_id: policy.lead_syndicate_id,
                    amount: lead_loss,
                },
            ));

            // Apply loss to followers
            for (syndicate_id, line_size) in &policy.followers {
                let follower_loss = amount * line_size;
                events.push((
                    current_t,
                    Event::ClaimReceived {
                        risk_id,
                        syndicate_id: *syndicate_id,
                        amount: follower_loss,
                    },
                ));
            }
        }

        events
    }

    fn apply_catastrophe_loss(
        &self,
        peril_region: usize,
        total_loss: f64,
        current_t: usize,
    ) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Find all risks in the affected peril region
        let affected_risks: Vec<usize> = self
            .risks
            .iter()
            .filter(|(_, risk)| risk.peril_region == peril_region)
            .map(|(id, _)| *id)
            .collect();

        if affected_risks.is_empty() {
            return events;
        }

        // Distribute loss equally among affected risks (simplified)
        let loss_per_risk = total_loss / affected_risks.len() as f64;

        for risk_id in affected_risks {
            if let Some(policy) = self.policies.get(&risk_id) {
                // Apply to lead
                let lead_loss = loss_per_risk * policy.lead_line_size;
                events.push((
                    current_t,
                    Event::ClaimReceived {
                        risk_id,
                        syndicate_id: policy.lead_syndicate_id,
                        amount: lead_loss,
                    },
                ));

                // Apply to followers
                for (syndicate_id, line_size) in &policy.followers {
                    let follower_loss = loss_per_risk * line_size;
                    events.push((
                        current_t,
                        Event::ClaimReceived {
                            risk_id,
                            syndicate_id: *syndicate_id,
                            amount: follower_loss,
                        },
                    ));
                }
            }
        }

        events
    }
}

impl Agent<Event, Stats> for CentralRiskRepository {
    fn act(&mut self, current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::RiskBroadcasted {
                risk_id,
                peril_region,
                limit,
                broker_id,
            } => {
                self.register_risk(*risk_id, *peril_region, *limit, *broker_id);

                // Select syndicates and request lead quotes (folded from BrokerSyndicateNetwork)
                let mut events = Vec::new();
                let lead_syndicates = self.select_syndicates_for_lead(*risk_id);
                for syndicate_id in lead_syndicates {
                    events.push((
                        current_t,
                        Event::LeadQuoteRequested {
                            risk_id: *risk_id,
                            syndicate_id,
                            peril_region: *peril_region,
                            risk_limit: *limit,
                        },
                    ));
                }

                Response::events(events)
            }
            Event::LeadQuoteOffered {
                risk_id,
                syndicate_id,
                price,
                line_size,
            } => {
                self.register_lead_quote(*risk_id, *syndicate_id, *price, *line_size);
                Response::new()
            }
            Event::FollowQuoteOffered {
                risk_id,
                syndicate_id,
                line_size,
            } => {
                self.register_follow_quote(*risk_id, *syndicate_id, *line_size);
                Response::new()
            }
            Event::LeadQuoteAccepted {
                risk_id,
                syndicate_id,
                ..
            } => {
                // Once lead is selected, request follow quotes (folded from BrokerSyndicateNetwork)
                let mut events = Vec::new();
                let follow_syndicates = self.select_syndicates_for_follow(*risk_id, *syndicate_id);
                for follower_id in follow_syndicates {
                    if follower_id != *syndicate_id {
                        // Don't ask lead to follow
                        let risk = self.risks.get(risk_id).expect("Risk should exist");
                        let policy = self.policies.get(risk_id).expect("Policy should exist");
                        events.push((
                            current_t,
                            Event::FollowQuoteRequested {
                                risk_id: *risk_id,
                                syndicate_id: follower_id,
                                lead_price: policy.lead_price,
                                peril_region: risk.peril_region,
                                risk_limit: risk.limit,
                            },
                        ));
                    }
                }

                Response::events(events)
            }
            Event::LeadQuoteSelectionDeadline { risk_id } => {
                Response::events(self.select_lead(*risk_id, current_t))
            }
            Event::FollowQuoteSelectionDeadline { risk_id } => {
                Response::events(self.select_followers(*risk_id, current_t))
            }
            Event::AttritionalLossOccurred { risk_id, amount } => {
                Response::events(self.apply_attritional_loss(*risk_id, *amount, current_t))
            }
            Event::CatastropheLossOccurred {
                peril_region,
                total_loss,
            } => {
                Response::events(self.apply_catastrophe_loss(*peril_region, *total_loss, current_t))
            }
            _ => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::CentralRiskRepositoryStats(self.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_risk() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config, 5, 12345);
        repo.register_risk(1, 0, 10_000_000.0, 0);
        assert_eq!(repo.stats.total_risks, 1);
        assert!(repo.risks.contains_key(&1));
    }

    #[test]
    fn test_select_lead_cheapest() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config, 5, 12345);
        repo.register_risk(1, 0, 10_000_000.0, 0);
        repo.register_lead_quote(1, 0, 300_000.0, 0.5);
        repo.register_lead_quote(1, 1, 250_000.0, 0.5); // Cheaper

        let events = repo.select_lead(1, 0);
        assert_eq!(events.len(), 1);

        match &events[0].1 {
            Event::LeadQuoteAccepted { syndicate_id, .. } => {
                assert_eq!(*syndicate_id, 1); // Syndicate 1 has cheaper quote
            }
            _ => panic!("Expected LeadQuoteAccepted event"),
        }
    }

    #[test]
    fn test_catastrophe_cascade() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config, 5, 12345);

        // Create 3 risks in peril region 0
        for i in 0..3 {
            repo.register_risk(i, 0, 10_000_000.0, 0);
            repo.register_lead_quote(i, 0, 300_000.0, 1.0);
            repo.select_lead(i, 0);
        }

        // Apply catastrophe to region 0
        let events = repo.apply_catastrophe_loss(0, 30_000_000.0, 0);

        // Should generate 3 claim events (one per risk)
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_selects_lead_syndicates() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config.clone(), 5, 12345);

        let selected = repo.select_syndicates_for_lead(1);
        assert_eq!(selected.len(), config.lead_top_k);
    }

    #[test]
    fn test_responds_to_risk_broadcast_with_lead_quote_requests() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config, 5, 12345);

        let resp = repo.act(
            0,
            &Event::RiskBroadcasted {
                risk_id: 1,
                peril_region: 0,
                limit: 10_000_000.0,
                broker_id: 0,
            },
        );

        // Should emit LeadQuoteRequested events
        assert!(!resp.events.is_empty());
        assert!(
            resp.events
                .iter()
                .all(|(_, e)| matches!(e, Event::LeadQuoteRequested { .. }))
        );
    }

    #[test]
    fn test_responds_to_lead_quote_accepted_with_follow_requests() {
        let config = ModelConfig::default();
        let mut repo = CentralRiskRepository::new(config, 5, 12345);

        // First register risk so it can be looked up
        repo.act(
            0,
            &Event::RiskBroadcasted {
                risk_id: 1,
                peril_region: 0,
                limit: 10_000_000.0,
                broker_id: 0,
            },
        );

        // Submit a lead quote so we have something to select
        repo.act(
            0,
            &Event::LeadQuoteOffered {
                risk_id: 1,
                syndicate_id: 0,
                price: 300_000.0,
                line_size: 0.5,
            },
        );

        // Trigger lead selection - this creates the policy and emits LeadQuoteAccepted
        let resp = repo.act(0, &Event::LeadQuoteSelectionDeadline { risk_id: 1 });

        // Should have emitted LeadQuoteAccepted
        assert!(
            resp.events
                .iter()
                .any(|(_, e)| matches!(e, Event::LeadQuoteAccepted { .. }))
        );

        // Now handle the LeadQuoteAccepted to get follow requests
        let lead_accepted_event = resp
            .events
            .iter()
            .find(|(_, e)| matches!(e, Event::LeadQuoteAccepted { .. }))
            .unwrap();

        let follow_resp = repo.act(0, &lead_accepted_event.1);

        // Should emit FollowQuoteRequested events
        assert!(!follow_resp.events.is_empty());
        assert!(
            follow_resp
                .events
                .iter()
                .all(|(_, e)| matches!(e, Event::FollowQuoteRequested { .. }))
        );
    }
}
