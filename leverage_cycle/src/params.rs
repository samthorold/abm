/// GARCH(1,1) process parameters for exogenous volatility clustering
#[derive(Debug, Clone)]
pub struct GarchParams {
    /// Baseline variance (a0)
    pub a0: f64,
    /// Error autoregressive coefficient (a1)
    pub a1: f64,
    /// Variance autoregressive coefficient (b1)
    pub b1: f64,
    /// Whether GARCH noise is enabled
    pub enabled: bool,
}

impl GarchParams {
    /// Create disabled GARCH (deterministic case)
    pub fn disabled() -> Self {
        GarchParams {
            a0: 0.0,
            a1: 0.0,
            b1: 0.0,
            enabled: false,
        }
    }

    /// Strong GARCH - highly clustered volatility (microprudential scenarios)
    pub fn strong() -> Self {
        GarchParams {
            a0: 0.001,
            a1: 0.04,
            b1: 0.95,
            enabled: true,
        }
    }

    /// Weak GARCH - more stable volatility (macroprudential scenarios)
    pub fn weak() -> Self {
        GarchParams {
            a0: 0.001,
            a1: 0.016,
            b1: 0.874,
            enabled: true,
        }
    }

    /// Compute unconditional variance of the GARCH process
    pub fn unconditional_variance(&self) -> f64 {
        if !self.enabled || (1.0 - self.a1 - self.b1).abs() < 1e-10 {
            return self.a0;
        }
        self.a0 / (1.0 - self.a1 - self.b1)
    }
}

impl Default for GarchParams {
    fn default() -> Self {
        GarchParams::weak()
    }
}

/// Model parameters for the Basel leverage cycle simulation
#[derive(Debug, Clone)]
pub struct ModelParams {
    /// Time step in years
    pub tau: f64,
    /// VaR time horizon in years
    pub t_var: f64,
    /// Memory parameter for volatility estimation (1/delta = lookback period in years)
    pub delta: f64,
    /// Leverage adjustment speed (year^-1)
    pub theta: f64,
    /// Equity redistribution speed (year^-1)
    pub eta: f64,
    /// Cyclicality parameter: -0.5 = Basel II, 0 = constant, +0.5 = countercyclical
    pub b: f64,
    /// Risk level parameter (controls average leverage)
    pub alpha: f64,
    /// Risk offset to bound leverage when volatility is very low
    pub sigma_0_sq: f64,
    /// Bank equity target
    pub e_bar: f64,
    /// Bank's portfolio weight in risky asset
    pub w_b: f64,
    /// Fundamental price
    pub mu: f64,
    /// Fund's mean reversion rate (year^-1)
    pub rho: f64,
    /// GARCH parameters for exogenous noise
    pub garch: GarchParams,
}

impl ModelParams {
    /// Compute target leverage given perceived variance
    pub fn target_leverage(&self, sigma_sq: f64) -> f64 {
        self.alpha * (sigma_sq + self.sigma_0_sq).powf(self.b)
    }

    /// Create parameters for deterministic microprudential scenario (i)
    /// Small bank, no noise - converges to fixed point
    pub fn deterministic_micro() -> Self {
        ModelParams {
            e_bar: 1e-5,
            garch: GarchParams::disabled(),
            ..Self::default()
        }
    }

    /// Create parameters for deterministic macroprudential scenario (ii)
    /// Large bank, no noise - shows distinct dynamics from micro case
    ///
    /// Uses mildly procyclical policy (b = -0.25) instead of full Basel II
    /// to keep dynamics bounded while still showing endogenous cycles.
    pub fn deterministic_macro() -> Self {
        ModelParams {
            e_bar: 0.01, // Larger than micro (1e-5) but still moderate
            b: -0.25,    // Mildly procyclical (less extreme than Basel II)
            garch: GarchParams::disabled(),
            ..Self::default()
        }
    }

    /// Create parameters for stochastic microprudential scenario (iii)
    /// Small bank, strong GARCH - mean-reverting random walk
    pub fn stochastic_micro() -> Self {
        ModelParams {
            e_bar: 1e-5,
            garch: GarchParams::strong(),
            ..Self::default()
        }
    }

    /// Create parameters for stochastic macroprudential scenario (iv)
    /// Large bank, weak GARCH - irregular leverage cycles
    ///
    /// Uses mildly procyclical policy with GARCH noise for irregular dynamics.
    pub fn stochastic_macro() -> Self {
        ModelParams {
            e_bar: 0.01, // Larger than micro but moderate
            b: -0.25,    // Mildly procyclical
            garch: GarchParams::weak(),
            ..Self::default()
        }
    }
}

impl Default for ModelParams {
    fn default() -> Self {
        ModelParams {
            tau: 0.1,         // Time step: 0.1 years
            t_var: 0.1,       // VaR horizon: 0.1 years
            delta: 0.5,       // ~2 year lookback for volatility
            theta: 10.0,      // Leverage adjustment speed
            eta: 10.0,        // Equity redistribution speed
            b: -0.5,          // Basel II (procyclical)
            alpha: 0.075,     // Risk level
            sigma_0_sq: 1e-6, // Risk offset
            e_bar: 0.15,      // Bank equity target (moderate, for stable dynamics)
            w_b: 0.3,         // Bank portfolio weight
            mu: 25.0,         // Fundamental price
            rho: 0.1,         // Fund mean reversion rate
            garch: GarchParams::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_leverage_procyclical() {
        let params = ModelParams::default();
        // With b = -0.5, leverage should decrease as volatility increases
        let low_vol = params.target_leverage(0.01);
        let high_vol = params.target_leverage(0.1);
        assert!(
            low_vol > high_vol,
            "Procyclical: lower vol should give higher leverage"
        );
    }

    #[test]
    fn target_leverage_constant() {
        let params = ModelParams {
            b: 0.0,
            ..Default::default()
        };
        // With b = 0, leverage should be constant (alpha)
        let low_vol = params.target_leverage(0.01);
        let high_vol = params.target_leverage(0.1);
        assert!(
            (low_vol - high_vol).abs() < 1e-10,
            "Constant leverage with b=0"
        );
    }

    #[test]
    fn garch_unconditional_variance() {
        let garch = GarchParams::weak();
        let uncond = garch.unconditional_variance();
        // Should be a0 / (1 - a1 - b1) = 0.001 / (1 - 0.016 - 0.874) = 0.001 / 0.11 ≈ 0.009
        assert!(uncond > 0.008 && uncond < 0.010);
    }
}
