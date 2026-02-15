use crate::{ExposureDecision, ModelConfig};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Bernoulli, Distribution};

/// Tracks exposure by peril region for a single syndicate
#[derive(Debug, Clone)]
pub struct PerilRegionExposure {
    pub peril_region: usize,
    pub total_exposure: f64, // Sum of (line_size × risk_limit) for all risks in this region
}

/// VaR-based exposure management using Monte Carlo simulation
///
/// This implements Scenario 3 from the Olmez et al. (2024) paper: sophisticated
/// exposure management that simulates catastrophe scenarios and limits exposure
/// based on Value-at-Risk at a specified exceedance probability.
pub struct VarExposureManager {
    peril_exposures: Vec<PerilRegionExposure>,
    capital: f64,
    config: ModelConfig,
    rng: StdRng,
    num_peril_regions: usize,
}

impl VarExposureManager {
    pub fn new(config: ModelConfig, capital: f64, seed: u64) -> Self {
        // Initialize exposure tracking for all peril regions
        let peril_exposures = (0..config.num_peril_regions)
            .map(|peril_region| PerilRegionExposure {
                peril_region,
                total_exposure: 0.0,
            })
            .collect();

        Self {
            peril_exposures,
            capital,
            num_peril_regions: config.num_peril_regions,
            config,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Evaluate whether to accept a new quote based on VaR constraints
    ///
    /// Returns ExposureDecision:
    /// - Accept: VaR with new exposure is within limits
    /// - Reject: VaR with new exposure exceeds limits
    /// - ScalePremium(factor): Accept but require higher premium
    pub fn evaluate_quote(
        &mut self,
        peril_region: usize,
        proposed_exposure: f64, // line_size × risk_limit
    ) -> ExposureDecision {
        // Check if VaR EM is enabled (var_exceedance_prob > 0)
        if self.config.var_exceedance_prob <= 0.0 {
            return ExposureDecision::Accept;
        }

        // Calculate current VaR
        let current_exposures = self.peril_exposures.clone();
        let current_var = self.calculate_var_internal(&current_exposures, 1000);

        // Calculate VaR with proposed exposure
        let mut proposed_exposures = self.peril_exposures.clone();
        if peril_region < proposed_exposures.len() {
            proposed_exposures[peril_region].total_exposure += proposed_exposure;
        }
        let proposed_var = self.calculate_var_internal(&proposed_exposures, 1000);

        // Check if proposed VaR exceeds capital threshold
        let var_threshold = self.capital * self.config.var_safety_factor;

        if proposed_var <= var_threshold {
            // Accept - VaR is within limits
            ExposureDecision::Accept
        } else if current_var >= var_threshold {
            // Already at/above threshold - reject new exposure
            ExposureDecision::Reject
        } else {
            // Propose premium scaling to compensate for elevated risk
            // Scale factor based on how much VaR exceeds threshold
            let excess_ratio = proposed_var / var_threshold;
            let scale_factor = excess_ratio.max(1.0).min(self.config.max_scaling_factor);
            ExposureDecision::ScalePremium(scale_factor)
        }
    }

    /// Calculate Value-at-Risk using Monte Carlo simulation (internal implementation)
    ///
    /// VaR represents the maximum expected loss at a given confidence level
    /// (determined by var_exceedance_prob). For example, with 5% exceedance
    /// probability, VaR is the 95th percentile of the loss distribution.
    fn calculate_var_internal(
        &mut self,
        exposures: &[PerilRegionExposure],
        num_simulations: usize,
    ) -> f64 {
        if num_simulations == 0 {
            return 0.0;
        }

        let mut losses = Vec::with_capacity(num_simulations);

        // Run Monte Carlo simulations of catastrophe scenarios
        for _ in 0..num_simulations {
            let mut total_loss = 0.0;

            // For each peril region, simulate whether a catastrophe occurs
            for exposure in exposures {
                // Catastrophe probability per year (Poisson approximation for rare events)
                let cat_prob = self.config.mean_cat_events_per_year / self.num_peril_regions as f64;

                if cat_prob > 0.0 {
                    let bernoulli = Bernoulli::new(cat_prob).unwrap();
                    if bernoulli.sample(&mut self.rng) {
                        // Catastrophe occurred - assume total loss of exposure
                        // (simplified - could use damage fraction from config)
                        let random_value = self.rng.gen_range(0.0..1.0);
                        let damage_fraction = self.config.min_cat_damage_fraction
                            + random_value * (1.0 - self.config.min_cat_damage_fraction);
                        total_loss += exposure.total_exposure * damage_fraction;
                    }
                }
            }

            losses.push(total_loss);
        }

        // Sort losses to find percentile
        losses.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // VaR at (1 - exceedance_prob) confidence level
        let percentile_index =
            ((1.0 - self.config.var_exceedance_prob) * num_simulations as f64) as usize;
        let var_index = percentile_index.min(num_simulations - 1);

        losses[var_index]
    }

    /// Record exposure when a quote is accepted
    pub fn record_exposure(&mut self, peril_region: usize, exposure: f64) {
        if peril_region < self.peril_exposures.len() {
            self.peril_exposures[peril_region].total_exposure += exposure;
        }
    }

    /// Update capital (called when capital changes due to claims/premiums/dividends)
    pub fn update_capital(&mut self, capital: f64) {
        self.capital = capital;
    }

    /// Calculate uniform deviation metric
    ///
    /// Returns a value between 0 and 1:
    /// - 0 = perfectly uniform distribution across peril regions
    /// - 1 = all exposure concentrated in one region
    ///
    /// Formula: std_dev / mean, normalized to [0, 1]
    pub fn uniform_deviation(&self) -> f64 {
        let total_exposure: f64 = self.peril_exposures.iter().map(|e| e.total_exposure).sum();

        if total_exposure == 0.0 || self.num_peril_regions == 0 {
            return 0.0;
        }

        let mean_exposure = total_exposure / self.num_peril_regions as f64;

        if mean_exposure == 0.0 {
            return 0.0;
        }

        // Calculate standard deviation
        let variance: f64 = self
            .peril_exposures
            .iter()
            .map(|e| (e.total_exposure - mean_exposure).powi(2))
            .sum::<f64>()
            / self.num_peril_regions as f64;

        let std_dev = variance.sqrt();

        // Coefficient of variation, capped at 1.0
        (std_dev / mean_exposure).min(1.0)
    }

    /// Get current exposures by region (for stats reporting)
    pub fn get_exposures(&self) -> Vec<(usize, f64)> {
        self.peril_exposures
            .iter()
            .map(|e| (e.peril_region, e.total_exposure))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_manager_initialization() {
        let config = ModelConfig::default();
        let manager = VarExposureManager::new(config.clone(), 10_000_000.0, 12345);

        assert_eq!(manager.peril_exposures.len(), config.num_peril_regions);
        assert_eq!(manager.capital, 10_000_000.0);
    }

    #[test]
    fn test_uniform_deviation_empty() {
        let config = ModelConfig::default();
        let manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        // With no exposure, deviation should be 0
        assert_eq!(manager.uniform_deviation(), 0.0);
    }

    #[test]
    fn test_uniform_deviation_uniform() {
        let config = ModelConfig::default();
        let mut manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        // Add uniform exposure across all regions
        for region in 0..10 {
            manager.record_exposure(region, 1_000_000.0);
        }

        // Should be close to 0 (perfectly uniform)
        assert!(manager.uniform_deviation() < 0.01);
    }

    #[test]
    fn test_uniform_deviation_concentrated() {
        let config = ModelConfig::default();
        let mut manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        // All exposure in one region
        manager.record_exposure(0, 10_000_000.0);

        // Should be high (concentrated)
        assert!(manager.uniform_deviation() > 0.9);
    }

    #[test]
    fn test_evaluate_quote_accepts_when_var_em_disabled() {
        let config = ModelConfig {
            var_exceedance_prob: 0.0, // Disable VaR EM
            ..Default::default()
        };
        let mut manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        let decision = manager.evaluate_quote(0, 5_000_000.0);
        assert_eq!(decision, ExposureDecision::Accept);
    }

    #[test]
    fn test_evaluate_quote_with_var_em_enabled() {
        let config = ModelConfig {
            var_exceedance_prob: 0.05,     // Enable VaR EM at 5%
            mean_cat_events_per_year: 0.1, // Higher cat frequency for testing
            ..Default::default()
        };
        let mut manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        // Small exposure should be accepted
        let decision = manager.evaluate_quote(0, 1_000_000.0);
        assert!(matches!(
            decision,
            ExposureDecision::Accept | ExposureDecision::ScalePremium(_)
        ));
    }

    #[test]
    fn test_record_exposure_updates_totals() {
        let config = ModelConfig::default();
        let mut manager = VarExposureManager::new(config, 10_000_000.0, 12345);

        manager.record_exposure(0, 2_000_000.0);
        manager.record_exposure(0, 3_000_000.0);

        assert_eq!(manager.peril_exposures[0].total_exposure, 5_000_000.0);
    }
}
