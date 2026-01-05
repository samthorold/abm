use crate::market_configs::MarketConfig;
use crate::{CoordinatorStats, Event, Order, OrderType, Role, Stats, Transaction, Unit};
use rand::prelude::*;
use rand::rngs::StdRng;

/// Order book for continuous double auction
/// Only maintains best bid and best ask
/// Previous unaccepted orders are implicitly canceled when new orders arrive
#[derive(Debug)]
pub struct OrderBook {
    pub best_bid: Option<Order>,
    pub best_ask: Option<Order>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            best_bid: None,
            best_ask: None,
        }
    }

    /// Process an incoming order
    /// Returns Some(Transaction) if order crosses the spread, None otherwise
    pub fn process_order(&mut self, order: Order) -> Option<Transaction> {
        match order.order_type {
            OrderType::Bid => {
                // Check if bid crosses best ask
                if let Some(ask) = self.best_ask {
                    if order.price >= ask.price {
                        // Transaction occurs at ask price (earlier order)
                        let buyer_value = order.value_or_cost;
                        let seller_cost = ask.value_or_cost;

                        self.best_ask = None; // Clear matched ask

                        return Some(Transaction {
                            sequence: 0, // Will be set by coordinator
                            buyer_id: order.trader_id,
                            seller_id: ask.trader_id,
                            price: ask.price,
                            buyer_value,
                            seller_cost,
                        });
                    }
                }
                // No cross - update best bid
                self.best_bid = Some(order);
                None
            }

            OrderType::Ask => {
                // Check if ask crosses best bid
                if let Some(bid) = self.best_bid {
                    if order.price <= bid.price {
                        // Transaction occurs at bid price (earlier order)
                        let buyer_value = bid.value_or_cost;
                        let seller_cost = order.value_or_cost;

                        self.best_bid = None; // Clear matched bid

                        return Some(Transaction {
                            sequence: 0, // Will be set by coordinator
                            buyer_id: bid.trader_id,
                            seller_id: order.trader_id,
                            price: bid.price,
                            buyer_value,
                            seller_cost,
                        });
                    }
                }
                // No cross - update best ask
                self.best_ask = Some(order);
                None
            }
        }
    }

    pub fn reset(&mut self) {
        self.best_bid = None;
        self.best_ask = None;
    }
}

/// Coordinator agent that orchestrates the trading period
/// Maintains order book, selects traders, processes transactions
pub struct Coordinator {
    market_config: MarketConfig,
    order_book: OrderBook,

    // Trader management
    all_trader_ids: Vec<usize>,
    active_trader_ids: Vec<usize>,

    // Period state
    current_period: usize,
    current_iteration: usize,
    max_iterations: usize,
    transaction_sequence: usize,

    // RNG for trader selection
    rng: StdRng,

    // Stats
    stats: CoordinatorStats,
}

impl Coordinator {
    pub fn new(market_config: MarketConfig, max_iterations: usize, seed: u64) -> Self {
        let num_buyers = market_config.num_buyers();
        let num_sellers = market_config.num_sellers();

        // Assign trader IDs: buyers 0..num_buyers, sellers num_buyers..num_buyers+num_sellers
        let all_trader_ids: Vec<usize> = (0..num_buyers + num_sellers).collect();

        let max_surplus = market_config.calculate_max_surplus();

        Coordinator {
            order_book: OrderBook::new(),
            all_trader_ids: all_trader_ids.clone(),
            active_trader_ids: all_trader_ids,
            current_period: 0,
            current_iteration: 0,
            max_iterations,
            transaction_sequence: 0,
            rng: StdRng::seed_from_u64(seed),
            stats: CoordinatorStats::new(
                0,
                market_config.id,
                market_config.equilibrium_price,
                market_config.equilibrium_quantity,
                max_surplus,
            ),
            market_config,
        }
    }

    /// Initialize trader units for a new period
    pub fn get_trader_units(&self, trader_id: usize, role: Role) -> Vec<Unit> {
        match role {
            Role::Buyer => {
                let buyer_index = trader_id;
                self.market_config.buyer_values[buyer_index]
                    .iter()
                    .map(|&value| Unit {
                        value_or_cost: value,
                    })
                    .collect()
            }
            Role::Seller => {
                let seller_index = trader_id - self.market_config.num_buyers();
                self.market_config.seller_costs[seller_index]
                    .iter()
                    .map(|&cost| Unit {
                        value_or_cost: cost,
                    })
                    .collect()
            }
        }
    }

    /// Get trader role from ID
    pub fn get_trader_role(&self, trader_id: usize) -> Role {
        if trader_id < self.market_config.num_buyers() {
            Role::Buyer
        } else {
            Role::Seller
        }
    }

    /// Select a random trader from active traders
    fn select_random_trader(&mut self) -> Option<usize> {
        if self.active_trader_ids.is_empty() {
            return None;
        }
        self.active_trader_ids.choose(&mut self.rng).copied()
    }

    /// Remove trader from active list (when they have no more units)
    fn deactivate_trader(&mut self, trader_id: usize) {
        self.active_trader_ids.retain(|&id| id != trader_id);
    }

    /// Reset for new period
    fn start_new_period(&mut self, period: usize) {
        self.current_period = period;
        self.current_iteration = 0;
        self.transaction_sequence = 0;
        self.order_book.reset();
        self.active_trader_ids = self.all_trader_ids.clone();

        // Reset stats
        self.stats = CoordinatorStats::new(
            period,
            self.market_config.id,
            self.market_config.equilibrium_price,
            self.market_config.equilibrium_quantity,
            self.market_config.calculate_max_surplus(),
        );
    }
}

impl des::Agent<Event, Stats> for Coordinator {
    fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();
        stats.best_bid = self.order_book.best_bid;
        stats.best_ask = self.order_book.best_ask;
        stats.orders_processed = self.current_iteration;
        Stats::Coordinator(stats)
    }

    fn act(&mut self, current_t: usize, event: &Event) -> des::Response<Event, Stats> {
        match event {
            Event::PeriodStart { period, .. } => {
                self.start_new_period(*period);

                // Request first order
                if let Some(trader_id) = self.select_random_trader() {
                    des::Response::event(
                        current_t,
                        Event::OrderRequest {
                            period: *period,
                            trader_id,
                            iteration: 0,
                        },
                    )
                } else {
                    // No active traders - end period immediately
                    des::Response::event(current_t, Event::PeriodEnd { period: *period })
                }
            }

            Event::OrderSubmitted {
                period,
                trader_id,
                order_type,
                price,
                value_or_cost,
            } => {
                self.current_iteration += 1;

                let order = Order {
                    trader_id: *trader_id,
                    order_type: *order_type,
                    price: *price,
                    value_or_cost: *value_or_cost,
                };

                let mut events = Vec::new();

                // Try to process order
                if let Some(mut txn) = self.order_book.process_order(order) {
                    // Transaction occurred
                    txn.sequence = self.transaction_sequence;
                    self.transaction_sequence += 1;

                    // Update stats
                    self.stats.total_surplus += txn.total_surplus();
                    self.stats.transactions.push(txn);

                    // Check if traders have more units
                    let buyer_role = self.get_trader_role(txn.buyer_id);
                    let buyer_units = self.get_trader_units(txn.buyer_id, buyer_role);
                    let buyer_units_traded = self
                        .stats
                        .transactions
                        .iter()
                        .filter(|t| t.buyer_id == txn.buyer_id)
                        .count();
                    if buyer_units_traded >= buyer_units.len() {
                        self.deactivate_trader(txn.buyer_id);
                    }

                    let seller_role = self.get_trader_role(txn.seller_id);
                    let seller_units = self.get_trader_units(txn.seller_id, seller_role);
                    let seller_units_traded = self
                        .stats
                        .transactions
                        .iter()
                        .filter(|t| t.seller_id == txn.seller_id)
                        .count();
                    if seller_units_traded >= seller_units.len() {
                        self.deactivate_trader(txn.seller_id);
                    }

                    // Broadcast transaction
                    events.push((
                        current_t,
                        Event::Transaction {
                            period: *period,
                            buyer_id: txn.buyer_id,
                            seller_id: txn.seller_id,
                            price: txn.price,
                            buyer_value: txn.buyer_value,
                            seller_cost: txn.seller_cost,
                            sequence: txn.sequence,
                        },
                    ));
                }

                // Check if we should continue
                if self.current_iteration >= self.max_iterations
                    || self.active_trader_ids.is_empty()
                {
                    // End period
                    events.push((current_t + 1, Event::PeriodEnd { period: *period }));
                } else {
                    // Request next order
                    if let Some(next_trader) = self.select_random_trader() {
                        events.push((
                            current_t + 1,
                            Event::OrderRequest {
                                period: *period,
                                trader_id: next_trader,
                                iteration: self.current_iteration,
                            },
                        ));
                    } else {
                        // No active traders left
                        events.push((current_t + 1, Event::PeriodEnd { period: *period }));
                    }
                }

                des::Response::events(events)
            }

            Event::OrderRequest { .. } => {
                // If trader doesn't respond (no units), we'll handle it when we don't get OrderSubmitted
                // For now, just pass through - the trader will respond or not
                // This is handled by checking active_trader_ids before selecting next trader

                // Actually, we need to handle the case where a trader was selected but has no units
                // The trader won't respond, so we won't get an OrderSubmitted
                // We'll handle this by timeout or by tracking responses

                // For now, assume traders always respond if selected and have units
                // The deactivation happens after a transaction when we detect no more units

                des::Response::new()
            }

            _ => des::Response::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_bid_ask_crossing() {
        let mut book = OrderBook::new();

        // Submit ask at 60
        let ask = Order {
            trader_id: 1,
            order_type: OrderType::Ask,
            price: 60,
            value_or_cost: 50, // Seller cost
        };
        let result = book.process_order(ask);
        assert!(result.is_none()); // No transaction yet

        // Submit bid at 70 (crosses)
        let bid = Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 70,
            value_or_cost: 100, // Buyer value
        };
        let result = book.process_order(bid);
        assert!(result.is_some());

        let txn = result.unwrap();
        assert_eq!(txn.price, 60); // Should execute at ask price (earlier order)
        assert_eq!(txn.buyer_id, 0);
        assert_eq!(txn.seller_id, 1);
        assert_eq!(txn.buyer_value, 100);
        assert_eq!(txn.seller_cost, 50);
    }

    #[test]
    fn test_order_book_ask_bid_crossing() {
        let mut book = OrderBook::new();

        // Submit bid at 70
        let bid = Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 70,
            value_or_cost: 100, // Buyer value
        };
        let result = book.process_order(bid);
        assert!(result.is_none());

        // Submit ask at 65 (crosses)
        let ask = Order {
            trader_id: 1,
            order_type: OrderType::Ask,
            price: 65,
            value_or_cost: 50, // Seller cost
        };
        let result = book.process_order(ask);
        assert!(result.is_some());

        let txn = result.unwrap();
        assert_eq!(txn.price, 70); // Should execute at bid price (earlier order)
        assert_eq!(txn.buyer_value, 100);
        assert_eq!(txn.seller_cost, 50);
    }

    #[test]
    fn test_order_book_no_crossing() {
        let mut book = OrderBook::new();

        // Submit bid at 50
        let bid = Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 50,
            value_or_cost: 100,
        };
        book.process_order(bid);

        // Submit ask at 60 (doesn't cross)
        let ask = Order {
            trader_id: 1,
            order_type: OrderType::Ask,
            price: 60,
            value_or_cost: 50,
        };
        let result = book.process_order(ask);
        assert!(result.is_none());

        assert_eq!(book.best_bid.unwrap().price, 50);
        assert_eq!(book.best_ask.unwrap().price, 60);
    }

    #[test]
    fn test_transaction_surplus_calculation() {
        let txn = Transaction {
            sequence: 0,
            buyer_id: 0,
            seller_id: 1,
            price: 80,
            buyer_value: 100,
            seller_cost: 70,
        };

        assert_eq!(txn.total_surplus(), 30); // (100-80) + (80-70) = 20 + 10 = 30
    }

    #[test]
    fn test_order_book_with_marginal_orders() {
        let mut book = OrderBook::new();

        // Orders clustered near same price (marginal units)
        let bid = Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 131,
            value_or_cost: 132,
        };
        book.process_order(bid);

        let ask = Order {
            trader_id: 1,
            order_type: OrderType::Ask,
            price: 131,
            value_or_cost: 130,
        };
        let result = book.process_order(ask);

        assert!(result.is_some());
        let txn = result.unwrap();
        assert_eq!(txn.price, 131); // Should match at earlier order price
        assert_eq!(txn.total_surplus(), 2); // (132-131) + (131-130) = 1 + 1 = 2
    }

    #[test]
    fn test_order_book_cancels_previous_order() {
        let mut book = OrderBook::new();

        // Submit first bid
        let bid1 = Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 100,
            value_or_cost: 120,
        };
        book.process_order(bid1);
        assert_eq!(book.best_bid.unwrap().price, 100);

        // Submit another bid (implicitly cancels first)
        let bid2 = Order {
            trader_id: 2,
            order_type: OrderType::Bid,
            price: 90,
            value_or_cost: 110,
        };
        book.process_order(bid2);

        // Best bid should be replaced
        assert_eq!(book.best_bid.unwrap().price, 90);
        assert_eq!(book.best_bid.unwrap().trader_id, 2);
    }

    #[test]
    fn test_order_book_empty_state() {
        let book = OrderBook::new();
        assert!(book.best_bid.is_none());
        assert!(book.best_ask.is_none());
    }

    #[test]
    fn test_order_book_reset() {
        let mut book = OrderBook::new();

        // Add orders
        book.process_order(Order {
            trader_id: 0,
            order_type: OrderType::Bid,
            price: 50,
            value_or_cost: 100,
        });
        book.process_order(Order {
            trader_id: 1,
            order_type: OrderType::Ask,
            price: 60,
            value_or_cost: 40,
        });

        assert!(book.best_bid.is_some());
        assert!(book.best_ask.is_some());

        // Reset should clear
        book.reset();
        assert!(book.best_bid.is_none());
        assert!(book.best_ask.is_none());
    }
}
