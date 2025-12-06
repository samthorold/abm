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
                    // Normalize to [0, 1]
                    (surplus as f64 / self.p_out as f64).max(0.0)
                } else {
                    0.0
                }
            } else {
                // Rejected price or denied service
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

    // Classifier systems (minimal: just pricing)
    price_rules: Vec<Rule<(), usize>>, // Unconditional pricing rules, action = price

    // Stock and revenue tracking (pub for coordinator access)
    pub stock: usize,
    pub initial_stock: usize,
    pub gross_revenue: usize,

    // Daily state
    current_day: usize,
    queue: Vec<usize>, // Buyer IDs in queue
    active_price_rules: Vec<(usize, usize)>, // (rule_idx, price_offered, times_used)

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

        SellerAgent {
            id,
            p_in,
            price_rules,
            stock: initial_stock,
            initial_stock,
            gross_revenue: 0,
            current_day: 0,
            queue: Vec::new(),
            active_price_rules: Vec::new(),
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    /// Choose a price to offer (pub for coordinator access)
    pub fn choose_price(&mut self) -> usize {
        let idx = stochastic_auction(&self.price_rules, 0.1, 0.025, &mut self.rng);
        self.price_rules[idx].action
    }

    /// Update pricing rule strengths based on revenue (pub for main loop)
    pub fn update_strengths(&mut self, learning_rate: f64) {
        let max_price = self.price_rules.len() - 1;

        // Update each pricing rule based on how it performed
        for rule in &mut self.price_rules {
            // Simple reward: if this price was offered and accepted, reward based on price
            // For now, we'll update based on average revenue per offer
            // This is simplified; full model tracks per-rule usage
            let reward = self.gross_revenue as f64 / (self.initial_stock as f64 * max_price as f64);
            rule.strength = update_strength(rule.strength, reward, learning_rate);
        }
    }

    /// Reset for new day (pub for main loop)
    pub fn reset_daily_state(&mut self) {
        self.stock = self.initial_stock;
        self.gross_revenue = 0;
        self.queue.clear();
        self.active_price_rules.clear();
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

