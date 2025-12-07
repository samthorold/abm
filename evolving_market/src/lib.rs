use rand::Rng;
use rand_distr::{Distribution, Normal};
use std::fmt::Debug;

pub mod agents;
pub mod coordinator;

/// Session type (Morning or Afternoon)
/// For minimal model, we only use Morning
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    Morning,
    #[allow(dead_code)]
    Afternoon, // For future expansion
}

/// Events in the evolving market simulation
#[derive(Debug, Clone)]
pub enum MarketEvent {
    /// Start of a trading session
    SessionStart { day: usize, session: Session },
    /// Buyers choose which seller to visit
    BuyersChooseSellers { day: usize, session: Session },
    /// Sellers process their queues and make price offers
    ProcessQueues { day: usize, session: Session },
    /// A transaction occurs (or fails)
    Transaction {
        day: usize,
        session: Session,
        buyer_id: usize,
        seller_id: usize,
        price: Option<usize>, // None if denied service
        accepted: bool,
    },
    /// End of trading day - trigger learning updates
    DayEnd { day: usize },
}

/// Buyer type based on valuation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyerType {
    Low,    // p_out = 12
    Medium, // p_out = 15
    High,   // p_out = 18
}

impl BuyerType {
    pub fn valuation(&self) -> usize {
        match self {
            BuyerType::Low => 12,
            BuyerType::Medium => 15,
            BuyerType::High => 18,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuyerType::Low => "Low",
            BuyerType::Medium => "Medium",
            BuyerType::High => "High",
        }
    }
}

/// Statistics tracked by agents
#[derive(Debug, Clone)]
pub struct MarketStats {
    pub day: usize,
    pub agent_type: String,
    pub agent_id: usize,
    // Price stats
    pub prices_offered: Vec<usize>,
    pub prices_accepted: Vec<usize>,
    // Transaction stats
    pub transactions_completed: usize,
    pub transactions_denied: usize,
    pub transactions_rejected: usize,
    // For buyers: which sellers visited
    pub sellers_visited: Vec<usize>,
    // For buyers: buyer type
    pub buyer_type: Option<String>,
    // For sellers: revenue and stock
    pub revenue: Option<usize>,
    pub stock_remaining: Option<usize>,
}

impl MarketStats {
    pub fn new_buyer(day: usize, buyer_id: usize, buyer_type: BuyerType) -> Self {
        MarketStats {
            day,
            agent_type: "Buyer".to_string(),
            agent_id: buyer_id,
            prices_offered: Vec::new(),
            prices_accepted: Vec::new(),
            transactions_completed: 0,
            transactions_denied: 0,
            transactions_rejected: 0,
            sellers_visited: Vec::new(),
            buyer_type: Some(buyer_type.name().to_string()),
            revenue: None,
            stock_remaining: None,
        }
    }

    pub fn new_seller(day: usize, seller_id: usize) -> Self {
        MarketStats {
            day,
            agent_type: "Seller".to_string(),
            agent_id: seller_id,
            prices_offered: Vec::new(),
            prices_accepted: Vec::new(),
            transactions_completed: 0,
            transactions_denied: 0,
            transactions_rejected: 0,
            sellers_visited: Vec::new(),
            buyer_type: None,
            revenue: Some(0),
            stock_remaining: None,
        }
    }
}

/// Classifier system infrastructure for reinforcement learning
/// Based on Kirman & Vriend (2001) ACE model
/// A rule in the classifier system
/// Condition can be any type (or unit type () for unconditional rules)
/// Action is the decision this rule recommends
#[derive(Clone, Debug)]
pub struct Rule<C, A> {
    pub condition: C,
    pub action: A,
    pub strength: f64,
}

impl<C, A> Rule<C, A> {
    pub fn new(condition: C, action: A) -> Self {
        Rule {
            condition,
            action,
            strength: 1.0, // All rules start with strength 1.0
        }
    }

    pub fn new_with_strength(condition: C, action: A, strength: f64) -> Self {
        Rule {
            condition,
            action,
            strength,
        }
    }
}

/// Stochastic auction for rule selection
/// Returns index of selected rule
///
/// Algorithm from Part 3.2:
/// 1. Normalize strengths to [0,1]
/// 2. Add exploration noise (Normal with std=noise_std)
/// 3. With probability tremble_prob, use random bid
/// 4. Select highest bid
pub fn stochastic_auction<C, A>(
    rules: &[Rule<C, A>],
    noise_std: f64,
    tremble_prob: f64,
    rng: &mut impl Rng,
) -> usize {
    assert!(!rules.is_empty(), "Cannot select from empty rule set");

    if rules.len() == 1 {
        return 0;
    }

    // Normalize strengths to [0,1]
    let strengths: Vec<f64> = rules.iter().map(|r| r.strength).collect();
    let min_s = strengths.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_s = strengths.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let strengths_norm: Vec<f64> = if max_s > min_s {
        strengths
            .iter()
            .map(|s| (s - min_s) / (max_s - min_s))
            .collect()
    } else {
        vec![0.5; strengths.len()]
    };

    // Compute bids
    let normal = Normal::new(0.0, noise_std).unwrap();
    let bids: Vec<f64> = strengths_norm
        .iter()
        .map(|&s_norm| {
            if rng.random::<f64>() < tremble_prob {
                // Trembling hand: random bid
                rng.random::<f64>()
            } else {
                // Normal: strength + noise
                s_norm + normal.sample(rng)
            }
        })
        .collect();

    // Select highest bid
    bids.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx)
        .unwrap()
}

/// Update rule strength using reinforcement learning
/// Formula: s(t) = (1-c) * s(t-1) + c * reward
pub fn update_strength(current_strength: f64, reward: f64, learning_rate: f64) -> f64 {
    (1.0 - learning_rate) * current_strength + learning_rate * reward
}

/// Update loyalty value for a buyer-seller pair
/// Formula: L(t) = L(t-1)/(1+α) + (α if visited, else 0)
/// Capped at 1.0 to maintain stated range of [0, 1]
pub fn update_loyalty(prev_loyalty: f64, visited: bool, alpha: f64) -> f64 {
    let r = if visited { alpha } else { 0.0 };
    let new_loyalty = prev_loyalty / (1.0 + alpha) + r;
    new_loyalty.min(1.0) // Cap at 1.0
}

/// Classify loyalty into discrete categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoyaltyClass {
    Low,    // L < 0.20
    Medium, // 0.20 <= L < 0.80
    High,   // L >= 0.80
}

impl LoyaltyClass {
    pub fn from_value(loyalty: f64) -> Self {
        if loyalty < 0.20 {
            LoyaltyClass::Low
        } else if loyalty < 0.80 {
            LoyaltyClass::Medium
        } else {
            LoyaltyClass::High
        }
    }
}

/// Classify stock/queue ratio into discrete categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StockQueueRatio {
    Low,    // ratio < 0.75
    Medium, // 0.75 <= ratio < 1.25
    High,   // ratio >= 1.25 or no queue
}

impl StockQueueRatio {
    pub fn from_values(stock: usize, queue_length: usize) -> Self {
        if queue_length == 0 {
            return StockQueueRatio::High;
        }

        let ratio = stock as f64 / queue_length as f64;
        if ratio < 0.75 {
            StockQueueRatio::Low
        } else if ratio < 1.25 {
            StockQueueRatio::Medium
        } else {
            StockQueueRatio::High
        }
    }
}

/// Pricing condition: combination of customer loyalty and market state
/// Sellers use this to condition their pricing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PricingCondition {
    pub loyalty: LoyaltyClass,
    pub stock_queue: StockQueueRatio,
}

impl PricingCondition {
    pub fn new(loyalty: LoyaltyClass, stock_queue: StockQueueRatio) -> Self {
        PricingCondition {
            loyalty,
            stock_queue,
        }
    }
}

/// Compute loyalty concentration metric γ for a buyer
/// γ = Σ(L_ij²) / (ΣL_ij)²
/// γ=1 means perfect loyalty to one seller
/// γ=1/n means equal visits to all n sellers
pub fn loyalty_concentration(loyalties: &[f64]) -> f64 {
    let sum_sq: f64 = loyalties.iter().map(|l| l * l).sum();
    let sum: f64 = loyalties.iter().sum();

    if sum > 0.0 {
        sum_sq / (sum * sum)
    } else {
        1.0 / loyalties.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stochastic_auction_favors_strong_rules() {
        let mut rng = rand::rng();
        let rules = vec![
            Rule::new((), 0).clone(),
            Rule::new_with_strength((), 1, 10.0),
            Rule::new((), 2).clone(),
        ];

        // Run auction many times, rule 1 should win most often
        let mut counts = vec![0; 3];
        for _ in 0..1000 {
            let idx = stochastic_auction(&rules, 0.1, 0.025, &mut rng);
            counts[idx] += 1;
        }

        assert!(counts[1] > counts[0]);
        assert!(counts[1] > counts[2]);
        assert!(counts[1] > 800); // Should win ~90%+ of the time
    }

    #[test]
    fn test_update_strength_convergence() {
        let mut strength = 1.0;
        let reward = 0.5;
        let learning_rate = 0.05;

        // Should converge toward reward
        for _ in 0..100 {
            strength = update_strength(strength, reward, learning_rate);
        }

        assert!((strength - reward).abs() < 0.01);
    }

    #[test]
    fn test_update_loyalty() {
        let mut loyalty = 0.0;
        let alpha = 0.25;

        // Visit every day for several days
        for _ in 0..10 {
            loyalty = update_loyalty(loyalty, true, alpha);
        }

        // Should approach 1.0
        assert!(loyalty > 0.9);
        assert!(loyalty <= 1.0);

        // Stop visiting
        for _ in 0..10 {
            loyalty = update_loyalty(loyalty, false, alpha);
        }

        // Should decay
        assert!(loyalty < 0.5);
    }

    #[test]
    fn test_loyalty_classification() {
        assert_eq!(LoyaltyClass::from_value(0.1), LoyaltyClass::Low);
        assert_eq!(LoyaltyClass::from_value(0.5), LoyaltyClass::Medium);
        assert_eq!(LoyaltyClass::from_value(0.9), LoyaltyClass::High);
    }

    #[test]
    fn test_loyalty_concentration() {
        // Perfect loyalty to one seller
        let perfect = vec![1.0, 0.0, 0.0, 0.0];
        assert!((loyalty_concentration(&perfect) - 1.0).abs() < 0.01);

        // Equal distribution
        let equal = vec![0.25, 0.25, 0.25, 0.25];
        assert!((loyalty_concentration(&equal) - 0.25).abs() < 0.01);
    }
}
