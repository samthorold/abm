pub mod analysis;
pub mod garch;
pub mod params;
pub mod scenarios;
pub mod state;
pub mod system;

pub use params::{GarchParams, ModelParams};
pub use state::SystemState;
pub use system::LeverageCycleSystem;

/// Events in the leverage cycle simulation
#[derive(Debug, Clone)]
pub enum Event {
    /// Advance the system by one time step
    Step { step: usize },
    /// Signal end of a simulation run
    RunEnd { run_id: usize },
}

/// Classification of system stability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityClass {
    /// System converges to fixed point equilibrium
    Stable,
    /// System exhibits bounded oscillations (leverage cycles)
    LeverageCycles,
    /// System diverges (prices go to infinity or zero)
    GloballyUnstable,
}

impl std::fmt::Display for StabilityClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StabilityClass::Stable => write!(f, "Stable"),
            StabilityClass::LeverageCycles => write!(f, "Leverage Cycles"),
            StabilityClass::GloballyUnstable => write!(f, "Globally Unstable"),
        }
    }
}

/// Statistics tracked by the leverage cycle system
#[derive(Debug, Clone)]
pub struct SystemStats {
    /// Name of the scenario being run
    pub scenario_name: String,
    /// Number of simulation steps completed
    pub steps_completed: usize,
    /// Current state snapshot
    pub current_leverage: f64,
    pub current_price: f64,
    pub current_volatility: f64,
    pub current_equity: f64,
    /// History of key variables (bounded to last N values)
    pub price_history: Vec<f64>,
    pub leverage_history: Vec<f64>,
    pub equity_return_history: Vec<f64>,
    pub volatility_history: Vec<f64>,
    /// Computed metrics
    pub realized_shortfall: Option<f64>,
    pub stability_class: StabilityClass,
    /// Summary statistics
    pub price_mean: f64,
    pub price_std: f64,
    pub leverage_mean: f64,
    pub leverage_std: f64,
}

impl SystemStats {
    pub fn new(scenario_name: String) -> Self {
        SystemStats {
            scenario_name,
            steps_completed: 0,
            current_leverage: 0.0,
            current_price: 0.0,
            current_volatility: 0.0,
            current_equity: 0.0,
            price_history: Vec::new(),
            leverage_history: Vec::new(),
            equity_return_history: Vec::new(),
            volatility_history: Vec::new(),
            realized_shortfall: None,
            stability_class: StabilityClass::Stable,
            price_mean: 0.0,
            price_std: 0.0,
            leverage_mean: 0.0,
            leverage_std: 0.0,
        }
    }

    /// Check if the system appears stable (price variance low in recent history)
    pub fn is_converged(&self, threshold: f64) -> bool {
        if self.price_history.len() < 100 {
            return false;
        }
        self.price_std < threshold
    }

    /// Check if the system is oscillating (leverage range exceeds threshold)
    pub fn has_cycles(&self, min_range: f64) -> bool {
        if self.leverage_history.len() < 100 {
            return false;
        }
        let recent: Vec<f64> = self
            .leverage_history
            .iter()
            .rev()
            .take(100)
            .copied()
            .collect();
        let max = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = recent.iter().cloned().fold(f64::INFINITY, f64::min);
        (max - min) > min_range
    }
}

/// Combined stats enum for DES framework compatibility
#[derive(Debug, Clone)]
pub enum Stats {
    System(SystemStats),
}
