//! Batch Experiment Runner
//!
//! Executes multiple simulation runs based on TOML configuration files.
//! Supports parameter sweeps and Monte Carlo analysis.
//!
//! Usage:
//!   cargo run --release --bin run_experiment -- experiments/baseline_validation.toml

use des::EventLoop;
use insurance_cycles::insurer::Insurer;
use insurance_cycles::market_coordinator::MarketCoordinator;
use insurance_cycles::output::SimulationOutput;
use insurance_cycles::{Customer, Event, ModelConfig, Stats, DAYS_PER_YEAR};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Top-level experiment configuration
#[derive(Debug, Clone, Deserialize)]
struct ExperimentConfig {
    experiment: ExperimentMetadata,
    model: ModelParams,
    output: OutputSettings,
    sweep: Option<SweepConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExperimentMetadata {
    name: String,
    description: String,
    num_runs: usize,
    num_years: usize,
    #[allow(dead_code)]
    warmup_years: usize, // Reserved for future warmup period implementation
    base_seed: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelParams {
    risk_loading_factor: f64,
    underwriter_smoothing: Option<f64>,
    distance_cost: Option<f64>,
    credibility_factor: Option<f64>,
    ewma_smoothing: f64,
    claim_frequency: f64,
    gamma_mean: f64,
    gamma_std: f64,
    num_insurers: usize,
    num_customers: usize,
    initial_capital: f64,
    leverage_ratio: Option<f64>,
}

impl ModelParams {
    fn to_model_config(&self) -> ModelConfig {
        ModelConfig {
            risk_loading_factor: self.risk_loading_factor,
            underwriter_smoothing: self.underwriter_smoothing.unwrap_or(0.3),
            distance_cost: self.distance_cost.unwrap_or(0.08),
            credibility_factor: self.credibility_factor.unwrap_or(0.2),
            ewma_smoothing: self.ewma_smoothing,
            claim_frequency: self.claim_frequency,
            gamma_mean: self.gamma_mean,
            gamma_std: self.gamma_std,
            num_insurers: self.num_insurers,
            num_customers: self.num_customers,
            initial_capital: self.initial_capital,
            leverage_ratio: self.leverage_ratio.unwrap_or(2.0),
            allocation_noise: 0.05, // Default ±5% noise
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OutputSettings {
    save_market_timeseries: bool,
    save_insurer_snapshots: bool,
    save_summary_stats: bool,
    /// Retained for TOML backward compatibility but not used (insurer snapshots are final-year only)
    #[allow(dead_code)]
    snapshot_interval: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct SweepConfig {
    parameter: String,
    values: Vec<f64>,
}

/// Aggregate statistics across multiple runs
#[derive(Debug, Clone, Serialize)]
struct AggregateMetrics {
    num_runs: usize,
    successful_runs: usize,
    cycle_detection_rate: f64,
    mean_loss_ratio: MeanStd,
    cycle_period: MeanStd,
    dominant_frequency: MeanStd,
    std_loss_ratio: MeanStd,
    ar2_a1: MeanStd,
    ar2_a2: MeanStd,
    cycle_conditions_met_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
struct MeanStd {
    mean: f64,
    std: f64,
    min: f64,
    max: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <experiment_config.toml>", args[0]);
        eprintln!("Example: {} experiments/baseline_validation.toml", args[0]);
        std::process::exit(1);
    }

    let config_path = &args[1];
    println!("=== Insurance Cycles Experiment Runner ===\n");
    println!("Loading experiment config: {}\n", config_path);

    // Load configuration
    let config_str = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("Error reading config file: {}", e);
        std::process::exit(1);
    });

    let exp_config: ExperimentConfig = toml::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("Error parsing TOML config: {}", e);
        std::process::exit(1);
    });

    println!("Experiment: {}", exp_config.experiment.name);
    println!("Description: {}", exp_config.experiment.description);
    println!(
        "Configuration: {} runs × {} years\n",
        exp_config.experiment.num_runs, exp_config.experiment.num_years
    );

    // Determine output directory
    let output_base = PathBuf::from("results").join(&exp_config.experiment.name);
    fs::create_dir_all(&output_base).unwrap_or_else(|e| {
        eprintln!("Error creating output directory: {}", e);
        std::process::exit(1);
    });

    // Handle parameter sweep or simple runs
    if let Some(sweep) = &exp_config.sweep {
        run_parameter_sweep(&exp_config, sweep, &output_base);
    } else {
        run_simple_experiment(&exp_config, &output_base);
    }
}

/// Run simple experiment (no parameter sweep)
fn run_simple_experiment(exp_config: &ExperimentConfig, output_dir: &Path) {
    let start_time = Instant::now();
    let total_runs = exp_config.experiment.num_runs;

    println!("Running {} Monte Carlo simulations...\n", total_runs);

    let mut all_outputs = Vec::new();

    for run_idx in 0..total_runs {
        let seed = exp_config.experiment.base_seed + run_idx as u64;
        let model_config = exp_config.model.to_model_config();

        print!("Run {}/{} (seed={})... ", run_idx + 1, total_runs, seed);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let run_start = Instant::now();

        // Run simulation
        let stats = run_simulation(&model_config, exp_config.experiment.num_years, seed);

        // Convert to output format
        let output = SimulationOutput::from_stats(
            stats,
            &model_config,
            seed,
            exp_config.experiment.num_years,
        );

        // Save individual run
        if exp_config.output.save_summary_stats
            || exp_config.output.save_market_timeseries
            || exp_config.output.save_insurer_snapshots
        {
            let run_dir = output_dir.join(format!("run_{}", seed));
            save_run_output(&output, &run_dir, &exp_config.output);
        }

        let elapsed = run_start.elapsed();
        println!(
            "✓ ({:.1}s) cycles={} period={:.1}yr",
            elapsed.as_secs_f64(),
            output.cycle_metrics.has_cycles,
            output
                .cycle_metrics
                .cycle_period
                .map(|p| format!("{:.1}", p))
                .unwrap_or_else(|| "N/A".to_string())
        );

        all_outputs.push(output);
    }

    // Aggregate statistics
    println!("\n=== Aggregating Results ===\n");
    let aggregate = compute_aggregate_metrics(&all_outputs);

    // Save aggregate summary
    let aggregate_json = serde_json::to_string_pretty(&aggregate).unwrap();
    fs::write(output_dir.join("aggregate_summary.json"), aggregate_json).unwrap();

    print_aggregate_summary(&aggregate);

    let total_elapsed = start_time.elapsed();
    println!(
        "\n✓ Experiment complete in {:.1}s ({:.1}s per run)",
        total_elapsed.as_secs_f64(),
        total_elapsed.as_secs_f64() / total_runs as f64
    );
    println!("Results saved to: {}", output_dir.display());
}

/// Run parameter sweep experiment
fn run_parameter_sweep(exp_config: &ExperimentConfig, sweep: &SweepConfig, output_dir: &Path) {
    let start_time = Instant::now();
    let total_combinations = sweep.values.len() * exp_config.experiment.num_runs;

    println!("Parameter sweep: {} ∈ {:?}", sweep.parameter, sweep.values);
    println!(
        "Total simulations: {} parameter values × {} runs = {}\n",
        sweep.values.len(),
        exp_config.experiment.num_runs,
        total_combinations
    );

    let mut sweep_results: HashMap<String, Vec<SimulationOutput>> = HashMap::new();

    for (param_idx, &param_value) in sweep.values.iter().enumerate() {
        println!(
            "\n--- {}={:.3} ({}/{}) ---\n",
            sweep.parameter,
            param_value,
            param_idx + 1,
            sweep.values.len()
        );

        let param_key = format!("{}_{:.3}", sweep.parameter, param_value);
        let param_dir = output_dir.join(&param_key);

        // Run simulations in parallel
        use rayon::prelude::*;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let completed = Arc::new(AtomicUsize::new(0));

        let param_outputs: Vec<SimulationOutput> = (0..exp_config.experiment.num_runs)
            .into_par_iter()
            .map(|run_idx| {
                let seed = exp_config.experiment.base_seed
                    + (param_idx * exp_config.experiment.num_runs + run_idx) as u64;

                // Create model config with swept parameter
                let mut model_config = exp_config.model.to_model_config();
                apply_parameter_value(&mut model_config, &sweep.parameter, param_value);

                let run_start = Instant::now();

                // Run simulation
                let stats = run_simulation(&model_config, exp_config.experiment.num_years, seed);

                // Convert to output
                let output = SimulationOutput::from_stats(
                    stats,
                    &model_config,
                    seed,
                    exp_config.experiment.num_years,
                );

                // Save individual run
                if exp_config.output.save_summary_stats {
                    let run_dir = param_dir.join(format!("run_{}", seed));
                    save_run_output(&output, &run_dir, &exp_config.output);
                }

                let elapsed = run_start.elapsed();
                let count = completed.fetch_add(1, Ordering::SeqCst) + 1;
                println!(
                    "  Run {}/{} (seed={}) ✓ ({:.1}s) cycles={} period={}",
                    count,
                    exp_config.experiment.num_runs,
                    seed,
                    elapsed.as_secs_f64(),
                    output.cycle_metrics.has_cycles,
                    output
                        .cycle_metrics
                        .cycle_period
                        .map(|p| format!("{:.1}yr", p))
                        .unwrap_or_else(|| "N/A".to_string())
                );

                output
            })
            .collect();

        // Aggregate stats for this parameter value
        let aggregate = compute_aggregate_metrics(&param_outputs);
        let aggregate_json = serde_json::to_string_pretty(&aggregate).unwrap();
        fs::create_dir_all(&param_dir).unwrap();
        fs::write(param_dir.join("aggregate_summary.json"), aggregate_json).unwrap();

        println!(
            "  → Cycles detected: {:.0}%, Mean period: {:.1}yr",
            aggregate.cycle_detection_rate * 100.0,
            aggregate.cycle_period.mean
        );

        sweep_results.insert(param_key, param_outputs);
    }

    // Save sweep-level summary
    let sweep_summary_path = output_dir.join("sweep_summary.json");
    let sweep_aggregates: HashMap<String, AggregateMetrics> = sweep_results
        .iter()
        .map(|(key, outputs)| (key.clone(), compute_aggregate_metrics(outputs)))
        .collect();
    let sweep_json = serde_json::to_string_pretty(&sweep_aggregates).unwrap();
    fs::write(sweep_summary_path, sweep_json).unwrap();

    let total_elapsed = start_time.elapsed();
    println!(
        "\n✓ Parameter sweep complete in {:.1}s ({:.1}s per run)",
        total_elapsed.as_secs_f64(),
        total_elapsed.as_secs_f64() / total_combinations as f64
    );
    println!("Results saved to: {}", output_dir.display());
}

/// Run a single simulation
fn run_simulation(config: &ModelConfig, num_years: usize, seed: u64) -> Vec<Stats> {
    let mut setup_rng = StdRng::seed_from_u64(seed);

    // Create customers
    let customers: Vec<Customer> = (0..config.num_customers)
        .map(|i| {
            let position = setup_rng.gen_range(0.0..(2.0 * PI));
            Customer::new(i, position)
        })
        .collect();

    // Create insurers
    let insurer_positions: HashMap<usize, f64> = (0..config.num_insurers)
        .map(|i| {
            let position = setup_rng.gen_range(0.0..(2.0 * PI));
            (i, position)
        })
        .collect();

    // Create agents
    let mut agents: Vec<Box<dyn des::Agent<Event, Stats>>> = Vec::new();

    for (insurer_id, &position) in &insurer_positions {
        let insurer = Insurer::new(
            *insurer_id,
            position,
            config.clone(),
            seed + (*insurer_id as u64),
        );
        agents.push(Box::new(insurer));
    }

    let coordinator = MarketCoordinator::new(
        config.clone(),
        customers,
        insurer_positions,
        seed + 1000,
        seed + 2000,
    );
    agents.push(Box::new(coordinator));

    // Schedule events
    let mut initial_events = Vec::new();
    for year in 1..=num_years {
        let time = year * DAYS_PER_YEAR;
        initial_events.push((time, Event::YearStart { year }));
    }

    // Run simulation
    let mut event_loop: EventLoop<Event, Stats> = EventLoop::new(initial_events, agents);
    let max_time = (num_years + 1) * DAYS_PER_YEAR;
    event_loop.run(max_time);

    event_loop.stats()
}

/// Apply parameter value to model config
fn apply_parameter_value(config: &mut ModelConfig, param_name: &str, value: f64) {
    match param_name {
        "underwriter_smoothing" => config.underwriter_smoothing = value,
        "credibility_factor" => config.credibility_factor = value,
        "distance_cost" => config.distance_cost = value,
        "leverage_ratio" => config.leverage_ratio = value,
        _ => panic!("Unknown parameter: {}", param_name),
    }
}

/// Save run output based on settings
fn save_run_output(output: &SimulationOutput, run_dir: &Path, settings: &OutputSettings) {
    fs::create_dir_all(run_dir).unwrap();

    if settings.save_market_timeseries {
        output
            .write_market_csv(run_dir.join("market_timeseries.csv"))
            .unwrap();
    }

    if settings.save_insurer_snapshots {
        output
            .write_insurer_csv(run_dir.join("insurer_snapshots.csv"))
            .unwrap();
    }

    if settings.save_summary_stats {
        output
            .write_summary_json(run_dir.join("summary.json"))
            .unwrap();
    }
}

/// Compute aggregate metrics across runs
fn compute_aggregate_metrics(outputs: &[SimulationOutput]) -> AggregateMetrics {
    let successful_runs = outputs.len();

    let cycles_detected = outputs
        .iter()
        .filter(|o| o.cycle_metrics.has_cycles)
        .count();

    let mean_loss_ratios: Vec<f64> = outputs
        .iter()
        .map(|o| o.cycle_metrics.mean_loss_ratio)
        .collect();

    let cycle_periods: Vec<f64> = outputs
        .iter()
        .filter_map(|o| o.cycle_metrics.cycle_period)
        .collect();

    let dominant_freqs: Vec<f64> = outputs
        .iter()
        .filter_map(|o| o.cycle_metrics.dominant_frequency)
        .collect();

    let std_loss_ratios: Vec<f64> = outputs
        .iter()
        .map(|o| o.cycle_metrics.std_loss_ratio)
        .collect();

    let ar2_a1s: Vec<f64> = outputs
        .iter()
        .filter_map(|o| o.cycle_metrics.ar2_coefficients.map(|(_, a1, _)| a1))
        .collect();

    let ar2_a2s: Vec<f64> = outputs
        .iter()
        .filter_map(|o| o.cycle_metrics.ar2_coefficients.map(|(_, _, a2)| a2))
        .collect();

    let conditions_met = outputs
        .iter()
        .filter(|o| o.cycle_metrics.meets_cycle_conditions == Some(true))
        .count();

    AggregateMetrics {
        num_runs: outputs.len(),
        successful_runs,
        cycle_detection_rate: cycles_detected as f64 / successful_runs as f64,
        mean_loss_ratio: compute_mean_std(&mean_loss_ratios),
        cycle_period: compute_mean_std(&cycle_periods),
        dominant_frequency: compute_mean_std(&dominant_freqs),
        std_loss_ratio: compute_mean_std(&std_loss_ratios),
        ar2_a1: compute_mean_std(&ar2_a1s),
        ar2_a2: compute_mean_std(&ar2_a2s),
        cycle_conditions_met_rate: conditions_met as f64 / successful_runs as f64,
    }
}

fn compute_mean_std(values: &[f64]) -> MeanStd {
    if values.is_empty() {
        return MeanStd {
            mean: 0.0,
            std: 0.0,
            min: 0.0,
            max: 0.0,
        };
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std = variance.sqrt();
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    MeanStd {
        mean,
        std,
        min,
        max,
    }
}

fn print_aggregate_summary(agg: &AggregateMetrics) {
    println!("Aggregate Results ({} runs):", agg.num_runs);
    println!(
        "  Cycle detection rate: {:.1}%",
        agg.cycle_detection_rate * 100.0
    );
    println!(
        "  Mean loss ratio: {:.3} ± {:.3}",
        agg.mean_loss_ratio.mean, agg.mean_loss_ratio.std
    );
    println!(
        "  Cycle period: {:.2} ± {:.2} years (n={})",
        agg.cycle_period.mean,
        agg.cycle_period.std,
        (agg.cycle_detection_rate * agg.num_runs as f64) as usize
    );
    println!(
        "  AR(2) coefficients: a1={:.3}±{:.3}, a2={:.3}±{:.3}",
        agg.ar2_a1.mean, agg.ar2_a1.std, agg.ar2_a2.mean, agg.ar2_a2.std
    );
    println!(
        "  Cycle conditions met: {:.1}%",
        agg.cycle_conditions_met_rate * 100.0
    );
}
