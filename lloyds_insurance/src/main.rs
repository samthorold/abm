use des::EventLoop;
use lloyds_insurance::{
    AttritionalLossGenerator, BrokerPool, CatastropheLossGenerator, CentralRiskRepository, Event,
    MarketStatisticsCollector, ModelConfig, Stats, Syndicate, SyndicateTimeSeriesStats,
    TimeGenerator, TimeSeriesStats,
};
use std::env;
use std::fs::File;
use std::io::Write;

fn main() {
    // Parse command-line arguments for experiment selection
    let args: Vec<String> = env::args().collect();
    let experiment = if args.len() > 1 {
        args[1].as_str()
    } else {
        "demo" // Default to demo mode (single run of Scenario 1)
    };

    match experiment {
        "exp1" => run_experiment_1(),
        "exp2" => run_experiment_2(),
        "exp3" => run_experiment_3(),
        "exp4" => run_experiment_4(),
        "exp5" => run_experiment_5(),
        "exp6" => run_experiment_6(),
        "exp7" => run_experiment_7(),
        "all" => run_all_experiments(),
        "demo" => run_demo(),
        _ => run_demo(),
    }
}

/// Run Experiment 1: Fair Price Convergence
/// Scenario 1 (attritional only), 10 replications
fn run_experiment_1() {
    println!("==============================================");
    println!("Experiment 1: Fair Price Convergence");
    println!("==============================================\n");

    for seed in 0..10 {
        println!("Running replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_1();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp1_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp1_rep{}", seed));
    }

    println!("\nExperiment 1 complete. Run Python analysis: python analysis/experiment_1.py");
}

/// Run Experiment 2: Catastrophe-Driven Cycles
/// Scenario 2 (with catastrophes), 10 replications
fn run_experiment_2() {
    println!("==============================================");
    println!("Experiment 2: Catastrophe-Driven Cycles");
    println!("==============================================\n");

    for seed in 0..10 {
        println!("Running replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_2();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp2_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp2_rep{}", seed));
    }

    println!("\nExperiment 2 complete. Run Python analysis: python analysis/experiment_2.py");
}

/// Run Experiment 3: VaR Exposure Management Effectiveness
/// Scenarios 2 vs 3 (without/with VaR EM), 10 replications each
fn run_experiment_3() {
    println!("==============================================");
    println!("Experiment 3: VaR Exposure Management");
    println!("==============================================\n");

    println!("Running Scenario 2 (no VaR EM)...");
    for seed in 0..10 {
        println!("  Replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_2();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp3_scenario2_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp3_scenario2_rep{}", seed));
    }

    println!("Running Scenario 3 (with VaR EM)...");
    for seed in 0..10 {
        println!("  Replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_3();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp3_scenario3_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp3_scenario3_rep{}", seed));
    }

    println!("\nExperiment 3 complete. Run Python analysis: python analysis/experiment_3.py");
}

/// Run Experiment 4: Lead-Follow Syndication Stability
/// Modified Scenario 1 (independent) vs Scenario 4 (syndicated), 10 replications each
fn run_experiment_4() {
    println!("==============================================");
    println!("Experiment 4: Lead-Follow Syndication");
    println!("==============================================\n");

    println!("Running independent syndicates (follow_top_k=0)...");
    for seed in 0..10 {
        println!("  Replication {} of 10...", seed + 1);
        let mut config = ModelConfig::scenario_1();
        config.follow_top_k = 0; // Disable followers
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp4_independent_rep{}", seed));
        export_syndicate_time_series_csv(
            &syndicate_series,
            &format!("exp4_independent_rep{}", seed),
        );
    }

    println!("Running syndicated (follow_top_k=5)...");
    for seed in 0..10 {
        println!("  Replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_4();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp4_syndicated_rep{}", seed));
        export_syndicate_time_series_csv(
            &syndicate_series,
            &format!("exp4_syndicated_rep{}", seed),
        );
    }

    println!("\nExperiment 4 complete. Run Python analysis: python analysis/experiment_4.py");
}

/// Run Experiment 5: Loss Ratio Equilibrium
/// All 4 scenarios, 10 replications each
fn run_experiment_5() {
    println!("==============================================");
    println!("Experiment 5: Loss Ratio Equilibrium");
    println!("==============================================\n");

    for scenario_num in 1..=4 {
        println!("Running Scenario {}...", scenario_num);
        for seed in 0..10 {
            println!("  Replication {} of 10...", seed + 1);
            let config = match scenario_num {
                1 => ModelConfig::scenario_1(),
                2 => ModelConfig::scenario_2(),
                3 => ModelConfig::scenario_3(),
                4 => ModelConfig::scenario_4(),
                _ => unreachable!(),
            };
            let (time_series, syndicate_series) = run_simulation(config, seed, 50);
            export_time_series_csv(
                &time_series,
                &format!("exp5_scenario{}_rep{}", scenario_num, seed),
            );
            export_syndicate_time_series_csv(
                &syndicate_series,
                &format!("exp5_scenario{}_rep{}", scenario_num, seed),
            );
        }
    }

    println!("\nExperiment 5 complete. Run Python analysis: python analysis/experiment_5.py");
}

/// Run Experiment 6: Markup Mechanism Validation
/// Scenario 1, 10 replications
fn run_experiment_6() {
    println!("==============================================");
    println!("Experiment 6: Markup Mechanism");
    println!("==============================================\n");

    for seed in 0..10 {
        println!("Running replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_1();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp6_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp6_rep{}", seed));
    }

    println!("\nExperiment 6 complete. Run Python analysis: python analysis/experiment_6.py");
}

/// Run Experiment 7: Loss Coupling in Syndicated Risks
/// Scenario 4 (with followers), 10 replications
fn run_experiment_7() {
    println!("==============================================");
    println!("Experiment 7: Loss Coupling");
    println!("==============================================\n");

    for seed in 0..10 {
        println!("Running replication {} of 10...", seed + 1);
        let config = ModelConfig::scenario_4();
        let (time_series, syndicate_series) = run_simulation(config, seed, 50);
        export_time_series_csv(&time_series, &format!("exp7_rep{}", seed));
        export_syndicate_time_series_csv(&syndicate_series, &format!("exp7_rep{}", seed));
    }

    println!("\nExperiment 7 complete. Run Python analysis: python analysis/experiment_7.py");
}

/// Run all experiments sequentially
fn run_all_experiments() {
    run_experiment_1();
    run_experiment_2();
    run_experiment_3();
    run_experiment_4();
    run_experiment_5();
    run_experiment_6();
    run_experiment_7();
    println!("\n==============================================");
    println!("All experiments complete!");
    println!("==============================================");
}

/// Run a single demonstration simulation (Scenario 1)
fn run_demo() {
    println!("Lloyd's of London Insurance Market Simulation");
    println!("==============================================\n");
    println!("Running demo simulation (Scenario 1, seed=12345)...\n");

    let config = ModelConfig::scenario_1();
    let (time_series, _syndicate_series) = run_simulation(config, 12345, 50);

    export_time_series_csv(&time_series, "demo");

    println!("\n==============================================");
    println!("Demo simulation complete!");
    println!("Time series exported to: lloyds_insurance/demo_time_series.csv");
    println!("==============================================");
}

/// Core simulation runner - returns time series stats
fn run_simulation(
    config: ModelConfig,
    seed: u64,
    sim_years: usize,
) -> (TimeSeriesStats, SyndicateTimeSeriesStats) {
    // Create initial events
    let events = vec![(0, Event::Day)];

    // Create agents
    let mut agents: Vec<Box<dyn des::Agent<Event, Stats>>> = Vec::new();

    // Add Time generator
    agents.push(Box::new(TimeGenerator::new()));

    // Add 5 syndicates
    for i in 0..5 {
        agents.push(Box::new(Syndicate::new(i, config.clone())));
    }

    // Add broker pool (manages 25 brokers internally, 1:5 ratio)
    agents.push(Box::new(BrokerPool::new(
        25,
        config.clone(),
        seed + 100, // Offset seed for brokers
    )));

    // Add central risk repository (handles syndicate selection and risk tracking)
    agents.push(Box::new(CentralRiskRepository::new(
        config.clone(),
        5,
        seed + 200, // Offset seed for repository
    )));

    // Add attritional loss generator
    agents.push(Box::new(AttritionalLossGenerator::new(
        config.clone(),
        seed + 300, // Offset seed for attritional losses
    )));

    // Add catastrophe loss generator
    agents.push(Box::new(CatastropheLossGenerator::new(
        config.clone(),
        sim_years,
        seed + 400, // Offset seed for catastrophes
    )));

    // Add market statistics collector
    agents.push(Box::new(MarketStatisticsCollector::new(5)));

    // Create event loop and run simulation
    let mut event_loop = EventLoop::new(events, agents);
    let sim_end = 365 * sim_years;
    event_loop.run(sim_end);

    // Extract stats
    let stats = event_loop.stats();

    // Extract combined market stats (both market-level and syndicate-level time series)
    let combined_stats = stats
        .iter()
        .find_map(|s| match s {
            Stats::CombinedMarketStats(cs) => Some(cs.clone()),
            _ => None,
        })
        .expect("Combined market stats should exist");

    (
        combined_stats.market_series,
        combined_stats.syndicate_series,
    )
}

/// Export market-level time series to CSV
fn export_time_series_csv(time_series: &TimeSeriesStats, filename_prefix: &str) {
    let output_path = format!("lloyds_insurance/{}_time_series.csv", filename_prefix);

    let mut file = match File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: Could not create time series CSV: {}", e);
            return;
        }
    };

    // Write header with new fields
    if let Err(e) = writeln!(
        file,
        "year,day,avg_premium,avg_loss_ratio,num_solvent_syndicates,num_insolvent_syndicates,total_capital,total_policies,premium_std_dev,markup_avg,markup_std_dev,cat_event_occurred,cat_event_loss,avg_uniform_deviation"
    ) {
        eprintln!("Warning: Could not write CSV header: {}", e);
        return;
    }

    // Write data for each snapshot
    for snapshot in &time_series.snapshots {
        if let Err(e) = writeln!(
            file,
            "{},{},{:.2},{:.4},{},{},{:.2},{},{:.2},{:.4},{:.4},{},{:.2},{:.4}",
            snapshot.year,
            snapshot.day,
            snapshot.avg_premium,
            snapshot.avg_loss_ratio,
            snapshot.num_solvent_syndicates,
            snapshot.num_insolvent_syndicates,
            snapshot.total_capital,
            snapshot.total_policies,
            snapshot.premium_std_dev,
            snapshot.markup_avg,
            snapshot.markup_std_dev,
            if snapshot.cat_event_occurred { 1 } else { 0 },
            snapshot.cat_event_loss,
            snapshot.avg_uniform_deviation,
        ) {
            eprintln!("Warning: Could not write CSV row: {}", e);
            return;
        }
    }

    println!(
        "  Exported: {} ({} rows)",
        output_path,
        time_series.snapshots.len()
    );
}

/// Export per-syndicate time series to CSV
fn export_syndicate_time_series_csv(
    syndicate_series: &SyndicateTimeSeriesStats,
    filename_prefix: &str,
) {
    if syndicate_series.snapshots.is_empty() {
        // Skip export if no data (placeholder implementation)
        return;
    }

    let output_path = format!(
        "lloyds_insurance/{}_syndicate_time_series.csv",
        filename_prefix
    );

    let mut file = match File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: Could not create syndicate time series CSV: {}", e);
            return;
        }
    };

    // Write header
    if let Err(e) = writeln!(
        file,
        "year,syndicate_id,capital,markup_m_t,loss_ratio,num_policies,annual_premiums,annual_claims"
    ) {
        eprintln!("Warning: Could not write CSV header: {}", e);
        return;
    }

    // Write data for each snapshot
    for snapshot in &syndicate_series.snapshots {
        if let Err(e) = writeln!(
            file,
            "{},{},{:.2},{:.4},{:.4},{},{:.2},{:.2}",
            snapshot.year,
            snapshot.syndicate_id,
            snapshot.capital,
            snapshot.markup_m_t,
            snapshot.loss_ratio,
            snapshot.num_policies,
            snapshot.annual_premiums,
            snapshot.annual_claims,
        ) {
            eprintln!("Warning: Could not write CSV row: {}", e);
            return;
        }
    }

    println!(
        "  Exported: {} ({} rows)",
        output_path,
        syndicate_series.snapshots.len()
    );
}
