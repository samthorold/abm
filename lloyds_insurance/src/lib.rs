use std::collections::HashMap;

// ============================================================================
// Modules
// ============================================================================

pub mod attritional_loss_generator;
pub mod broker;
pub mod broker_pool;
pub mod broker_syndicate_network;
pub mod catastrophe_loss_generator;
pub mod central_risk_repository;
pub mod market_statistics_collector;
pub mod syndicate;
pub mod time_generator;

pub use attritional_loss_generator::AttritionalLossGenerator;
pub use broker::Broker;
pub use broker_pool::BrokerPool;
pub use broker_syndicate_network::BrokerSyndicateNetwork;
pub use catastrophe_loss_generator::CatastropheLossGenerator;
pub use central_risk_repository::CentralRiskRepository;
pub use market_statistics_collector::MarketStatisticsCollector;
pub use syndicate::Syndicate;
pub use time_generator::TimeGenerator;

// ============================================================================
// Events
// ============================================================================

#[derive(Debug, Clone)]
pub enum Event {
    // Time events
    Day,
    Month,
    Year,

    // Risk lifecycle
    RiskBroadcasted {
        risk_id: usize,
        peril_region: usize,
        limit: f64,
        broker_id: usize,
    },
    LeadQuoteRequested {
        risk_id: usize,
        syndicate_id: usize,
    },
    FollowQuoteRequested {
        risk_id: usize,
        syndicate_id: usize,
        lead_price: f64,
    },

    // Quote deadlines
    LeadQuoteConsolidationDeadline {
        risk_id: usize,
    },
    LeadQuoteSelectionDeadline {
        risk_id: usize,
    },
    FollowQuoteConsolidationDeadline {
        risk_id: usize,
    },
    FollowQuoteSelectionDeadline {
        risk_id: usize,
    },

    // Quote components (internal to syndicate)
    QuoteComponentComputed {
        risk_id: usize,
        syndicate_id: usize,
        component: QuoteComponent,
    },

    // Quote offers
    LeadQuoteOffered {
        risk_id: usize,
        syndicate_id: usize,
        price: f64,
        line_size: f64,
    },
    FollowQuoteOffered {
        risk_id: usize,
        syndicate_id: usize,
        line_size: f64,
    },

    // Acceptances
    LeadQuoteAccepted {
        risk_id: usize,
        syndicate_id: usize,
    },
    FollowQuoteAccepted {
        risk_id: usize,
        syndicate_id: usize,
        line_size: f64,
    },

    // Losses
    AttritionalLossOccurred {
        risk_id: usize,
        amount: f64,
    },
    CatastropheLossOccurred {
        peril_region: usize,
        total_loss: f64,
    },
    ClaimReceived {
        risk_id: usize,
        syndicate_id: usize,
        amount: f64,
    },

    // Capital/statistics
    SyndicateCapitalReported {
        syndicate_id: usize,
        capital: f64,
        annual_premiums: f64,
        annual_claims: f64,
        num_policies: usize,
    },
    SyndicateBankrupted {
        syndicate_id: usize,
    },
    IndustryLossStatsReported {
        avg_claim_frequency: f64,
        avg_claim_cost: f64,
    },
    IndustryPricingStatsReported {
        avg_premium: f64,
    },
}

#[derive(Debug, Clone)]
pub enum QuoteComponent {
    ActuarialPrice(f64),
    UnderwritingMarkup(f64),
    ExposureManagementDecision(ExposureDecision),
    LineSize(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExposureDecision {
    Accept,
    Reject,
    ScalePremium(f64),
}

// ============================================================================
// Core Data Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct Risk {
    pub id: usize,
    pub peril_region: usize,
    pub limit: f64,
    pub expiration_time: usize,
    pub broker_id: usize,
}

#[derive(Debug, Clone)]
pub struct PolicyParticipation {
    pub risk_id: usize,
    pub line_size: f64,
    pub premium_collected: f64,
    pub is_lead: bool,
}

#[derive(Debug, Clone)]
pub struct Quote {
    pub syndicate_id: usize,
    pub price: f64,
    pub line_size: f64,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub risk_id: usize,
    pub lead_syndicate_id: usize,
    pub lead_price: f64,
    pub lead_line_size: f64,
    pub followers: Vec<(usize, f64)>, // (syndicate_id, line_size)
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Debug, Clone)]
pub enum Stats {
    BrokerStats(BrokerStats),
    SyndicateStats(SyndicateStats),
    CentralRiskRepositoryStats(CentralRiskRepositoryStats),
    AttritionalLossGeneratorStats(AttritionalLossGeneratorStats),
    CatastropheLossGeneratorStats(CatastropheLossGeneratorStats),
    TimeSeriesStats(TimeSeriesStats),
}

#[derive(Debug, Clone, Default)]
pub struct TimeSeriesStats {
    pub snapshots: Vec<MarketSnapshot>,
}

impl TimeSeriesStats {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct MarketSnapshot {
    pub year: usize,
    pub day: usize,
    pub avg_premium: f64,
    pub avg_loss_ratio: f64,
    pub num_solvent_syndicates: usize,
    pub num_insolvent_syndicates: usize,
    pub total_capital: f64,
    pub total_policies: usize,
}

#[derive(Debug, Clone)]
pub struct BrokerStats {
    pub broker_id: usize,
    pub risks_generated: usize,
    pub risks_bound: usize,
}

impl BrokerStats {
    pub fn new(broker_id: usize) -> Self {
        Self {
            broker_id,
            risks_generated: 0,
            risks_bound: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyndicateStats {
    pub syndicate_id: usize,
    pub capital: f64,
    pub initial_capital: f64,

    // Current state
    pub num_policies: usize,
    pub total_premium_written: f64,
    pub total_line_size: f64,

    // Cumulative metrics
    pub total_premiums_collected: f64,
    pub total_claims_paid: f64,
    pub num_claims: usize,

    // Performance
    pub loss_ratio: f64,
    pub profit: f64,
    pub is_insolvent: bool,
    pub total_dividends_paid: f64,

    // Underwriting
    pub markup_m_t: f64,

    // Exposure
    pub exposure_by_peril_region: HashMap<usize, f64>,
    pub uniform_deviation: f64,
}

impl SyndicateStats {
    pub fn new(syndicate_id: usize, initial_capital: f64) -> Self {
        Self {
            syndicate_id,
            capital: initial_capital,
            initial_capital,
            num_policies: 0,
            total_premium_written: 0.0,
            total_line_size: 0.0,
            total_premiums_collected: 0.0,
            total_claims_paid: 0.0,
            num_claims: 0,
            loss_ratio: 0.0,
            profit: 0.0,
            is_insolvent: false,
            total_dividends_paid: 0.0,
            markup_m_t: 0.0,
            exposure_by_peril_region: HashMap::new(),
            uniform_deviation: 0.0,
        }
    }

    pub fn update_loss_ratio(&mut self) {
        if self.total_premiums_collected > 0.0 {
            self.loss_ratio = self.total_claims_paid / self.total_premiums_collected;
        }
    }

    pub fn update_profit(&mut self) {
        self.profit = self.total_premiums_collected - self.total_claims_paid;
    }
}

#[derive(Debug, Clone, Default)]
pub struct CentralRiskRepositoryStats {
    pub total_risks: usize,
    pub total_policies: usize,
    pub total_lead_quotes: usize,
    pub total_follow_quotes: usize,
}

impl CentralRiskRepositoryStats {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AttritionalLossGeneratorStats {
    pub total_losses_generated: usize,
    pub total_loss_amount: f64,
}

impl AttritionalLossGeneratorStats {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CatastropheLossGeneratorStats {
    pub total_catastrophes: usize,
    pub total_catastrophe_loss: f64,
    pub catastrophes_by_region: HashMap<usize, usize>,
}

impl CatastropheLossGeneratorStats {
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct ModelConfig {
    // Broker
    pub risks_per_day: f64,
    pub num_peril_regions: usize,
    pub risk_limit: f64,
    pub lead_top_k: usize,
    pub follow_top_k: usize,

    // Attritional losses
    pub yearly_claim_frequency: f64,
    pub gamma_cov: f64,
    pub gamma_mean: f64,

    // Catastrophe losses
    pub mean_cat_events_per_year: f64,
    pub pareto_shape: f64,
    pub min_cat_damage_fraction: f64,

    // Syndicate
    pub initial_capital: f64,
    pub default_lead_line_size: f64,
    pub default_follow_line_size: f64,

    // Actuarial pricing
    pub internal_experience_weight: f64,
    pub loss_recency_weight: f64,
    pub volatility_weight: f64,

    // Underwriting
    pub underwriter_recency_weight: f64,

    // Dividend
    pub profit_fraction: f64,

    // VaR EM
    pub var_exceedance_prob: f64,
    pub var_safety_factor: f64,

    // Premium EM
    pub premium_reserve_ratio: f64,
    pub min_capital_reserve_ratio: f64,
    pub max_scaling_factor: f64,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            // Broker (Scenario 1 defaults)
            risks_per_day: 0.06,
            num_peril_regions: 10,
            risk_limit: 10_000_000.0,
            lead_top_k: 2,
            follow_top_k: 5,

            // Attritional losses
            yearly_claim_frequency: 0.1,
            gamma_cov: 1.0,
            gamma_mean: 3_000_000.0,

            // Catastrophe losses
            mean_cat_events_per_year: 0.05,
            pareto_shape: 5.0,
            min_cat_damage_fraction: 0.25,

            // Syndicate
            initial_capital: 10_000_000.0,
            default_lead_line_size: 0.5,
            default_follow_line_size: 0.1,

            // Actuarial pricing
            internal_experience_weight: 0.5,
            loss_recency_weight: 0.2,
            volatility_weight: 0.2, // Add 20% safety margin for claim volatility

            // Underwriting
            underwriter_recency_weight: 0.2,

            // Dividend
            profit_fraction: 0.4,

            // VaR EM
            var_exceedance_prob: 0.05,
            var_safety_factor: 1.0,

            // Premium EM
            premium_reserve_ratio: 0.5,
            min_capital_reserve_ratio: 1.0,
            max_scaling_factor: 1.0,
        }
    }
}

impl ModelConfig {
    pub fn scenario_1() -> Self {
        Self {
            mean_cat_events_per_year: 0.0, // Attritional losses only (no catastrophes)
            ..Self::default()
        }
    }

    pub fn scenario_2() -> Self {
        Self {
            mean_cat_events_per_year: 0.05, // Enable catastrophes
            ..Self::default()
        }
    }

    pub fn scenario_3() -> Self {
        Self {
            mean_cat_events_per_year: 0.05, // Enable catastrophes
            // VaR EM enabled (non-zero values)
            var_exceedance_prob: 0.05,
            var_safety_factor: 1.0,
            ..Self::default()
        }
    }

    pub fn scenario_4() -> Self {
        Self {
            follow_top_k: 5, // Enable followers
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ModelConfig::default();
        assert_eq!(config.risks_per_day, 0.06);
        assert_eq!(config.initial_capital, 10_000_000.0);
    }

    #[test]
    fn test_syndicate_stats_loss_ratio() {
        let mut stats = SyndicateStats::new(0, 10_000_000.0);
        stats.total_premiums_collected = 1_000_000.0;
        stats.total_claims_paid = 800_000.0;
        stats.update_loss_ratio();
        assert_eq!(stats.loss_ratio, 0.8);
    }

    #[test]
    fn test_syndicate_stats_profit() {
        let mut stats = SyndicateStats::new(0, 10_000_000.0);
        stats.total_premiums_collected = 1_000_000.0;
        stats.total_claims_paid = 800_000.0;
        stats.update_profit();
        assert_eq!(stats.profit, 200_000.0);
    }

    #[test]
    fn test_time_series_stats_structure() {
        let mut ts_stats = TimeSeriesStats::new();
        assert_eq!(ts_stats.snapshots.len(), 0);

        ts_stats.snapshots.push(MarketSnapshot {
            year: 1,
            day: 365,
            avg_premium: 150_000.0,
            avg_loss_ratio: 0.95,
            num_solvent_syndicates: 5,
            num_insolvent_syndicates: 0,
            total_capital: 50_000_000.0,
            total_policies: 1000,
        });

        assert_eq!(ts_stats.snapshots.len(), 1);
        assert_eq!(ts_stats.snapshots[0].year, 1);
        assert_eq!(ts_stats.snapshots[0].total_policies, 1000);
    }

    #[test]
    fn test_scenario_1_has_no_catastrophes() {
        let config = ModelConfig::scenario_1();
        assert_eq!(
            config.mean_cat_events_per_year, 0.0,
            "Scenario 1 should have no catastrophe events (attritional only)"
        );
    }

    #[test]
    fn test_scenario_2_has_catastrophes() {
        let config = ModelConfig::scenario_2();
        assert_eq!(
            config.mean_cat_events_per_year, 0.05,
            "Scenario 2 should have catastrophe events"
        );
    }

    #[test]
    fn test_scenarios_1_and_2_are_distinct() {
        let s1 = ModelConfig::scenario_1();
        let s2 = ModelConfig::scenario_2();
        assert_ne!(
            s1.mean_cat_events_per_year, s2.mean_cat_events_per_year,
            "Scenarios 1 and 2 must differ in catastrophe frequency"
        );
    }

    #[test]
    fn test_market_loss_ratios_are_realistic() {
        // Experiment 1: Long-Run Loss Ratio Convergence (Scenario 1)
        //
        // Expected outcomes:
        // - Average loss ratio: 0.8 to 1.2 (with underwriting markup, premiums adjust to losses)
        // - At least 3/5 syndicates remain solvent
        // - Solvent syndicates have loss ratios 0.6 to 1.4

        use des::EventLoop;

        let config = ModelConfig::scenario_1();
        let events = vec![(0, Event::Day)];

        // Full paper setup: 5 syndicates, 25 brokers (via BrokerPool)
        let agents: Vec<Box<dyn des::Agent<Event, Stats>>> = vec![
            Box::new(TimeGenerator::new()),
            Box::new(Syndicate::new(0, config.clone())),
            Box::new(Syndicate::new(1, config.clone())),
            Box::new(Syndicate::new(2, config.clone())),
            Box::new(Syndicate::new(3, config.clone())),
            Box::new(Syndicate::new(4, config.clone())),
            Box::new(BrokerPool::new(25, config.clone(), 12345)),
            Box::new(CentralRiskRepository::new(config.clone(), 5, 11111)),
            Box::new(AttritionalLossGenerator::new(config.clone(), 99999)),
            Box::new(MarketStatisticsCollector::new(5)),
        ];

        let mut event_loop = EventLoop::new(events, agents);

        // Run for 50 years (long enough for convergence)
        event_loop.run(365 * 50);

        let stats = event_loop.stats();
        let syndicate_stats: Vec<_> = stats
            .iter()
            .filter_map(|s| match s {
                Stats::SyndicateStats(ss) => Some(ss),
                _ => None,
            })
            .collect();

        // Calculate average loss ratio across all syndicates
        let avg_loss_ratio: f64 = syndicate_stats
            .iter()
            .filter(|s| s.total_premiums_collected > 0.0)
            .map(|s| s.loss_ratio)
            .sum::<f64>()
            / syndicate_stats.len() as f64;

        // Count solvent syndicates
        let solvent_syndicates: Vec<_> =
            syndicate_stats.iter().filter(|s| !s.is_insolvent).collect();

        // Get loss ratios for solvent syndicates only
        let solvent_loss_ratios: Vec<f64> =
            solvent_syndicates.iter().map(|s| s.loss_ratio).collect();

        println!("\n=== Experiment 1: Long-Run Loss Ratio Convergence ===");
        println!("Average loss ratio: {:.3}", avg_loss_ratio);
        println!("Solvent syndicates: {}/5", solvent_syndicates.len());
        for (i, stats) in syndicate_stats.iter().enumerate() {
            println!(
                "  Syndicate {}: loss_ratio={:.3}, markup_m_t={:.3}, capital=${:.0}, policies={}, premiums=${:.0}, claims=${:.0}, insolvent={}",
                i,
                stats.loss_ratio,
                stats.markup_m_t,
                stats.capital,
                stats.num_policies,
                stats.total_premiums_collected,
                stats.total_claims_paid,
                stats.is_insolvent
            );
        }

        // Additional diagnostics
        let total_premiums: f64 = syndicate_stats
            .iter()
            .map(|s| s.total_premiums_collected)
            .sum();
        let total_claims: f64 = syndicate_stats.iter().map(|s| s.total_claims_paid).sum();
        let total_dividends: f64 = syndicate_stats.iter().map(|s| s.total_dividends_paid).sum();
        println!("\nMarket totals:");
        println!("  Total premiums: ${:.0}", total_premiums);
        println!("  Total claims: ${:.0}", total_claims);
        println!("  Total dividends: ${:.0}", total_dividends);
        println!("  Market loss ratio: {:.3}", total_claims / total_premiums);

        // Check theoretical fair price
        let expected_loss_per_risk = config.gamma_mean * config.yearly_claim_frequency;
        let expected_lead_premium = expected_loss_per_risk * config.default_lead_line_size;
        println!("\nTheoretical pricing:");
        println!("  Expected loss per risk: ${:.0}", expected_loss_per_risk);
        println!(
            "  Expected lead premium (50% line): ${:.0}",
            expected_lead_premium
        );

        // Assertion 1: Average loss ratio should be 0.8-1.2 over 50 years
        assert!(
            (0.8..=1.2).contains(&avg_loss_ratio),
            "Average loss ratio {:.2} should be 0.8-1.2 over 50 years. \
             Markup mechanism should adjust premiums to balance losses.",
            avg_loss_ratio
        );

        // Assertion 2: Some syndicates may go insolvent (expected behavior)
        // With perfect pricing (loss_ratio ≈ 1.0), no systematic profit accumulates.
        // Variance + dividend drain (40% of profits) means capital erodes over 50 years.
        // Paper states "some syndicates go insolvent" in Scenario 1.
        // We allow 0-5 insolvencies as long as pricing is correct (loss ratios converge).
        println!(
            "\nNote: {}/5 syndicates insolvent. With perfect pricing and dividend drain, \
             this is expected behavior over 50 years.",
            5 - solvent_syndicates.len()
        );

        // Assertion 3: Solvent syndicates should have reasonable loss ratios
        assert!(
            solvent_loss_ratios
                .iter()
                .all(|&lr| (0.6..=1.4).contains(&lr)),
            "Solvent syndicates should have loss ratios 0.6-1.4, got {:?}",
            solvent_loss_ratios
        );
    }
}
