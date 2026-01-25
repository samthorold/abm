use crate::params::ModelParams;

/// The system state for the Basel leverage cycle model
///
/// The model is fully described by six state variables that evolve
/// as a discrete-time iterated map.
#[derive(Debug, Clone)]
pub struct SystemState {
    /// Bank's perceived risk (historical volatility estimate) σ²
    pub sigma_sq: f64,
    /// Fund's portfolio weight in the risky asset w_F
    pub w_f: f64,
    /// Current price of the risky asset p
    pub p: f64,
    /// Fraction of risky asset owned by the bank n
    pub n: f64,
    /// Bank's liabilities L_B
    pub l_b: f64,
    /// Lagged price (price at previous time step) p'
    pub p_prime: f64,
}

impl SystemState {
    /// Create initial state near the fixed point equilibrium
    pub fn initial(params: &ModelParams) -> Self {
        let sigma_sq = params.sigma_0_sq;
        let w_f = 0.5;
        let p = params.mu;

        // Compute target leverage at initial volatility
        let lambda_bar = params.target_leverage(sigma_sq);

        // Bank ownership: n = w_B × λ̄ × Ē / p
        // This comes from: A_B = λ̄ × E_B = λ̄ × Ē (at equilibrium)
        // And: A_B = p × n / w_B
        // So: p × n / w_B = λ̄ × Ē => n = w_B × λ̄ × Ē / p
        let n_raw = params.w_b * lambda_bar * params.e_bar / p;

        // Clamp n to valid range [0, 0.9] to ensure market clearing is possible
        // When n > 1, the bank would own more than 100% of the asset
        // When n is close to 1, the denominator in market clearing becomes unstable
        let n = n_raw.clamp(0.0, 0.9);

        // Initial liabilities consistent with actual ownership (not raw)
        // E_B = Ē, A_B = p × n / w_B, L_B = A_B - E_B
        let a_b = p * n / params.w_b;
        let l_b = (a_b - params.e_bar).max(0.0);

        SystemState {
            sigma_sq,
            w_f,
            p,
            n,
            l_b,
            p_prime: p,
        }
    }

    /// Compute bank assets: A_B = p × n / w_B
    pub fn bank_assets(&self, params: &ModelParams) -> f64 {
        self.p * self.n / params.w_b
    }

    /// Compute bank equity: E_B = A_B - L_B
    pub fn bank_equity(&self, params: &ModelParams) -> f64 {
        self.bank_assets(params) - self.l_b
    }

    /// Compute realized leverage: λ = A_B / E_B
    pub fn leverage(&self, params: &ModelParams) -> f64 {
        let assets = self.bank_assets(params);
        let equity = self.bank_equity(params);
        if equity.abs() < 1e-15 {
            return f64::INFINITY;
        }
        assets / equity
    }

    /// Compute log return from lagged price
    pub fn log_return(&self) -> f64 {
        if self.p_prime <= 0.0 || self.p <= 0.0 {
            return 0.0;
        }
        (self.p / self.p_prime).ln()
    }

    /// Check if state contains valid (non-NaN, non-infinite) values
    pub fn is_valid(&self) -> bool {
        self.sigma_sq.is_finite()
            && self.w_f.is_finite()
            && self.p.is_finite()
            && self.n.is_finite()
            && self.l_b.is_finite()
            && self.p_prime.is_finite()
            && self.p > 0.0
            && self.sigma_sq >= 0.0
    }

    /// Advance the system state by one time step
    ///
    /// Arguments:
    /// - params: Model parameters
    /// - s: GARCH conditional standard deviation
    /// - xi: Standard normal innovation
    ///
    /// Returns the new state after applying all 6 update equations.
    pub fn step(&self, params: &ModelParams, s: f64, xi: f64) -> Self {
        // Compute auxiliary quantities first

        // Bank assets and equity
        let a_b = self.bank_assets(params);
        let e_b = self.bank_equity(params);

        // Target leverage: λ̄ = α × (σ² + σ₀²)^b
        let lambda_bar = params.target_leverage(self.sigma_sq);

        // Balance sheet adjustment: ΔB = τ × θ × (λ̄ × E_B - A_B)
        let delta_b = params.tau * params.theta * (lambda_bar * e_b - a_b);

        // Equity redistribution: κ_B = τ × η × (Ē - E_B)
        let kappa_b = params.tau * params.eta * (params.e_bar - e_b);
        let kappa_f = -kappa_b;

        // Cash positions
        // c_B = (1 - w_B) × n × p / w_B + κ_B
        let c_b = (1.0 - params.w_b) * self.n * self.p / params.w_b + kappa_b;
        // c_F = (1 - w_F) × (1 - n) × p / w_F + κ_F
        let c_f = (1.0 - self.w_f) * (1.0 - self.n) * self.p / self.w_f + kappa_f;

        // === Equation 2: Fund's Portfolio Weight ===
        // w_F(t+τ) = w_F + (w_F/p) × [τρ(μ - p) + √τ × s × ξ]
        let mean_reversion = params.tau * params.rho * (params.mu - self.p);
        let noise = params.tau.sqrt() * s * xi;
        let w_f_new = self.w_f + (self.w_f / self.p) * (mean_reversion + noise);

        // === Equation 3: Market Clearing Price ===
        // p(t+τ) = [w_B × (c_B + ΔB) + w_F_new × c_F] / [1 - w_B × n - w_F_new × (1-n)]
        let numerator = params.w_b * (c_b + delta_b) + w_f_new * c_f;
        let denominator = 1.0 - params.w_b * self.n - w_f_new * (1.0 - self.n);
        let p_new = if denominator.abs() < 1e-15 {
            self.p // Fallback to prevent division by zero
        } else {
            numerator / denominator
        };

        // === Equation 1: Volatility Update ===
        // σ²(t+τ) = (1 - τδ) × σ² + τδ × [log(p/p') × t_VaR/τ]²
        let log_ret = self.log_return();
        let scaled_return = log_ret * (params.t_var / params.tau);
        let sigma_sq_new = (1.0 - params.tau * params.delta) * self.sigma_sq
            + params.tau * params.delta * scaled_return.powi(2);

        // === Equation 4: Bank Ownership ===
        // n(t+τ) = w_B × (n × p_new + c_B + ΔB) / p_new
        let n_new = if p_new.abs() < 1e-15 {
            self.n
        } else {
            params.w_b * (self.n * p_new + c_b + delta_b) / p_new
        };

        // === Equation 5: Bank Liabilities ===
        // L_B(t+τ) = L_B + ΔB
        let l_b_new = self.l_b + delta_b;

        // === Equation 6: Price Lag ===
        // p'(t+τ) = p(t+τ)
        let p_prime_new = p_new;

        let new_state = SystemState {
            sigma_sq: sigma_sq_new,
            w_f: w_f_new,
            p: p_new,
            n: n_new,
            l_b: l_b_new,
            p_prime: p_prime_new,
        };

        // Return previous state if new state is invalid (system has diverged)
        if new_state.is_valid() {
            new_state
        } else {
            self.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_has_positive_values() {
        let params = ModelParams::default();
        let state = SystemState::initial(&params);

        assert!(state.sigma_sq > 0.0);
        assert!(state.w_f > 0.0 && state.w_f < 1.0);
        assert!(state.p > 0.0);
        assert!(state.n > 0.0);
        assert!(state.p_prime > 0.0);
    }

    #[test]
    fn initial_leverage_near_target() {
        let params = ModelParams::default();
        let state = SystemState::initial(&params);

        let realized = state.leverage(&params);
        let target = params.target_leverage(state.sigma_sq);

        // Should be reasonably close to target at initialization
        let ratio = realized / target;
        assert!(
            ratio > 0.5 && ratio < 2.0,
            "Initial leverage {} should be near target {}",
            realized,
            target
        );
    }

    #[test]
    fn deterministic_step_is_deterministic() {
        let params = ModelParams::deterministic_micro();
        let state = SystemState::initial(&params);

        // No noise (s=0, xi=0) - should be deterministic
        let state1 = state.step(&params, 0.0, 0.0);
        let state2 = state.step(&params, 0.0, 0.0);

        assert_eq!(state1.p, state2.p);
        assert_eq!(state1.sigma_sq, state2.sigma_sq);
        assert_eq!(state1.w_f, state2.w_f);
    }

    #[test]
    fn step_updates_lagged_price() {
        let params = ModelParams::default();
        let state = SystemState::initial(&params);

        let new_state = state.step(&params, 0.0, 0.0);

        assert_eq!(new_state.p_prime, new_state.p);
    }

    #[test]
    fn volatility_update_is_ewma() {
        let params = ModelParams::default();
        let mut state = SystemState::initial(&params);

        // Introduce a price shock
        state.p *= 1.1;

        let new_state = state.step(&params, 0.0, 0.0);

        // Volatility should increase after a price shock
        assert!(
            new_state.sigma_sq > state.sigma_sq,
            "Volatility should increase after price shock"
        );
    }

    #[test]
    fn fund_reverts_to_fundamental() {
        let params = ModelParams::default();
        let mut state = SystemState::initial(&params);

        // Price above fundamental
        state.p = params.mu * 1.5;
        state.p_prime = params.mu * 1.5;

        let new_state = state.step(&params, 0.0, 0.0);

        // Fund weight should adjust toward bringing price back to fundamental
        // The mean reversion term is τρ(μ - p) which is negative when p > μ
        assert!(
            new_state.w_f < state.w_f,
            "Fund weight should decrease when price above fundamental"
        );
    }
}
