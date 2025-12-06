use crate::*;
use des::{Agent, Response};

/// Buyer agent for the minimal model
/// Decisions: (1) Which seller to visit, (2) Accept or reject price
pub struct BuyerAgent {
    pub id: usize,
    pub p_out: usize, // Resale price (value of good)

    // Classifier systems
    seller_choice_rules: Vec<Rule<(), usize>>, // Unconditional rules, action = seller_id
    price_acceptance_rules: Vec<Rule<usize, bool>>, // Condition = price, action = accept/reject

    // Daily state (pub for coordinator access)
    current_day: usize,
    pub visited_seller: Option<usize>,
    pub price_offered: Option<usize>,
    pub transaction_completed: bool,
    pub denied_service: bool,

    // Active rules (for learning)
    active_seller_choice_rule: Option<usize>,
    active_price_rule: Option<usize>,

    // RNG
    rng: rand::rngs::StdRng,
}

impl BuyerAgent {
    pub fn new(id: usize, p_out: usize, n_sellers: usize, max_price: usize, seed: u64) -> Self {
        use rand::SeedableRng;

        // Initialize seller choice rules (one per seller, unconditional)
        let seller_choice_rules: Vec<_> = (0..n_sellers)
            .map(|seller_id| Rule::new((), seller_id))
            .collect();

        // Initialize price acceptance rules (one for each price × {accept, reject})
        let mut price_acceptance_rules = Vec::new();
        for price in 0..=max_price {
            price_acceptance_rules.push(Rule::new(price, true));  // Accept
            price_acceptance_rules.push(Rule::new(price, false)); // Reject
        }

        BuyerAgent {
            id,
            p_out,
            seller_choice_rules,
            price_acceptance_rules,
            current_day: 0,
            visited_seller: None,
            price_offered: None,
            transaction_completed: false,
            denied_service: false,
            active_seller_choice_rule: None,
            active_price_rule: None,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Choose which seller to visit (pub for main loop)
    pub fn choose_seller(&mut self) -> usize {
        let idx = stochastic_auction(&self.seller_choice_rules, 0.1, 0.025, &mut self.rng);
        self.active_seller_choice_rule = Some(idx);
        self.seller_choice_rules[idx].action
    }

    /// Respond to a price offer (pub for coordinator access)
    pub fn respond_to_price(&mut self, price: usize) -> bool {
        self.price_offered = Some(price);

        // Find applicable rules for this price
        let applicable: Vec<(usize, &Rule<usize, bool>)> = self.price_acceptance_rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.condition == price)
            .collect();

        let applicable_rules: Vec<_> = applicable.iter().map(|(_, r)| (*r).clone()).collect();
        let local_idx = stochastic_auction(&applicable_rules, 0.1, 0.025, &mut self.rng);
        let global_idx = applicable[local_idx].0;

        self.active_price_rule = Some(global_idx);
        self.price_acceptance_rules[global_idx].action
    }

    /// Update rule strengths based on day's outcomes (pub for main loop)
    pub fn update_strengths(&mut self, learning_rate: f64) {
        // Seller choice reward
        if let Some(rule_idx) = self.active_seller_choice_rule {
            let reward = if self.transaction_completed {
                // Positive reward based on surplus
                if let Some(price) = self.price_offered {
                    let surplus = self.p_out as i32 - price as i32;
                    surplus.max(0) as f64 / self.p_out as f64
                } else {
                    0.0
                }
            } else if self.denied_service {
                // Service denial - neutral (seller may have legitimately sold out)
                0.0
            } else {
                // Rejected price - evaluate after session ends
                0.0
            };

            let old_strength = self.seller_choice_rules[rule_idx].strength;
            self.seller_choice_rules[rule_idx].strength =
                update_strength(old_strength, reward, learning_rate);
        }

        // Price acceptance/rejection reward
        if let Some(rule_idx) = self.active_price_rule {
            let reward = if self.transaction_completed {
                // Accepted price
                if let Some(price) = self.price_offered {
                    let surplus = self.p_out as i32 - price as i32;
                    if surplus >= 0 {
                        // Good deal - normalize to [0, 1]
                        surplus as f64 / self.p_out as f64
                    } else {
                        // Bad deal - negative reward (loss)
                        0.0
                    }
                } else {
                    0.0
                }
            } else if let Some(price) = self.price_offered {
                // Rejected price - reward for avoiding bad deals
                if price > self.p_out {
                    // Good rejection - avoided guaranteed loss
                    0.5 // Medium reward for avoiding a bad deal
                } else {
                    // Rejected a potentially good price
                    0.0
                }
            } else {
                // Denied service
                0.0
            };

            let old_strength = self.price_acceptance_rules[rule_idx].strength;
            self.price_acceptance_rules[rule_idx].strength =
                update_strength(old_strength, reward, learning_rate);
        }
    }

    /// Reset daily state (pub for main loop)
    pub fn reset_daily_state(&mut self) {
        self.visited_seller = None;
        self.price_offered = None;
        self.transaction_completed = false;
        self.denied_service = false;
        self.active_seller_choice_rule = None;
        self.active_price_rule = None;
    }
}

impl Agent<MarketEvent, MarketStats> for BuyerAgent {
    fn act(&mut self, current_t: usize, event: &MarketEvent) -> Response<MarketEvent, MarketStats> {
        match event {
            MarketEvent::BuyersChooseSellers { day, session } => {
                self.current_day = *day;
                let seller_id = self.choose_seller();
                self.visited_seller = Some(seller_id);

                // Emit event indicating choice (will be handled by MarketCoordinator)
                Response::event(current_t + 1, MarketEvent::ProcessQueues {
                    day: *day,
                    session: *session
                })
            }

            MarketEvent::Transaction {
                buyer_id,
                price,
                accepted,
                ..
            } if *buyer_id == self.id => {
                if let Some(p) = price {
                    // Seller offered a price
                    let would_accept = self.respond_to_price(*p);
                    if would_accept && *accepted {
                        self.transaction_completed = true;
                    }
                } else {
                    // Denied service (no stock)
                    self.denied_service = true;
                }
                Response::new()
            }

            MarketEvent::DayEnd { .. } => {
                // Update learning
                self.update_strengths(0.05);
                self.reset_daily_state();
                Response::new()
            }

            _ => Response::new(),
        }
    }

    fn stats(&self) -> MarketStats {
        let mut stats = MarketStats::new_buyer(self.current_day, self.id);
        if let Some(seller) = self.visited_seller {
            stats.sellers_visited.push(seller);
        }
        if let Some(price) = self.price_offered {
            stats.prices_offered.push(price);
            if self.transaction_completed {
                stats.prices_accepted.push(price);
                stats.transactions_completed = 1;
            } else if self.denied_service {
                stats.transactions_denied = 1;
            } else {
                stats.transactions_rejected = 1;
            }
        }
        stats
    }
}

/// Seller agent for the minimal model
/// Decisions: (1) Pricing (simplified: unconditional pricing rules)
/// For minimal model: Fixed supply
pub struct SellerAgent {
    pub id: usize,
    pub p_in: usize, // Purchase price for supply

    // Classifier systems
    price_rules: Vec<Rule<(), usize>>, // Unconditional pricing rules, action = price
    beta_rules: Vec<Rule<(), i32>>, // Queue handling parameter: -25 to +25 in steps of 5

    // Stock and revenue tracking (pub for coordinator access)
    pub stock: usize,
    pub initial_stock: usize,
    pub gross_revenue: usize,

    // Daily state
    current_day: usize,
    queue: Vec<usize>, // Buyer IDs in queue
    pub beta: i32, // Current queue handling parameter

    // Track rule usage for learning
    price_rule_usage: Vec<(usize, usize)>, // (times_used, total_revenue) parallel to price_rules
    active_beta_rule: Option<usize>, // Which beta rule is being used today

    // RNG
    rng: rand::rngs::StdRng,
}

impl SellerAgent {
    pub fn new(
        id: usize,
        p_in: usize,
        initial_stock: usize,
        max_price: usize,
        seed: u64,
    ) -> Self {
        use rand::SeedableRng;

        // Initialize pricing rules (one per possible price, unconditional)
        let price_rules: Vec<_> = (0..=max_price)
            .map(|price| Rule::new((), price))
            .collect();

        let n_rules = price_rules.len();

        // Initialize beta rules: -25 to +25 in steps of 5
        let beta_rules: Vec<_> = (-25..=25)
            .step_by(5)
            .map(|beta| Rule::new((), beta))
            .collect();

        SellerAgent {
            id,
            p_in,
            price_rules,
            beta_rules,
            stock: initial_stock,
            initial_stock,
            gross_revenue: 0,
            current_day: 0,
            queue: Vec::new(),
            beta: 0, // Start with neutral queue handling
            price_rule_usage: vec![(0, 0); n_rules],
            active_beta_rule: None,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Choose a price to offer (pub for coordinator access)
    /// Returns (price, rule_idx) so we can track which rule was used
    pub fn choose_price(&mut self) -> (usize, usize) {
        let idx = stochastic_auction(&self.price_rules, 0.1, 0.025, &mut self.rng);
        let price = self.price_rules[idx].action;
        (price, idx)
    }

    /// Record that a price rule was used and whether it was accepted
    /// This tracks usage for learning updates
    pub fn record_price_offer(&mut self, rule_idx: usize, accepted: bool, price: usize) {
        let (ref mut times_used, ref mut total_revenue) = self.price_rule_usage[rule_idx];
        *times_used += 1;
        if accepted {
            *total_revenue += price;
        }
    }

    /// Choose beta parameter for queue handling (pub for coordinator access)
    pub fn choose_beta(&mut self) {
        let idx = stochastic_auction(&self.beta_rules, 0.1, 0.025, &mut self.rng);
        self.beta = self.beta_rules[idx].action;
        self.active_beta_rule = Some(idx);
    }

    /// Update pricing rule strengths based on revenue (pub for main loop)
    pub fn update_strengths(&mut self, learning_rate: f64) {
        let max_price = self.price_rules.len() - 1;

        // Update each pricing rule based on its actual usage and performance
        for (idx, rule) in self.price_rules.iter_mut().enumerate() {
            let (times_used, total_revenue) = self.price_rule_usage[idx];

            let reward = if times_used > 0 {
                // Average revenue per use, normalized to [0, 1]
                let avg_revenue = total_revenue as f64 / times_used as f64;
                avg_revenue / max_price as f64
            } else {
                // Rule not used - no update (keeps current strength)
                // This maintains exploration potential
                continue;
            };

            rule.strength = update_strength(rule.strength, reward, learning_rate);
        }

        // Update beta rule based on gross revenue
        if let Some(beta_idx) = self.active_beta_rule {
            let max_possible_revenue = max_price * self.initial_stock;
            let reward = if max_possible_revenue > 0 {
                self.gross_revenue as f64 / max_possible_revenue as f64
            } else {
                0.0
            };

            self.beta_rules[beta_idx].strength =
                update_strength(self.beta_rules[beta_idx].strength, reward, learning_rate);
        }
    }

    /// Reset for new day (pub for main loop)
    pub fn reset_daily_state(&mut self) {
        self.stock = self.initial_stock;
        self.gross_revenue = 0;
        self.queue.clear();
        self.beta = 0; // Will be chosen again
        self.active_beta_rule = None;
        // Reset usage tracking for new day
        for usage in &mut self.price_rule_usage {
            *usage = (0, 0);
        }
    }
}

impl Agent<MarketEvent, MarketStats> for SellerAgent {
    fn act(&mut self, _current_t: usize, event: &MarketEvent) -> Response<MarketEvent, MarketStats> {
        match event {
            MarketEvent::SessionStart { day, .. } => {
                self.current_day = *day;
                Response::new()
            }

            MarketEvent::BuyersChooseSellers { .. } => {
                // Buyers are making choices; we'll learn about our queue next
                Response::new()
            }

            MarketEvent::ProcessQueues { day, session } => {
                // This is where we would process our queue
                // For now, emit transaction events for buyers in queue
                // Note: The actual queue will be managed by MarketCoordinator
                // This is placeholder logic
                Response::new()
            }

            MarketEvent::DayEnd { .. } => {
                // Update learning
                self.update_strengths(0.05);
                self.reset_daily_state();
                Response::new()
            }

            _ => Response::new(),
        }
    }

    fn stats(&self) -> MarketStats {
        let mut stats = MarketStats::new_seller(self.current_day, self.id);
        stats.revenue = Some(self.gross_revenue);
        stats.stock_remaining = Some(self.stock);
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buyer_learns_to_reject_bad_prices() {
        let mut buyer = BuyerAgent::new(0, 15, 10, 20, 42); // p_out = 15

        // Simulate many days where high prices are offered
        for _ in 0..100 {
            buyer.reset_daily_state();

            // Offer price = 20 (above p_out)
            let accepts = buyer.respond_to_price(20);

            if accepts {
                // If accidentally accepted, gets 0 reward (bad deal)
                buyer.transaction_completed = true;
            }
            // If rejected, gets 0.5 reward (good decision)

            buyer.update_strengths(0.05);
        }

        // Find the reject rule for price=20
        let reject_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 20 && r.action == false)
            .unwrap();

        let accept_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 20 && r.action == true)
            .unwrap();

        // Reject rule should be stronger than accept rule
        assert!(
            buyer.price_acceptance_rules[reject_rule_idx].strength
                > buyer.price_acceptance_rules[accept_rule_idx].strength,
            "Buyer should learn to reject prices above p_out"
        );
    }

    #[test]
    fn test_buyer_learns_to_accept_good_prices() {
        let mut buyer = BuyerAgent::new(0, 15, 10, 20, 42); // p_out = 15

        // Simulate many days where good prices are offered
        for _ in 0..100 {
            buyer.reset_daily_state();

            // Offer price = 10 (below p_out, good deal)
            let accepts = buyer.respond_to_price(10);

            if accepts {
                buyer.transaction_completed = true;
            }

            buyer.update_strengths(0.05);
        }

        // Find the accept rule for price=10
        let accept_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 10 && r.action == true)
            .unwrap();

        let reject_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 10 && r.action == false)
            .unwrap();

        // Accept rule should be stronger than reject rule for good prices
        assert!(
            buyer.price_acceptance_rules[accept_rule_idx].strength
                > buyer.price_acceptance_rules[reject_rule_idx].strength,
            "Buyer should learn to accept prices below p_out"
        );
    }

    #[test]
    fn test_seller_tracks_price_rule_usage() {
        let mut seller = SellerAgent::new(0, 9, 15, 20, 42);

        // Choose price multiple times
        let (price1, idx1) = seller.choose_price();
        let (price2, idx2) = seller.choose_price();

        // Record outcomes: first accepted, second rejected
        seller.record_price_offer(idx1, true, price1);
        seller.record_price_offer(idx2, false, price2);

        // Check that usage was tracked
        let (times1, revenue1) = seller.price_rule_usage[idx1];
        assert_eq!(times1, 1);
        assert_eq!(revenue1, price1);

        let (times2, revenue2) = seller.price_rule_usage[idx2];
        assert_eq!(times2, 1);
        assert_eq!(revenue2, 0); // Rejected, no revenue
    }

    #[test]
    fn test_seller_learning_differentiates_by_performance() {
        let mut seller = SellerAgent::new(0, 9, 15, 20, 42);

        // Manually set up usage: high price (15) used and accepted, low price (5) used and rejected
        let high_price_idx = 15;
        let low_price_idx = 5;

        seller.record_price_offer(high_price_idx, true, 15);
        seller.record_price_offer(low_price_idx, false, 5);

        let strength_before_high = seller.price_rules[high_price_idx].strength;
        let strength_before_low = seller.price_rules[low_price_idx].strength;

        // Update strengths
        seller.update_strengths(0.05);

        let strength_after_high = seller.price_rules[high_price_idx].strength;
        let strength_after_low = seller.price_rules[low_price_idx].strength;

        // Both should weaken from initial 1.0, but high price should weaken less
        // (converging toward their actual rewards: 0.75 for high, 0.0 for low)
        assert!(
            strength_after_low < strength_before_low,
            "Low price rule should weaken toward 0"
        );

        assert!(
            strength_after_high > strength_after_low,
            "High price rule should be stronger than low price rule after update"
        );

        // High price should be closer to its reward (0.75)
        let expected_high = 0.95 * 1.0 + 0.05 * (15.0 / 20.0);
        assert!(
            (strength_after_high - expected_high).abs() < 0.001,
            "High price strength should converge toward 0.75"
        );
    }

    #[test]
    fn test_seller_resets_usage_tracking() {
        let mut seller = SellerAgent::new(0, 9, 15, 20, 42);

        // Use some rules
        let (price, idx) = seller.choose_price();
        seller.record_price_offer(idx, true, price);

        // Verify usage was tracked
        let (times, _) = seller.price_rule_usage[idx];
        assert_eq!(times, 1);

        // Reset for new day
        seller.reset_daily_state();

        // Verify usage was reset
        for (times, revenue) in &seller.price_rule_usage {
            assert_eq!(*times, 0);
            assert_eq!(*revenue, 0);
        }
    }

    #[test]
    fn test_seller_convergence_to_profitable_price() {
        let mut seller = SellerAgent::new(0, 9, 15, 20, 42);

        // Simulate many days where price 12 is always accepted, others rejected
        for _ in 0..100 {
            seller.reset_daily_state();

            // Simulate 10 offers per day
            for _ in 0..10 {
                let (price, idx) = seller.choose_price();
                let accepted = price == 12;
                seller.record_price_offer(idx, accepted, price);
            }

            seller.update_strengths(0.05);
        }

        // Rule for price 12 should be strongest
        let price_12_strength = seller.price_rules[12].strength;
        let mut is_strongest = true;
        for (idx, rule) in seller.price_rules.iter().enumerate() {
            if idx != 12 && rule.strength > price_12_strength {
                is_strongest = false;
                break;
            }
        }

        assert!(
            is_strongest,
            "Price 12 rule should be strongest after consistent acceptance"
        );
    }
}
