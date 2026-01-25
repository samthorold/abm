use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

use crate::params::GarchParams;

/// GARCH(1,1) noise process for exogenous volatility clustering
///
/// The process generates noise with time-varying conditional variance:
///   s²(t) = a₀ + a₁ × χ²(t-1) + b₁ × s²(t-1)
///   χ(t) = s(t) × ξ(t)
///
/// where ξ(t) ~ N(0,1)
#[derive(Debug, Clone)]
pub struct GarchProcess {
    /// Current conditional variance s²
    variance: f64,
    /// Previous shock χ(t-1)
    prev_shock: f64,
    /// Parameters
    params: GarchParams,
}

impl GarchProcess {
    /// Create a new GARCH process initialized at unconditional variance
    pub fn new(params: GarchParams) -> Self {
        let variance = params.unconditional_variance();
        GarchProcess {
            variance,
            prev_shock: 0.0,
            params,
        }
    }

    /// Create a new GARCH process with specified initial variance
    pub fn with_initial_variance(params: GarchParams, initial_variance: f64) -> Self {
        GarchProcess {
            variance: initial_variance,
            prev_shock: 0.0,
            params,
        }
    }

    /// Generate the next shock and update internal state
    ///
    /// Returns (s, xi) where:
    /// - s is the conditional standard deviation
    /// - xi is the standard normal innovation
    pub fn next<R: Rng>(&mut self, rng: &mut R) -> (f64, f64) {
        if !self.params.enabled {
            return (0.0, 0.0);
        }

        // Update variance: s²(t) = a₀ + a₁ × χ²(t-1) + b₁ × s²(t-1)
        self.variance = self.params.a0
            + self.params.a1 * self.prev_shock.powi(2)
            + self.params.b1 * self.variance;

        // Generate standard normal innovation
        let xi: f64 = StandardNormal.sample(rng);

        // Compute shock: χ(t) = s(t) × ξ(t)
        let s = self.variance.sqrt();
        let chi = s * xi;

        // Store for next iteration
        self.prev_shock = chi;

        (s, xi)
    }

    /// Get current conditional variance
    pub fn variance(&self) -> f64 {
        self.variance
    }

    /// Get current conditional standard deviation
    pub fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }

    /// Check if GARCH is enabled
    pub fn is_enabled(&self) -> bool {
        self.params.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn disabled_garch_returns_zero() {
        let mut garch = GarchProcess::new(GarchParams::disabled());
        let mut rng = StdRng::seed_from_u64(42);

        let (s, xi) = garch.next(&mut rng);
        assert_eq!(s, 0.0);
        assert_eq!(xi, 0.0);
    }

    #[test]
    fn garch_produces_positive_variance() {
        let mut garch = GarchProcess::new(GarchParams::weak());
        let mut rng = StdRng::seed_from_u64(42);

        for _ in 0..100 {
            garch.next(&mut rng);
            assert!(garch.variance() > 0.0);
        }
    }

    #[test]
    fn garch_deterministic_with_seed() {
        let mut garch1 = GarchProcess::new(GarchParams::weak());
        let mut garch2 = GarchProcess::new(GarchParams::weak());
        let mut rng1 = StdRng::seed_from_u64(123);
        let mut rng2 = StdRng::seed_from_u64(123);

        for _ in 0..50 {
            let (s1, xi1) = garch1.next(&mut rng1);
            let (s2, xi2) = garch2.next(&mut rng2);
            assert_eq!(s1, s2);
            assert_eq!(xi1, xi2);
        }
    }

    #[test]
    fn strong_garch_more_volatile_than_weak() {
        let mut strong = GarchProcess::new(GarchParams::strong());
        let mut weak = GarchProcess::new(GarchParams::weak());
        let mut rng = StdRng::seed_from_u64(42);

        // Run both processes and accumulate variance changes
        let mut strong_var_changes = 0.0;
        let mut weak_var_changes = 0.0;
        let mut prev_strong_var = strong.variance();
        let mut prev_weak_var = weak.variance();

        for _ in 0..500 {
            strong.next(&mut rng);
            weak.next(&mut rng);

            strong_var_changes += (strong.variance() - prev_strong_var).abs();
            weak_var_changes += (weak.variance() - prev_weak_var).abs();

            prev_strong_var = strong.variance();
            prev_weak_var = weak.variance();
        }

        // Strong GARCH should have more variance fluctuation
        assert!(
            strong_var_changes > weak_var_changes,
            "Strong GARCH should be more volatile"
        );
    }
}
