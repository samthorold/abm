use crate::agents::{BuyerAgent, SellerAgent};
use crate::*;
use rand::Rng;
use std::collections::HashMap;

/// Market Coordinator manages shared state and orchestrates transactions
/// This is NOT an agent - it's a helper struct for managing market mechanics
pub struct MarketCoordinator {
    pub n_buyers: usize,
    pub n_sellers: usize,

    /// Loyalty matrix L_ij: loyalty[buyer_id][seller_id]
    pub loyalty: Vec<Vec<f64>>,

    /// Alpha parameter for loyalty decay
    pub alpha: f64,

    /// Queues for current session: seller_id -> list of buyer_ids
    queues: HashMap<usize, Vec<usize>>,

    /// Buyer choices for current session
    buyer_choices: HashMap<usize, usize>, // buyer_id -> seller_id

    /// RNG for loyalty-weighted selection
    rng: rand::rngs::StdRng,
}

impl MarketCoordinator {
    pub fn new(n_buyers: usize, n_sellers: usize, alpha: f64) -> Self {
        use rand::SeedableRng;

        // Initialize loyalty matrix with zeros
        let loyalty = vec![vec![0.0; n_sellers]; n_buyers];

        MarketCoordinator {
            n_buyers,
            n_sellers,
            loyalty,
            alpha,
            queues: HashMap::new(),
            buyer_choices: HashMap::new(),
            rng: rand::rngs::StdRng::seed_from_u64(123), // Fixed seed for reproducibility
        }
    }

    /// Record a buyer's choice of seller
    pub fn record_buyer_choice(&mut self, buyer_id: usize, seller_id: usize) {
        self.buyer_choices.insert(buyer_id, seller_id);
    }

    /// Form queues based on buyer choices
    pub fn form_queues(&mut self) {
        self.queues.clear();

        for (buyer_id, seller_id) in &self.buyer_choices {
            self.queues.entry(*seller_id).or_default().push(*buyer_id);
        }
    }

    /// Select next customer from queue using loyalty-weighted probabilities
    /// Formula: P(buyer) ∝ (1 + L_ij)^β
    fn select_next_customer(
        &mut self,
        queue: &[usize],
        seller_id: usize,
        beta: i32,
    ) -> Option<usize> {
        if queue.is_empty() {
            return None;
        }

        if queue.len() == 1 {
            return Some(queue[0]);
        }

        // Compute weights for each buyer in queue
        let weights: Vec<f64> = queue
            .iter()
            .map(|&buyer_id| {
                let loyalty = self.loyalty[buyer_id][seller_id];
                let base = 1.0 + loyalty;
                if beta == 0 {
                    1.0 // Equal weighting when β = 0
                } else {
                    base.powi(beta)
                }
            })
            .collect();

        let total_weight: f64 = weights.iter().sum();

        if total_weight == 0.0 {
            // Fallback to random selection
            let idx = self.rng.random::<f64>() * queue.len() as f64;
            return Some(queue[idx as usize]);
        }

        // Random selection with loyalty-weighted probabilities
        let mut r = self.rng.random::<f64>() * total_weight;
        for (idx, &weight) in weights.iter().enumerate() {
            r -= weight;
            if r <= 0.0 {
                return Some(queue[idx]);
            }
        }

        // Fallback (shouldn't happen)
        Some(queue[queue.len() - 1])
    }

    /// Process queues: sellers make offers, buyers respond
    /// Returns list of transaction events
    pub fn process_queues(
        &mut self,
        sellers: &mut [SellerAgent],
        buyers: &mut [BuyerAgent],
        day: usize,
        session: Session,
    ) -> Vec<MarketEvent> {
        let mut events = Vec::new();

        for seller in sellers {
            let seller_id = seller.id;
            let mut remaining_queue = self.queues.get(&seller_id).cloned().unwrap_or_default();

            // Process queue using loyalty-weighted selection
            while !remaining_queue.is_empty() && seller.stock > 0 {
                // Select next buyer based on loyalty and beta
                let buyer_id =
                    match self.select_next_customer(&remaining_queue, seller_id, seller.beta) {
                        Some(id) => id,
                        None => break,
                    };

                // Remove selected buyer from queue
                remaining_queue.retain(|&id| id != buyer_id);

                // Get buyer's loyalty to this seller
                let buyer_loyalty = self.loyalty[buyer_id][seller_id];
                let queue_length = remaining_queue.len() + 1; // +1 for current buyer

                // Seller chooses price based on buyer loyalty and queue state
                let (price, rule_idx) = seller.choose_price(buyer_loyalty, queue_length);

                // Buyer responds
                let buyer = &mut buyers[buyer_id];
                let accepted = buyer.respond_to_price(price);

                // Record the price offer for learning
                seller.record_price_offer(rule_idx, accepted, price);

                if accepted {
                    // Transaction successful
                    seller.stock -= 1;
                    seller.gross_revenue += price;
                    buyer.transaction_completed = true;

                    events.push(MarketEvent::Transaction {
                        day,
                        session,
                        buyer_id,
                        seller_id,
                        price: Some(price),
                        accepted: true,
                    });
                } else {
                    // Price rejected
                    events.push(MarketEvent::Transaction {
                        day,
                        session,
                        buyer_id,
                        seller_id,
                        price: Some(price),
                        accepted: false,
                    });
                }
            }

            // Handle buyers left in queue (denied service due to stock out)
            for buyer_id in remaining_queue {
                events.push(MarketEvent::Transaction {
                    day,
                    session,
                    buyer_id,
                    seller_id,
                    price: None,
                    accepted: false,
                });
            }
        }

        events
    }

    /// Update loyalty matrix at end of day
    pub fn update_loyalty(&mut self) {
        for buyer_id in 0..self.n_buyers {
            for seller_id in 0..self.n_sellers {
                let visited = self.buyer_choices.get(&buyer_id) == Some(&seller_id);
                self.loyalty[buyer_id][seller_id] =
                    update_loyalty(self.loyalty[buyer_id][seller_id], visited, self.alpha);
            }
        }
    }

    /// Get loyalty value for a buyer-seller pair
    pub fn get_loyalty(&self, buyer_id: usize, seller_id: usize) -> f64 {
        self.loyalty[buyer_id][seller_id]
    }

    /// Compute average loyalty concentration across all buyers
    pub fn average_loyalty_concentration(&self) -> f64 {
        let concentrations: Vec<f64> = (0..self.n_buyers)
            .map(|buyer_id| loyalty_concentration(&self.loyalty[buyer_id]))
            .collect();

        concentrations.iter().sum::<f64>() / concentrations.len() as f64
    }

    /// Reset for new session
    pub fn reset_session(&mut self) {
        self.buyer_choices.clear();
        self.queues.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loyalty_weighted_selection_favors_loyal_with_positive_beta() {
        let mut coordinator = MarketCoordinator::new(3, 1, 0.25);

        // Set up loyalty values
        coordinator.loyalty[0][0] = 0.9; // Highly loyal
        coordinator.loyalty[1][0] = 0.5; // Medium loyal
        coordinator.loyalty[2][0] = 0.1; // Not loyal

        let queue = vec![0, 1, 2];
        let beta = 10; // Positive beta favors loyal customers

        // Run selection many times and count
        let mut selections = [0, 0, 0];
        for _ in 0..1000 {
            if let Some(selected) = coordinator.select_next_customer(&queue, 0, beta) {
                let idx = queue.iter().position(|&b| b == selected).unwrap();
                selections[idx] += 1;
            }
        }

        // Highly loyal buyer should be selected much more often
        assert!(
            selections[0] > selections[1],
            "Highly loyal buyer should be selected more than medium loyal"
        );
        assert!(
            selections[1] > selections[2],
            "Medium loyal should be selected more than non-loyal"
        );
        assert!(
            selections[0] > 600,
            "Highly loyal buyer should be selected >60% of the time with β=10"
        );
    }

    #[test]
    fn test_loyalty_weighted_selection_neutral_with_zero_beta() {
        let mut coordinator = MarketCoordinator::new(3, 1, 0.25);

        // Set up different loyalty values
        coordinator.loyalty[0][0] = 0.9;
        coordinator.loyalty[1][0] = 0.5;
        coordinator.loyalty[2][0] = 0.1;

        let queue = vec![0, 1, 2];
        let beta = 0; // Neutral - all equal

        // Run selection many times
        let mut selections = [0, 0, 0];
        for _ in 0..900 {
            if let Some(selected) = coordinator.select_next_customer(&queue, 0, beta) {
                let idx = queue.iter().position(|&b| b == selected).unwrap();
                selections[idx] += 1;
            }
        }

        // All should be roughly equal (within 20% of expected 300)
        for count in &selections {
            assert!(
                *count > 240 && *count < 360,
                "With β=0, selections should be roughly equal: got {:?}",
                selections
            );
        }
    }

    #[test]
    fn test_loyalty_weighted_selection_disfavors_loyal_with_negative_beta() {
        let mut coordinator = MarketCoordinator::new(3, 1, 0.25);

        // Set up loyalty values
        coordinator.loyalty[0][0] = 0.9; // Highly loyal
        coordinator.loyalty[1][0] = 0.5; // Medium loyal
        coordinator.loyalty[2][0] = 0.1; // Not loyal

        let queue = vec![0, 1, 2];
        let beta = -10; // Negative beta favors NEW customers

        // Run selection many times
        let mut selections = [0, 0, 0];
        for _ in 0..1000 {
            if let Some(selected) = coordinator.select_next_customer(&queue, 0, beta) {
                let idx = queue.iter().position(|&b| b == selected).unwrap();
                selections[idx] += 1;
            }
        }

        // Non-loyal buyer should be selected much more often
        assert!(
            selections[2] > selections[1],
            "Non-loyal buyer should be selected more than medium loyal"
        );
        assert!(
            selections[1] > selections[0],
            "Medium loyal should be selected more than highly loyal"
        );
        assert!(
            selections[2] > 600,
            "Non-loyal buyer should be selected >60% of the time with β=-10"
        );
    }
}
