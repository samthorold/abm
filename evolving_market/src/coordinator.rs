use crate::*;
use crate::agents::{BuyerAgent, SellerAgent};
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
}

impl MarketCoordinator {
    pub fn new(n_buyers: usize, n_sellers: usize, alpha: f64) -> Self {
        // Initialize loyalty matrix with zeros
        let loyalty = vec![vec![0.0; n_sellers]; n_buyers];

        MarketCoordinator {
            n_buyers,
            n_sellers,
            loyalty,
            alpha,
            queues: HashMap::new(),
            buyer_choices: HashMap::new(),
        }
    }

    /// Record a buyer's choice of seller
    pub fn record_buyer_choice(&mut self, buyer_id: usize, seller_id: usize) {
        self.buyer_choices.insert(buyer_id, seller_id);
    }

    /// Form queues based on buyer choices
    /// For minimal model: simple FIFO queues
    pub fn form_queues(&mut self) {
        self.queues.clear();

        for (buyer_id, seller_id) in &self.buyer_choices {
            self.queues
                .entry(*seller_id)
                .or_insert_with(Vec::new)
                .push(*buyer_id);
        }
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
            let queue = self.queues.get(&seller_id).cloned().unwrap_or_default();

            // Process each buyer in queue
            for buyer_id in queue {
                // Check if seller has stock
                if seller.stock == 0 {
                    // Denied service
                    events.push(MarketEvent::Transaction {
                        day,
                        session,
                        buyer_id,
                        seller_id,
                        price: None,
                        accepted: false,
                    });
                    continue;
                }

                // Seller chooses price
                let price = seller.choose_price();

                // Buyer responds
                let buyer = &mut buyers[buyer_id];
                let accepted = buyer.respond_to_price(price);

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
