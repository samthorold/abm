use des::{Agent, Response};
use std::collections::HashMap;
use crate::{Event, Stats, CentralRiskRepositoryStats, Risk, Quote, Policy};

/// Central repository that tracks all risks, quotes, and policies
pub struct CentralRiskRepository {
    risks: HashMap<usize, Risk>,
    lead_quotes: HashMap<usize, Vec<Quote>>, // risk_id -> quotes
    follow_quotes: HashMap<usize, Vec<Quote>>, // risk_id -> quotes
    policies: HashMap<usize, Policy>, // risk_id -> policy
    stats: CentralRiskRepositoryStats,
}

impl CentralRiskRepository {
    pub fn new() -> Self {
        Self {
            risks: HashMap::new(),
            lead_quotes: HashMap::new(),
            follow_quotes: HashMap::new(),
            policies: HashMap::new(),
            stats: CentralRiskRepositoryStats::new(),
        }
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

    fn register_lead_quote(&mut self, risk_id: usize, syndicate_id: usize, price: f64, line_size: f64) {
        let quote = Quote {
            syndicate_id,
            price,
            line_size,
        };
        self.lead_quotes.entry(risk_id).or_insert_with(Vec::new).push(quote);
        self.stats.total_lead_quotes += 1;
    }

    fn register_follow_quote(&mut self, risk_id: usize, syndicate_id: usize, line_size: f64) {
        let quote = Quote {
            syndicate_id,
            price: 0.0, // Followers accept lead price
            line_size,
        };
        self.follow_quotes.entry(risk_id).or_insert_with(Vec::new).push(quote);
        self.stats.total_follow_quotes += 1;
    }

    fn select_lead(&mut self, risk_id: usize, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        if let Some(quotes) = self.lead_quotes.get(&risk_id) {
            if !quotes.is_empty() {
                // Select cheapest quote
                let best_quote = quotes.iter()
                    .min_by(|a, b| a.price.partial_cmp(&b.price).unwrap())
                    .unwrap();

                // Notify winning syndicate
                events.push((
                    current_t,
                    Event::LeadQuoteAccepted {
                        risk_id,
                        syndicate_id: best_quote.syndicate_id,
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
                    events.push((
                        current_t,
                        Event::FollowQuoteAccepted {
                            risk_id,
                            syndicate_id: quote.syndicate_id,
                            line_size: allocated_line,
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

    fn apply_attritional_loss(&self, risk_id: usize, amount: f64, current_t: usize) -> Vec<(usize, Event)> {
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

    fn apply_catastrophe_loss(&self, peril_region: usize, total_loss: f64, current_t: usize) -> Vec<(usize, Event)> {
        let mut events = Vec::new();

        // Find all risks in the affected peril region
        let affected_risks: Vec<usize> = self.risks.iter()
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
            Event::RiskBroadcasted { risk_id, peril_region, limit, broker_id } => {
                self.register_risk(*risk_id, *peril_region, *limit, *broker_id);
                Response::new()
            }
            Event::LeadQuoteOffered { risk_id, syndicate_id, price, line_size } => {
                self.register_lead_quote(*risk_id, *syndicate_id, *price, *line_size);
                Response::new()
            }
            Event::FollowQuoteOffered { risk_id, syndicate_id, line_size } => {
                self.register_follow_quote(*risk_id, *syndicate_id, *line_size);
                Response::new()
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
            Event::CatastropheLossOccurred { peril_region, total_loss } => {
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
        let mut repo = CentralRiskRepository::new();
        repo.register_risk(1, 0, 10_000_000.0, 0);
        assert_eq!(repo.stats.total_risks, 1);
        assert!(repo.risks.contains_key(&1));
    }

    #[test]
    fn test_select_lead_cheapest() {
        let mut repo = CentralRiskRepository::new();
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
        let mut repo = CentralRiskRepository::new();

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
}
