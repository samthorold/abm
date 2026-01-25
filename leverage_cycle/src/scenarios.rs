use des::EventLoop;

use crate::params::ModelParams;
use crate::system::LeverageCycleSystem;
use crate::{Event, Stats, SystemStats};

/// Configuration for a simulation scenario
#[derive(Debug, Clone)]
pub struct ScenarioConfig {
    /// Name of the scenario
    pub name: String,
    /// Model parameters
    pub params: ModelParams,
    /// Number of simulation steps
    pub steps: usize,
    /// Random seed for reproducibility
    pub seed: u64,
}

impl ScenarioConfig {
    /// Create scenario (i): Deterministic Microprudential
    /// Small bank, no noise - converges to fixed point equilibrium
    pub fn deterministic_micro(steps: usize, seed: u64) -> Self {
        ScenarioConfig {
            name: "Deterministic Micro (i)".to_string(),
            params: ModelParams::deterministic_micro(),
            steps,
            seed,
        }
    }

    /// Create scenario (ii): Deterministic Macroprudential
    /// Large bank, no noise - chaotic leverage cycles
    pub fn deterministic_macro(steps: usize, seed: u64) -> Self {
        ScenarioConfig {
            name: "Deterministic Macro (ii)".to_string(),
            params: ModelParams::deterministic_macro(),
            steps,
            seed,
        }
    }

    /// Create scenario (iii): Stochastic Microprudential
    /// Small bank, strong GARCH - mean-reverting random walk
    pub fn stochastic_micro(steps: usize, seed: u64) -> Self {
        ScenarioConfig {
            name: "Stochastic Micro (iii)".to_string(),
            params: ModelParams::stochastic_micro(),
            steps,
            seed,
        }
    }

    /// Create scenario (iv): Stochastic Macroprudential
    /// Large bank, weak GARCH - irregular leverage cycles
    pub fn stochastic_macro(steps: usize, seed: u64) -> Self {
        ScenarioConfig {
            name: "Stochastic Macro (iv)".to_string(),
            params: ModelParams::stochastic_macro(),
            steps,
            seed,
        }
    }

    /// Get all four scenarios from the paper's Experiment 1
    pub fn all_four(steps: usize, seed: u64) -> Vec<Self> {
        vec![
            Self::deterministic_micro(steps, seed),
            Self::deterministic_macro(steps, seed),
            Self::stochastic_micro(steps, seed),
            Self::stochastic_macro(steps, seed),
        ]
    }
}

/// Result of running a scenario
#[derive(Debug)]
pub struct ScenarioResult {
    /// Scenario configuration
    pub config: ScenarioConfig,
    /// System statistics at end of run
    pub stats: SystemStats,
}

impl ScenarioResult {
    /// Print a summary of the scenario result
    pub fn print_summary(&self) {
        println!("\n=== {} ===", self.config.name);
        println!("Steps completed: {}", self.stats.steps_completed);
        println!("Stability: {}", self.stats.stability_class);
        println!(
            "Price: mean={:.2}, std={:.4}",
            self.stats.price_mean, self.stats.price_std
        );
        println!(
            "Leverage: mean={:.2}, std={:.4}",
            self.stats.leverage_mean, self.stats.leverage_std
        );
        println!(
            "Final state: price={:.2}, leverage={:.2}, volatility={:.6}",
            self.stats.current_price, self.stats.current_leverage, self.stats.current_volatility
        );
    }
}

/// Run a single scenario and return results
pub fn run_scenario(config: ScenarioConfig) -> ScenarioResult {
    let system = LeverageCycleSystem::new(config.params.clone(), config.name.clone(), config.seed);

    let mut event_loop = EventLoop::new(vec![(0, Event::Step { step: 0 })], vec![Box::new(system)]);

    event_loop.run(config.steps);

    let stats_vec = event_loop.stats();
    let stats = match stats_vec.into_iter().next() {
        Some(Stats::System(s)) => s,
        _ => panic!("Expected SystemStats from event loop"),
    };

    ScenarioResult { config, stats }
}

/// Run all four scenarios from the paper
pub fn run_all_scenarios(steps: usize, seed: u64) -> Vec<ScenarioResult> {
    ScenarioConfig::all_four(steps, seed)
        .into_iter()
        .map(run_scenario)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_micro_is_stable() {
        let config = ScenarioConfig::deterministic_micro(500, 42);
        let result = run_scenario(config);

        // Small bank should converge
        assert!(
            result.stats.price_std < 1.0,
            "Micro should have low price variance, got {}",
            result.stats.price_std
        );
    }

    #[test]
    fn deterministic_macro_has_different_dynamics() {
        let micro = run_scenario(ScenarioConfig::deterministic_micro(500, 42));
        let macro_ = run_scenario(ScenarioConfig::deterministic_macro(500, 42));

        // Both should complete without NaN
        assert!(
            !macro_.stats.price_std.is_nan(),
            "Macro price_std should not be NaN"
        );
        assert!(
            !micro.stats.price_std.is_nan(),
            "Micro price_std should not be NaN"
        );

        // Macro should show different dynamics than micro
        // Either higher variance or different stability class
        let dynamics_differ = macro_.stats.price_std != micro.stats.price_std
            || macro_.stats.stability_class != micro.stats.stability_class
            || macro_.stats.leverage_std != micro.stats.leverage_std;

        assert!(
            dynamics_differ,
            "Macro and micro should have different dynamics"
        );
    }

    #[test]
    fn stochastic_scenarios_complete() {
        let micro = run_scenario(ScenarioConfig::stochastic_micro(200, 42));
        let macro_ = run_scenario(ScenarioConfig::stochastic_macro(200, 42));

        assert!(micro.stats.steps_completed > 0);
        assert!(macro_.stats.steps_completed > 0);
    }

    #[test]
    fn all_four_scenarios_run() {
        let results = run_all_scenarios(200, 42);

        assert_eq!(results.len(), 4);
        for result in &results {
            assert!(result.stats.steps_completed > 0);
        }
    }
}
