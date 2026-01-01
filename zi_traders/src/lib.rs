use std::fmt;

pub mod analysis;
pub mod coordinator;
pub mod market_configs;
pub mod traders;

/// Role of a trader in the market
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Buyer,
    Seller,
}

/// Type of trader agent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderType {
    ZIU, // Zero-Intelligence Unconstrained
    ZIC, // Zero-Intelligence Constrained
}

impl fmt::Display for TraderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraderType::ZIU => write!(f, "ZI-U"),
            TraderType::ZIC => write!(f, "ZI-C"),
        }
    }
}

/// Order type (bid or ask)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Bid,
    Ask,
}

/// A unit to be traded
/// For buyers: has redemption value
/// For sellers: has cost
#[derive(Debug, Clone, Copy)]
pub struct Unit {
    pub value_or_cost: usize,
}

/// An order submitted to the market
#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub trader_id: usize,
    pub order_type: OrderType,
    pub price: usize,
    pub value_or_cost: usize, // Buyer's value or seller's cost for this unit
}

/// A completed transaction
#[derive(Debug, Clone, Copy)]
pub struct Transaction {
    pub sequence: usize,      // Transaction number within period
    pub buyer_id: usize,
    pub seller_id: usize,
    pub price: usize,
    pub buyer_value: usize,
    pub seller_cost: usize,
}

impl Transaction {
    /// Calculate total surplus from this transaction
    pub fn total_surplus(&self) -> i32 {
        let buyer_surplus = self.buyer_value as i32 - self.price as i32;
        let seller_surplus = self.price as i32 - self.seller_cost as i32;
        buyer_surplus + seller_surplus
    }

    /// Calculate price deviation from equilibrium
    pub fn price_deviation(&self, equilibrium_price: usize) -> i32 {
        self.price as i32 - equilibrium_price as i32
    }
}

/// Events in the ZI traders simulation
#[derive(Debug, Clone)]
pub enum Event {
    /// Start a trading period
    PeriodStart {
        period: usize,
        market_id: usize,
    },

    /// Coordinator requests order from specific trader
    OrderRequest {
        period: usize,
        trader_id: usize,
        iteration: usize,
    },

    /// Trader submits order to coordinator
    OrderSubmitted {
        period: usize,
        trader_id: usize,
        order_type: OrderType,
        price: usize,
        value_or_cost: usize, // Buyer's value or seller's cost
    },

    /// Transaction executed (broadcast to all agents)
    Transaction {
        period: usize,
        buyer_id: usize,
        seller_id: usize,
        price: usize,
        buyer_value: usize,
        seller_cost: usize,
        sequence: usize,
    },

    /// Period ends
    PeriodEnd {
        period: usize,
    },
}

/// Statistics tracked by trader agents
#[derive(Debug, Clone)]
pub struct TraderStats {
    pub trader_id: usize,
    pub trader_type: TraderType,
    pub role: Role,

    // Current state
    pub current_unit_index: usize,
    pub units_total: usize,

    // Cumulative metrics
    pub orders_submitted: usize,
    pub units_traded: usize,
    pub total_profit: i32,

    // Per-unit tracking
    pub unit_profits: Vec<i32>,
}

impl TraderStats {
    pub fn new(trader_id: usize, trader_type: TraderType, role: Role, units_total: usize) -> Self {
        TraderStats {
            trader_id,
            trader_type,
            role,
            current_unit_index: 0,
            units_total,
            orders_submitted: 0,
            units_traded: 0,
            total_profit: 0,
            unit_profits: Vec::new(),
        }
    }

    /// Check if trader has units remaining
    pub fn has_units_remaining(&self) -> bool {
        self.current_unit_index < self.units_total
    }

    /// Get number of units remaining
    pub fn units_remaining(&self) -> usize {
        self.units_total.saturating_sub(self.current_unit_index)
    }
}

/// Statistics tracked by coordinator
#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    pub period: usize,
    pub market_id: usize,

    // Current state
    pub best_bid: Option<Order>,
    pub best_ask: Option<Order>,
    pub orders_processed: usize,

    // Cumulative period metrics
    pub transactions: Vec<Transaction>,
    pub total_surplus: i32,

    // Market parameters (for analysis)
    pub equilibrium_price: usize,
    pub equilibrium_quantity: usize,
    pub max_possible_surplus: i32,
}

impl CoordinatorStats {
    pub fn new(
        period: usize,
        market_id: usize,
        equilibrium_price: usize,
        equilibrium_quantity: usize,
        max_possible_surplus: i32,
    ) -> Self {
        CoordinatorStats {
            period,
            market_id,
            best_bid: None,
            best_ask: None,
            orders_processed: 0,
            transactions: Vec::new(),
            total_surplus: 0,
            equilibrium_price,
            equilibrium_quantity,
            max_possible_surplus,
        }
    }

    /// Calculate allocative efficiency as percentage
    pub fn efficiency(&self) -> f64 {
        if self.max_possible_surplus == 0 {
            return 0.0;
        }
        (self.total_surplus as f64 / self.max_possible_surplus as f64) * 100.0
    }

    /// Get number of transactions
    pub fn num_transactions(&self) -> usize {
        self.transactions.len()
    }

    /// Calculate root mean squared deviation from equilibrium price
    pub fn price_rmsd(&self) -> f64 {
        if self.transactions.is_empty() {
            return 0.0;
        }

        let sum_sq_dev: f64 = self
            .transactions
            .iter()
            .map(|t| {
                let dev = t.price_deviation(self.equilibrium_price) as f64;
                dev * dev
            })
            .sum();

        (sum_sq_dev / self.transactions.len() as f64).sqrt()
    }
}

/// Combined stats enum for all agent types
#[derive(Debug, Clone)]
pub enum Stats {
    Trader(TraderStats),
    Coordinator(CoordinatorStats),
}
