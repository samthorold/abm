use crate::{Event, OrderType, Role, Stats, TraderStats, TraderType, Unit};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Price range constants
const PRICE_MIN: usize = 1;
const PRICE_MAX: usize = 200;

/// Zero-Intelligence Unconstrained trader
/// Submits random bids/asks in range [1, 200] with no constraint on profitability
pub struct ZIUTrader {
    id: usize,
    role: Role,
    units: Vec<Unit>,
    current_unit_index: usize,
    rng: StdRng,
    stats: TraderStats,
}

impl ZIUTrader {
    pub fn new(id: usize, role: Role, units: Vec<Unit>, seed: u64) -> Self {
        let units_total = units.len();
        ZIUTrader {
            id,
            role,
            units,
            current_unit_index: 0,
            rng: StdRng::seed_from_u64(seed),
            stats: TraderStats::new(id, TraderType::ZIU, role, units_total),
        }
    }

    fn get_current_unit(&self) -> Option<&Unit> {
        self.units.get(self.current_unit_index)
    }

    fn generate_price(&mut self) -> usize {
        self.rng.random_range(PRICE_MIN..=PRICE_MAX)
    }
}

impl des::Agent<Event, Stats> for ZIUTrader {
    fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();
        stats.current_unit_index = self.current_unit_index;
        Stats::Trader(stats)
    }

    fn act(&mut self, current_t: usize, event: &Event) -> des::Response<Event, Stats> {
        match event {
            Event::OrderRequest {
                trader_id, period, ..
            } => {
                // Only respond if this request is for us
                if trader_id != &self.id {
                    return des::Response::new();
                }

                // Check if we have units remaining
                if let Some(unit) = self.get_current_unit() {
                    let value_or_cost = unit.value_or_cost;
                    let price = self.generate_price();
                    let order_type = match self.role {
                        Role::Buyer => OrderType::Bid,
                        Role::Seller => OrderType::Ask,
                    };

                    self.stats.orders_submitted += 1;

                    des::Response::event(
                        current_t,
                        Event::OrderSubmitted {
                            period: *period,
                            trader_id: self.id,
                            order_type,
                            price,
                            value_or_cost,
                        },
                    )
                } else {
                    // No units remaining - don't respond
                    des::Response::new()
                }
            }

            Event::Transaction {
                buyer_id,
                seller_id,
                price,
                buyer_value,
                seller_cost,
                ..
            } => {
                // Check if we're involved in this transaction
                let profit = if buyer_id == &self.id {
                    // We're the buyer
                    self.current_unit_index += 1;
                    self.stats.units_traded += 1;
                    (*buyer_value as i32) - (*price as i32)
                } else if seller_id == &self.id {
                    // We're the seller
                    self.current_unit_index += 1;
                    self.stats.units_traded += 1;
                    (*price as i32) - (*seller_cost as i32)
                } else {
                    // Not involved in this transaction
                    return des::Response::new();
                };

                self.stats.total_profit += profit;
                self.stats.unit_profits.push(profit);

                des::Response::new()
            }

            _ => des::Response::new(),
        }
    }
}

/// Zero-Intelligence Constrained trader
/// Submits random bids/asks constrained to avoid guaranteed losses
/// Buyers: bid in [1, value]
/// Sellers: ask in [cost, 200]
pub struct ZICTrader {
    id: usize,
    role: Role,
    units: Vec<Unit>,
    current_unit_index: usize,
    rng: StdRng,
    stats: TraderStats,
}

impl ZICTrader {
    pub fn new(id: usize, role: Role, units: Vec<Unit>, seed: u64) -> Self {
        let units_total = units.len();
        ZICTrader {
            id,
            role,
            units,
            current_unit_index: 0,
            rng: StdRng::seed_from_u64(seed),
            stats: TraderStats::new(id, TraderType::ZIC, role, units_total),
        }
    }

    fn get_current_unit(&self) -> Option<&Unit> {
        self.units.get(self.current_unit_index)
    }

    fn generate_price(&mut self, unit: &Unit) -> usize {
        match self.role {
            Role::Buyer => {
                // Bid in range [PRICE_MIN, value]
                let max_price = unit.value_or_cost;
                self.rng.random_range(PRICE_MIN..=max_price)
            }
            Role::Seller => {
                // Ask in range [cost, PRICE_MAX]
                let min_price = unit.value_or_cost;
                self.rng.random_range(min_price..=PRICE_MAX)
            }
        }
    }
}

impl des::Agent<Event, Stats> for ZICTrader {
    fn stats(&self) -> Stats {
        let mut stats = self.stats.clone();
        stats.current_unit_index = self.current_unit_index;
        Stats::Trader(stats)
    }

    fn act(&mut self, current_t: usize, event: &Event) -> des::Response<Event, Stats> {
        match event {
            Event::OrderRequest {
                trader_id, period, ..
            } => {
                // Only respond if this request is for us
                if trader_id != &self.id {
                    return des::Response::new();
                }

                // Check if we have units remaining
                if let Some(unit) = self.get_current_unit() {
                    // Copy the unit value to avoid borrow issues
                    let value_or_cost = unit.value_or_cost;
                    let price = self.generate_price(&Unit { value_or_cost });
                    let order_type = match self.role {
                        Role::Buyer => OrderType::Bid,
                        Role::Seller => OrderType::Ask,
                    };

                    self.stats.orders_submitted += 1;

                    des::Response::event(
                        current_t,
                        Event::OrderSubmitted {
                            period: *period,
                            trader_id: self.id,
                            order_type,
                            price,
                            value_or_cost,
                        },
                    )
                } else {
                    // No units remaining - don't respond
                    des::Response::new()
                }
            }

            Event::Transaction {
                buyer_id,
                seller_id,
                price,
                buyer_value,
                seller_cost,
                ..
            } => {
                // Check if we're involved in this transaction
                let profit = if buyer_id == &self.id {
                    // We're the buyer
                    self.current_unit_index += 1;
                    self.stats.units_traded += 1;
                    (*buyer_value as i32) - (*price as i32)
                } else if seller_id == &self.id {
                    // We're the seller
                    self.current_unit_index += 1;
                    self.stats.units_traded += 1;
                    (*price as i32) - (*seller_cost as i32)
                } else {
                    // Not involved in this transaction
                    return des::Response::new();
                };

                self.stats.total_profit += profit;
                self.stats.unit_profits.push(profit);

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
    fn test_zi_u_generates_any_price() {
        let units = vec![Unit { value_or_cost: 100 }];
        let mut trader = ZIUTrader::new(0, Role::Buyer, units, 42);

        // Generate many prices and verify they span the range
        let mut prices = Vec::new();
        for _ in 0..1000 {
            let price = trader.generate_price();
            prices.push(price);
        }

        // Should have prices in lower, middle, and upper ranges
        assert!(prices.iter().any(|&p| p < 70));
        assert!(prices.iter().any(|&p| p > 70 && p < 130));
        assert!(prices.iter().any(|&p| p > 130));
    }

    #[test]
    fn test_zi_c_buyer_respects_value_constraint() {
        let value = 100;
        let units = vec![Unit {
            value_or_cost: value,
        }];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Generate many prices - all should be <= value
        for _ in 0..1000 {
            let unit_value = trader.get_current_unit().unwrap().value_or_cost;
            let price = trader.generate_price(&Unit {
                value_or_cost: unit_value,
            });
            assert!(
                price >= PRICE_MIN && price <= value,
                "Buyer bid {} should be in range [1, {}]",
                price,
                value
            );
        }
    }

    #[test]
    fn test_zi_c_seller_respects_cost_constraint() {
        let cost = 100;
        let units = vec![Unit {
            value_or_cost: cost,
        }];
        let mut trader = ZICTrader::new(0, Role::Seller, units, 42);

        // Generate many prices - all should be >= cost
        for _ in 0..1000 {
            let unit_cost = trader.get_current_unit().unwrap().value_or_cost;
            let price = trader.generate_price(&Unit {
                value_or_cost: unit_cost,
            });
            assert!(
                price >= cost && price <= PRICE_MAX,
                "Seller ask {} should be in range [{}, 200]",
                price,
                cost
            );
        }
    }

    #[test]
    fn test_zi_c_edge_case_cost_equals_max() {
        // Seller with cost = 200 can only offer exactly 200
        let units = vec![Unit { value_or_cost: 200 }];
        let mut trader = ZICTrader::new(0, Role::Seller, units, 42);

        for _ in 0..100 {
            let unit_cost = trader.get_current_unit().unwrap().value_or_cost;
            let price = trader.generate_price(&Unit {
                value_or_cost: unit_cost,
            });
            assert_eq!(price, 200, "Seller with cost=200 should only ask 200");
        }
    }

    #[test]
    fn test_trader_stats_updated_on_transaction() {
        use des::Agent;

        let units = vec![Unit { value_or_cost: 100 }];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Simulate transaction
        trader.act(
            0,
            &Event::Transaction {
                period: 1,
                buyer_id: 0,
                seller_id: 1,
                price: 80,
                buyer_value: 100,
                seller_cost: 70,
                sequence: 1,
            },
        );

        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert_eq!(s.units_traded, 1);
            assert_eq!(s.total_profit, 20); // 100 - 80
            assert_eq!(s.unit_profits.len(), 1);
            assert_eq!(s.unit_profits[0], 20);
        } else {
            panic!("Expected TraderStats");
        }
    }

    #[test]
    fn test_trader_with_multiple_units_advances_correctly() {
        use des::Agent;

        let units = vec![
            Unit { value_or_cost: 100 },
            Unit { value_or_cost: 80 },
            Unit { value_or_cost: 60 },
        ];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Verify initial state
        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert_eq!(s.current_unit_index, 0);
            assert_eq!(s.units_total, 3);
            assert_eq!(s.units_remaining(), 3);
        }

        // First transaction
        trader.act(
            0,
            &Event::Transaction {
                period: 1,
                buyer_id: 0,
                seller_id: 1,
                price: 90,
                buyer_value: 100,
                seller_cost: 80,
                sequence: 0,
            },
        );

        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert_eq!(s.current_unit_index, 1, "Should advance to unit 1");
            assert_eq!(s.units_traded, 1);
            assert_eq!(s.units_remaining(), 2);
        }

        // Second transaction
        trader.act(
            1,
            &Event::Transaction {
                period: 1,
                buyer_id: 0,
                seller_id: 1,
                price: 70,
                buyer_value: 80,
                seller_cost: 60,
                sequence: 1,
            },
        );

        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert_eq!(s.current_unit_index, 2, "Should advance to unit 2");
            assert_eq!(s.units_traded, 2);
            assert_eq!(s.units_remaining(), 1);
        }
    }

    #[test]
    fn test_trader_with_no_units_remaining_doesnt_respond() {
        use des::Agent;

        let units = vec![Unit { value_or_cost: 100 }];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Trade the only unit
        trader.act(
            0,
            &Event::Transaction {
                period: 1,
                buyer_id: 0,
                seller_id: 1,
                price: 90,
                buyer_value: 100,
                seller_cost: 80,
                sequence: 0,
            },
        );

        // Verify no units remaining
        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert!(!s.has_units_remaining());
            assert_eq!(s.units_remaining(), 0);
        }

        // Try to get an order - should not respond
        let response = trader.act(
            1,
            &Event::OrderRequest {
                trader_id: 0,
                period: 1,
                iteration: 10,
            },
        );

        // No events should be generated
        assert_eq!(response.events.len(), 0);
    }

    #[test]
    fn test_zi_c_buyer_with_value_near_price_min() {
        let units = vec![Unit { value_or_cost: 2 }];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Can still bid despite narrow range [1, 2]
        for _ in 0..100 {
            let unit_value = trader.get_current_unit().unwrap().value_or_cost;
            let price = trader.generate_price(&Unit {
                value_or_cost: unit_value,
            });
            assert!(
                (PRICE_MIN..=2).contains(&price),
                "Buyer with value=2 should bid in [1, 2], got {}",
                price
            );
        }
    }

    #[test]
    fn test_zi_c_seller_with_cost_near_price_max() {
        let units = vec![Unit { value_or_cost: 199 }];
        let mut trader = ZICTrader::new(0, Role::Seller, units, 42);

        // Can still ask despite narrow range [199, 200]
        for _ in 0..100 {
            let unit_cost = trader.get_current_unit().unwrap().value_or_cost;
            let price = trader.generate_price(&Unit {
                value_or_cost: unit_cost,
            });
            assert!(
                (199..=PRICE_MAX).contains(&price),
                "Seller with cost=199 should ask in [199, 200], got {}",
                price
            );
        }
    }

    #[test]
    fn test_trader_ignores_other_traders_transactions() {
        use des::Agent;

        let units = vec![Unit { value_or_cost: 100 }];
        let mut trader = ZICTrader::new(0, Role::Buyer, units, 42);

        // Transaction involving other traders
        trader.act(
            0,
            &Event::Transaction {
                period: 1,
                buyer_id: 5,
                seller_id: 10,
                price: 90,
                buyer_value: 120,
                seller_cost: 80,
                sequence: 0,
            },
        );

        // Trader state should be unchanged
        let stats = trader.stats();
        if let Stats::Trader(s) = stats {
            assert_eq!(s.units_traded, 0);
            assert_eq!(s.total_profit, 0);
            assert_eq!(s.current_unit_index, 0);
        }
    }
}
