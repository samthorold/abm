use crate::{ExposureDecision, ModelConfig};

/// Tracks exposure by peril region for a single syndicate
#[derive(Debug, Clone)]
pub struct PerilRegionExposure {
    pub peril_region: usize,
    pub total_exposure: f64, // Sum of (line_size × risk_limit) for all risks in this region
}

/// VaR-based exposure management for Scenario 3 (Olmez et al. 2024).
///
/// Limits per-region exposure using an analytical VaR: for each peril region,
/// the expected catastrophe loss (region_exposure × E[damage_fraction]) must
/// not exceed capital × var_safety_factor.  This forces syndicates to spread
/// risk uniformly across regions, reducing concentration and insolvency risk.
pub struct VarExposureManager {
    peril_exposures: Vec<PerilRegionExposure>,
    capital: f64,
    config: ModelConfig,
    num_peril_regions: usize,
}

impl VarExposureManager {
    pub fn new(config: ModelConfig, capital: f64) -> Self {
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
        }
    }

    /// Evaluate whether to accept a new quote based on VaR constraints.
    ///
    /// Uses a per-region analytical VaR: the expected loss if a catastrophe hits
    /// the proposed peril region (region_exposure × E[damage_fraction]).
    ///
    /// Monte Carlo at the standard cat_prob (0.005/region/year) gives a near-zero
    /// 95th-percentile VaR because ~95% of simulations have zero catastrophes, so
    /// the Monte Carlo is replaced with this deterministic equivalent.
    ///
    /// Returns ExposureDecision:
    /// - Accept: expected catastrophe loss within capital threshold
    /// - Reject: already at/above threshold
    /// - ScalePremium(factor): near threshold, syndicate quotes higher to reduce selection probability
    pub fn evaluate_quote(
        &mut self,
        peril_region: usize,
        proposed_exposure: f64, // line_size × risk_limit
    ) -> ExposureDecision {
        if self.config.var_exceedance_prob <= 0.0 {
            return ExposureDecision::Accept;
        }

        let current_region_exposure = self.peril_exposures[peril_region].total_exposure;
        let proposed_region_exposure = current_region_exposure + proposed_exposure;

        // Per-region VaR = expected loss from a catastrophe hitting this region
        // E[damage] = (min_damage + 1.0) / 2 (uniform distribution)
        let mean_damage = (self.config.min_cat_damage_fraction + 1.0) / 2.0;
        let current_var = current_region_exposure * mean_damage;
        let proposed_var = proposed_region_exposure * mean_damage;

        let var_threshold = self.capital * self.config.var_safety_factor;

        if proposed_var <= var_threshold {
            ExposureDecision::Accept
        } else if current_var >= var_threshold {
            ExposureDecision::Reject
        } else {
            // Between thresholds: scale premium so this syndicate quotes higher and
            // is outcompeted by syndicates with more capacity in this region.
            let excess_ratio = proposed_var / var_threshold;
            let scale_factor = excess_ratio.max(1.0).min(self.config.max_scaling_factor);
            ExposureDecision::ScalePremium(scale_factor)
        }
    }

    /// Reset per-region exposures at year end (active policies expire after 365 days).
    pub fn reset_exposures(&mut self) {
        for exposure in &mut self.peril_exposures {
            exposure.total_exposure = 0.0;
        }
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
        let manager = VarExposureManager::new(config.clone(), 10_000_000.0);

        assert_eq!(manager.peril_exposures.len(), config.num_peril_regions);
        assert_eq!(manager.capital, 10_000_000.0);
    }

    #[test]
    fn test_uniform_deviation_empty() {
        let config = ModelConfig::default();
        let manager = VarExposureManager::new(config, 10_000_000.0);

        // With no exposure, deviation should be 0
        assert_eq!(manager.uniform_deviation(), 0.0);
    }

    #[test]
    fn test_uniform_deviation_uniform() {
        let config = ModelConfig::default();
        let mut manager = VarExposureManager::new(config, 10_000_000.0);

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
        let mut manager = VarExposureManager::new(config, 10_000_000.0);

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
        let mut manager = VarExposureManager::new(config, 10_000_000.0);

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
        let mut manager = VarExposureManager::new(config, 10_000_000.0);

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
        let mut manager = VarExposureManager::new(config, 10_000_000.0);

        manager.record_exposure(0, 2_000_000.0);
        manager.record_exposure(0, 3_000_000.0);

        assert_eq!(manager.peril_exposures[0].total_exposure, 5_000_000.0);
    }
}
