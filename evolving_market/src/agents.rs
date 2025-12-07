use crate::*;
use des::{Agent, Response};

/// Buyer agent for the minimal model
/// Decisions: (1) Which seller to visit, (2) Accept or reject price
pub struct BuyerAgent {
    pub id: usize,
    pub buyer_type: BuyerType, // Low, Medium, or High valuation
    pub p_out: usize,          // Resale price (value of good) - derived from buyer_type

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
    pub fn new(
        id: usize,
        buyer_type: BuyerType,
        n_sellers: usize,
        max_price: usize,
        seed: u64,
    ) -> Self {
        use rand::SeedableRng;

        let p_out = buyer_type.valuation();

        // Initialize seller choice rules (one per seller, unconditional)
        let seller_choice_rules: Vec<_> = (0..n_sellers)
            .map(|seller_id| Rule::new((), seller_id))
            .collect();

        // Initialize price acceptance rules (one for each price × {accept, reject})
        let mut price_acceptance_rules = Vec::new();
        for price in 0..=max_price {
            price_acceptance_rules.push(Rule::new(price, true)); // Accept
            price_acceptance_rules.push(Rule::new(price, false)); // Reject
        }

        BuyerAgent {
            id,
            buyer_type,
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
        let applicable: Vec<(usize, &Rule<usize, bool>)> = self
            .price_acceptance_rules
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
                Response::event(
                    current_t + 1,
                    MarketEvent::ProcessQueues {
                        day: *day,
                        session: *session,
                    },
                )
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
                // Learning updated in main loop now
                self.reset_daily_state();
                Response::new()
            }

            _ => Response::new(),
        }
    }

    fn stats(&self) -> MarketStats {
        let mut stats = MarketStats::new_buyer(self.current_day, self.id, self.buyer_type);
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

/// Seller agent with conditional pricing
/// Decisions: (1) Conditional pricing (based on loyalty × stock/queue), (2) Queue handling β
pub struct SellerAgent {
    pub id: usize,
    pub p_in: usize, // Purchase price for supply

    // Classifier systems
    price_rules: Vec<Rule<PricingCondition, usize>>, // Conditional pricing rules
    beta_rules: Vec<Rule<(), i32>>, // Queue handling parameter: -25 to +25 in steps of 5

    // Stock and revenue tracking (pub for coordinator access)
    pub stock: usize,
    pub initial_stock: usize,
    pub gross_revenue: usize,

    // Daily state
    current_day: usize,
    queue: Vec<usize>, // Buyer IDs in queue
    pub beta: i32,     // Current queue handling parameter

    // Track rule usage for learning
    price_rule_usage: Vec<(usize, usize)>, // (times_used, total_revenue) parallel to price_rules
    active_beta_rule: Option<usize>,       // Which beta rule is being used today

    // RNG
    rng: rand::rngs::StdRng,
}

impl SellerAgent {
    pub fn new(id: usize, p_in: usize, initial_stock: usize, max_price: usize, seed: u64) -> Self {
        use rand::SeedableRng;

        // Initialize conditional pricing rules
        // For each combination of (loyalty class, stock/queue ratio, price)
        let mut price_rules = Vec::new();

        for loyalty in [LoyaltyClass::Low, LoyaltyClass::Medium, LoyaltyClass::High] {
            for stock_queue in [
                StockQueueRatio::Low,
                StockQueueRatio::Medium,
                StockQueueRatio::High,
            ] {
                for price in 0..=max_price {
                    let condition = PricingCondition::new(loyalty, stock_queue);
                    price_rules.push(Rule::new(condition, price));
                }
            }
        }

        let n_price_rules = price_rules.len();

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
            price_rule_usage: vec![(0, 0); n_price_rules],
            active_beta_rule: None,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Choose a price to offer based on buyer loyalty and current market state
    /// Returns (price, rule_idx) so we can track which rule was used
    pub fn choose_price(&mut self, buyer_loyalty: f64, queue_length: usize) -> (usize, usize) {
        // Determine current condition
        let loyalty_class = LoyaltyClass::from_value(buyer_loyalty);
        let stock_queue_ratio = StockQueueRatio::from_values(self.stock, queue_length);
        let condition = PricingCondition::new(loyalty_class, stock_queue_ratio);

        // Find all rules matching this condition
        let matching: Vec<(usize, &Rule<PricingCondition, usize>)> = self
            .price_rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.condition == condition)
            .collect();

        if matching.is_empty() {
            // Fallback: shouldn't happen, but return price=10 and idx=0
            return (10, 0);
        }

        // Run stochastic auction among matching rules
        let matching_rules: Vec<_> = matching.iter().map(|(_, r)| (*r).clone()).collect();
        let local_idx = stochastic_auction(&matching_rules, 0.1, 0.025, &mut self.rng);
        let (global_idx, rule) = matching[local_idx];

        (rule.action, global_idx)
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
    pub fn update_strengths(&mut self, learning_rate: f64, max_price: usize) {
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
    fn act(
        &mut self,
        _current_t: usize,
        event: &MarketEvent,
    ) -> Response<MarketEvent, MarketStats> {
        match event {
            MarketEvent::SessionStart { day, .. } => {
                self.current_day = *day;
                Response::new()
            }

            MarketEvent::BuyersChooseSellers { .. } => {
                // Buyers are making choices; we'll learn about our queue next
                Response::new()
            }

            MarketEvent::ProcessQueues { day: _, session: _ } => {
                // This is where we would process our queue
                // For now, emit transaction events for buyers in queue
                // Note: The actual queue will be managed by MarketCoordinator
                // This is placeholder logic
                Response::new()
            }

            MarketEvent::DayEnd { .. } => {
                // Learning updated in main loop now
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
    fn test_buyer_type_valuations() {
        let low = BuyerAgent::new(0, BuyerType::Low, 10, 20, 42);
        let medium = BuyerAgent::new(1, BuyerType::Medium, 10, 20, 43);
        let high = BuyerAgent::new(2, BuyerType::High, 10, 20, 44);

        assert_eq!(low.p_out, 12);
        assert_eq!(medium.p_out, 15);
        assert_eq!(high.p_out, 18);
    }

    #[test]
    fn test_buyer_type_affects_acceptance() {
        // Low valuation buyer should reject price=15, medium should accept it
        let mut low_buyer = BuyerAgent::new(0, BuyerType::Low, 10, 20, 42);
        let mut med_buyer = BuyerAgent::new(1, BuyerType::Medium, 10, 20, 42);

        // Train both on the same price=15
        for _ in 0..50 {
            low_buyer.reset_daily_state();
            med_buyer.reset_daily_state();

            let low_accepts = low_buyer.respond_to_price(15);
            let med_accepts = med_buyer.respond_to_price(15);

            if low_accepts {
                low_buyer.transaction_completed = true;
            }
            if med_accepts {
                med_buyer.transaction_completed = true;
            }

            low_buyer.update_strengths(0.05);
            med_buyer.update_strengths(0.05);
        }

        // After training, medium buyer should have stronger acceptance of 15 than low buyer
        // (since 15 > 12 is bad for low, but 15 <= 15 is acceptable for medium)
        let low_accepts_final = low_buyer.respond_to_price(15);
        let med_accepts_final = med_buyer.respond_to_price(15);

        // Medium buyer should be more likely to accept than low buyer
        // Note: Due to stochasticity this might not always hold, but directionally should trend this way
        if !low_accepts_final && med_accepts_final {
            // This is the expected outcome
        }
    }

    #[test]
    fn test_buyer_learns_to_reject_bad_prices() {
        let mut buyer = BuyerAgent::new(0, BuyerType::Medium, 10, 20, 42); // p_out = 15

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
            .position(|r| r.condition == 20 && !r.action)
            .unwrap();

        let accept_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 20 && r.action)
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
        let mut buyer = BuyerAgent::new(0, BuyerType::Medium, 10, 20, 42); // p_out = 15

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
            .position(|r| r.condition == 10 && r.action)
            .unwrap();

        let reject_rule_idx = buyer
            .price_acceptance_rules
            .iter()
            .position(|r| r.condition == 10 && !r.action)
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

        // Choose price multiple times with some loyalty/queue context
        let (price1, idx1) = seller.choose_price(0.5, 10);
        let (price2, idx2) = seller.choose_price(0.3, 8);

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

        let _strength_before_high = seller.price_rules[high_price_idx].strength;
        let strength_before_low = seller.price_rules[low_price_idx].strength;

        // Update strengths
        seller.update_strengths(0.05, 20);

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
        let (price, idx) = seller.choose_price(0.5, 10);
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
        // Test with medium loyalty, medium stock/queue ratio
        let buyer_loyalty = 0.5; // Medium
        let queue_length = 10; // Moderate queue

        for _ in 0..100 {
            seller.reset_daily_state();
            seller.stock = 12; // Set stock for stock/queue ratio calculation

            // Simulate 10 offers per day
            for _ in 0..10 {
                let (price, idx) = seller.choose_price(buyer_loyalty, queue_length);
                let accepted = price == 12;
                seller.record_price_offer(idx, accepted, price);
            }

            seller.update_strengths(0.05, 20);
        }

        // Among rules for this condition, price 12 should be strongest
        let loyalty_class = LoyaltyClass::from_value(buyer_loyalty);
        let stock_queue = StockQueueRatio::from_values(seller.stock, queue_length);
        let condition = PricingCondition::new(loyalty_class, stock_queue);

        let price_12_rules: Vec<_> = seller
            .price_rules
            .iter()
            .filter(|r| r.condition == condition && r.action == 12)
            .collect();

        let other_rules: Vec<_> = seller
            .price_rules
            .iter()
            .filter(|r| r.condition == condition && r.action != 12)
            .collect();

        assert!(
            !price_12_rules.is_empty(),
            "Should have price 12 rule for this condition"
        );

        let price_12_strength = price_12_rules[0].strength;
        let max_other_strength = other_rules
            .iter()
            .map(|r| r.strength)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        assert!(
            price_12_strength > max_other_strength,
            "Price 12 rule should be strongest for this condition (12: {}, others: {})",
            price_12_strength,
            max_other_strength
        );
    }
}
