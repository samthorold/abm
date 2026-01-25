use des::{Agent, Response};
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::garch::GarchProcess;
use crate::params::ModelParams;
use crate::state::SystemState;
use crate::{Event, StabilityClass, Stats, SystemStats};

/// Maximum history length to prevent unbounded memory growth
const MAX_HISTORY_LEN: usize = 10_000;

/// The main agent that runs the Basel leverage cycle simulation
///
/// This implements the coupled 6-variable dynamical system where Bank and Fund
/// follow deterministic update rules each timestep. A single agent is more
/// faithful to the paper's mathematical structure than fragmenting into multiple agents.
pub struct LeverageCycleSystem {
    /// Model parameters
    params: ModelParams,
    /// Current system state (6 variables)
    state: SystemState,
    /// GARCH process for exogenous noise
    garch: GarchProcess,
    /// Random number generator
    rng: StdRng,
    /// Scenario name for reporting
    scenario_name: String,
    /// History tracking
    price_history: Vec<f64>,
    leverage_history: Vec<f64>,
    equity_history: Vec<f64>,
    volatility_history: Vec<f64>,
    /// Simulation step counter
    step_count: usize,
}

impl LeverageCycleSystem {
    /// Create a new system with the given parameters and seed
    pub fn new(params: ModelParams, scenario_name: String, seed: u64) -> Self {
        let state = SystemState::initial(&params);
        let garch = GarchProcess::new(params.garch.clone());
        let rng = StdRng::seed_from_u64(seed);

        let initial_price = state.p;
        let initial_leverage = state.leverage(&params);
        let initial_equity = state.bank_equity(&params);
        let initial_volatility = state.sigma_sq.sqrt();

        LeverageCycleSystem {
            params,
            state,
            garch,
            rng,
            scenario_name,
            price_history: vec![initial_price],
            leverage_history: vec![initial_leverage],
            equity_history: vec![initial_equity],
            volatility_history: vec![initial_volatility],
            step_count: 0,
        }
    }

    /// Advance the system by one time step
    fn step(&mut self) {
        // Generate GARCH noise
        let (s, xi) = self.garch.next(&mut self.rng);

        // Update state using the 6 coupled equations
        let new_state = self.state.step(&self.params, s, xi);

        // Record history
        self.record_state(&new_state);

        // Update state
        self.state = new_state;
        self.step_count += 1;
    }

    /// Record state to history, maintaining bounded size
    fn record_state(&mut self, state: &SystemState) {
        // Compute derived quantities
        let price = state.p;
        let leverage = state.leverage(&self.params);
        let equity = state.bank_equity(&self.params);
        let volatility = state.sigma_sq.sqrt();

        // Add to history
        self.price_history.push(price);
        self.leverage_history.push(leverage);
        self.equity_history.push(equity);
        self.volatility_history.push(volatility);

        // Bound history length
        if self.price_history.len() > MAX_HISTORY_LEN {
            self.price_history.remove(0);
            self.leverage_history.remove(0);
            self.equity_history.remove(0);
            self.volatility_history.remove(0);
        }
    }

    /// Compute equity returns from history
    fn compute_equity_returns(&self) -> Vec<f64> {
        if self.equity_history.len() < 2 {
            return Vec::new();
        }

        self.equity_history
            .windows(2)
            .map(|w| {
                let prev = w[0];
                let curr = w[1];
                if prev.abs() < 1e-15 {
                    0.0
                } else {
                    (curr / prev).ln()
                }
            })
            .collect()
    }

    /// Classify stability based on simulation history
    fn classify_stability(&self) -> StabilityClass {
        if self.price_history.len() < 100 {
            return StabilityClass::Stable;
        }

        let recent_prices: Vec<f64> = self.price_history.iter().rev().take(100).copied().collect();

        // Check for divergence (price going to extremes)
        let max_price = recent_prices
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_price = recent_prices.iter().cloned().fold(f64::INFINITY, f64::min);

        if max_price > 1000.0 * self.params.mu || min_price < 0.001 * self.params.mu {
            return StabilityClass::GloballyUnstable;
        }

        // Check for oscillations in leverage
        let recent_leverage: Vec<f64> = self
            .leverage_history
            .iter()
            .rev()
            .take(100)
            .copied()
            .collect();

        let max_lev = recent_leverage
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let min_lev = recent_leverage
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let leverage_range = max_lev - min_lev;

        // Check for convergence (low variance in prices)
        let price_mean: f64 = recent_prices.iter().sum::<f64>() / recent_prices.len() as f64;
        let price_variance: f64 = recent_prices
            .iter()
            .map(|p| (p - price_mean).powi(2))
            .sum::<f64>()
            / recent_prices.len() as f64;

        // If price variance is very low, system is stable
        if price_variance < 0.01 {
            return StabilityClass::Stable;
        }

        // If leverage has significant oscillation, classify as cycles
        if leverage_range > 1.0 {
            return StabilityClass::LeverageCycles;
        }

        StabilityClass::Stable
    }

    /// Compute summary statistics
    fn compute_summary_stats(&self) -> (f64, f64, f64, f64) {
        let n = self.price_history.len() as f64;
        if n < 1.0 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let price_mean = self.price_history.iter().sum::<f64>() / n;
        let price_std = (self
            .price_history
            .iter()
            .map(|p| (p - price_mean).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();

        let leverage_mean = self.leverage_history.iter().sum::<f64>() / n;
        let leverage_std = (self
            .leverage_history
            .iter()
            .map(|l| (l - leverage_mean).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();

        (price_mean, price_std, leverage_mean, leverage_std)
    }

    /// Get current state for external inspection
    pub fn state(&self) -> &SystemState {
        &self.state
    }

    /// Get parameters for external inspection
    pub fn params(&self) -> &ModelParams {
        &self.params
    }
}

impl Agent<Event, Stats> for LeverageCycleSystem {
    fn act(&mut self, _current_t: usize, data: &Event) -> Response<Event, Stats> {
        match data {
            Event::Step { step } => {
                self.step();

                // Schedule next step
                Response::event(*step + 1, Event::Step { step: step + 1 })
            }
            Event::RunEnd { .. } => Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        let equity_returns = self.compute_equity_returns();
        let (price_mean, price_std, leverage_mean, leverage_std) = self.compute_summary_stats();

        let stats = SystemStats {
            scenario_name: self.scenario_name.clone(),
            steps_completed: self.step_count,
            current_leverage: self.state.leverage(&self.params),
            current_price: self.state.p,
            current_volatility: self.state.sigma_sq.sqrt(),
            current_equity: self.state.bank_equity(&self.params),
            price_history: self.price_history.clone(),
            leverage_history: self.leverage_history.clone(),
            equity_return_history: equity_returns,
            volatility_history: self.volatility_history.clone(),
            realized_shortfall: None, // Computed by analysis module
            stability_class: self.classify_stability(),
            price_mean,
            price_std,
            leverage_mean,
            leverage_std,
        };

        Stats::System(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use des::EventLoop;

    #[test]
    fn system_advances_on_step_event() {
        let params = ModelParams::deterministic_micro();
        let mut system = LeverageCycleSystem::new(params, "test".to_string(), 42);

        let initial_step = system.step_count;
        system.act(0, &Event::Step { step: 0 });

        assert_eq!(system.step_count, initial_step + 1);
    }

    #[test]
    fn history_is_bounded() {
        let params = ModelParams::deterministic_micro();
        let mut system = LeverageCycleSystem::new(params, "test".to_string(), 42);

        // Run many steps
        for i in 0..MAX_HISTORY_LEN + 1000 {
            system.act(i, &Event::Step { step: i });
        }

        assert!(system.price_history.len() <= MAX_HISTORY_LEN);
        assert!(system.leverage_history.len() <= MAX_HISTORY_LEN);
    }

    #[test]
    fn deterministic_micro_converges() {
        let params = ModelParams::deterministic_micro();
        let system = LeverageCycleSystem::new(params, "micro".to_string(), 42);

        let mut event_loop =
            EventLoop::new(vec![(0, Event::Step { step: 0 })], vec![Box::new(system)]);

        event_loop.run(1000);

        let stats = event_loop.stats();
        assert_eq!(stats.len(), 1);

        let Stats::System(sys_stats) = &stats[0];
        // Micro should be stable
        assert!(
            sys_stats.price_std < 1.0,
            "Deterministic micro should have low price variance, got {}",
            sys_stats.price_std
        );
    }

    #[test]
    fn deterministic_with_same_seed_is_reproducible() {
        let params = ModelParams::deterministic_micro();

        let mut system1 = LeverageCycleSystem::new(params.clone(), "test1".to_string(), 42);
        let mut system2 = LeverageCycleSystem::new(params, "test2".to_string(), 42);

        for i in 0..100 {
            system1.act(i, &Event::Step { step: i });
            system2.act(i, &Event::Step { step: i });
        }

        assert_eq!(system1.state.p, system2.state.p);
        assert_eq!(system1.state.sigma_sq, system2.state.sigma_sq);
    }

    #[test]
    fn stochastic_with_same_seed_is_reproducible() {
        let params = ModelParams::stochastic_micro();

        let mut system1 = LeverageCycleSystem::new(params.clone(), "test1".to_string(), 42);
        let mut system2 = LeverageCycleSystem::new(params, "test2".to_string(), 42);

        for i in 0..100 {
            system1.act(i, &Event::Step { step: i });
            system2.act(i, &Event::Step { step: i });
        }

        assert_eq!(system1.state.p, system2.state.p);
        assert_eq!(system1.state.sigma_sq, system2.state.sigma_sq);
    }
}
