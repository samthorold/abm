use des::EventLoop;
use lloyds_insurance::{
    AttritionalLossGenerator, BrokerPool, CatastropheLossGenerator, CentralRiskRepository, Event,
    MarketStatisticsCollector, ModelConfig, Stats, Syndicate, TimeGenerator,
};
use std::fs::File;
use std::io::Write as IoWrite;

fn main() {
    println!("Running Scenario 3: VaR Exposure Management");
    println!("============================================\n");

    let config = ModelConfig::scenario_3();
    let seed = 900u64;
    let sim_years = 50;

    println!("Configuration:");
    println!("  Catastrophes: λ={}/year", config.mean_cat_events_per_year);
    println!("  VaR exceedance prob: {}", config.var_exceedance_prob);
    println!("  VaR safety factor: {}", config.var_safety_factor);
    println!(
        "  Initial capital: ${:.1}M",
        config.initial_capital / 1_000_000.0
    );
    println!();

    // Create initial events
    let events = vec![(0, Event::Day)];

    // Create agents
    let agents: Vec<Box<dyn des::Agent<Event, Stats>>> = vec![
        Box::new(TimeGenerator::new()),
        Box::new(Syndicate::new(0, config.clone())),
        Box::new(Syndicate::new(1, config.clone())),
        Box::new(Syndicate::new(2, config.clone())),
        Box::new(Syndicate::new(3, config.clone())),
        Box::new(Syndicate::new(4, config.clone())),
        Box::new(BrokerPool::new(25, config.clone(), seed + 100)),
        Box::new(CentralRiskRepository::new(config.clone(), 5, seed + 200)),
        Box::new(AttritionalLossGenerator::new(config.clone(), seed + 300)),
        Box::new(CatastropheLossGenerator::new(
            config.clone(),
            sim_years,
            seed + 400,
        )),
        Box::new(MarketStatisticsCollector::new(5)),
    ];

    // Run simulation
    println!("Running simulation for {} years...", sim_years);
    let mut event_loop = EventLoop::new(events, agents);
    event_loop.run(365 * sim_years);

    // Extract stats
    let stats = event_loop.stats();

    // Find combined stats
    let combined_stats = stats
        .iter()
        .find_map(|s| match s {
            Stats::CombinedMarketStats(cs) => Some(cs),
            _ => None,
        })
        .expect("Combined market stats should exist");

    // Export both market and syndicate time series
    let market_path = "scenario3_market_time_series.csv";
    let mut market_file = File::create(market_path).expect("Failed to create market file");
    writeln!(
        market_file,
        "year,day,avg_premium,avg_loss_ratio,num_solvent_syndicates,num_insolvent_syndicates,total_capital,total_policies,premium_std_dev,markup_avg,markup_std_dev,cat_event_occurred,cat_event_loss,avg_uniform_deviation"
    ).unwrap();

    for snapshot in &combined_stats.market_series.snapshots {
        writeln!(
            market_file,
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
        )
        .unwrap();
    }

    let syndicate_path = "scenario3_syndicate_time_series.csv";
    let mut syndicate_file = File::create(syndicate_path).expect("Failed to create syndicate file");
    writeln!(
        syndicate_file,
        "year,syndicate_id,capital,markup_m_t,loss_ratio,num_policies,annual_premiums,annual_claims"
    )
    .unwrap();

    for snapshot in &combined_stats.syndicate_series.snapshots {
        writeln!(
            syndicate_file,
            "{},{},{:.2},{:.4},{:.4},{},{:.2},{:.2}",
            snapshot.year,
            snapshot.syndicate_id,
            snapshot.capital,
            snapshot.markup_m_t,
            snapshot.loss_ratio,
            snapshot.num_policies,
            snapshot.annual_premiums,
            snapshot.annual_claims,
        )
        .unwrap();
    }

    println!("✓ Exported: {}", market_path);
    println!("✓ Exported: {}", syndicate_path);

    // Print summary
    let final_snapshot = combined_stats.market_series.snapshots.last().unwrap();
    println!("\n{}", "=".repeat(60));
    println!("FINAL STATE (Year {})", sim_years);
    println!("{}", "=".repeat(60));
    println!(
        "  Solvent syndicates: {}/5",
        final_snapshot.num_solvent_syndicates
    );
    println!(
        "  Insolvent syndicates: {}/5",
        final_snapshot.num_insolvent_syndicates
    );
    println!(
        "  Total capital: ${:.2}M",
        final_snapshot.total_capital / 1_000_000.0
    );
    println!("  Avg premium: ${:.0}", final_snapshot.avg_premium);
    println!("  Avg loss ratio: {:.3}", final_snapshot.avg_loss_ratio);
    println!(
        "  Avg uniform deviation: {:.4}",
        final_snapshot.avg_uniform_deviation
    );

    // Compare with Scenario 2 expectations
    println!("\n{}", "=".repeat(60));
    println!("PAPER EXPECTATIONS (Scenario 3 vs Scenario 2)");
    println!("{}", "=".repeat(60));
    println!("  ✓ Uniform deviation → 0 (better risk distribution)");
    println!("  ✓ Fewer insolvencies");
    println!("  ✓ Better capitalized portfolios");

    if final_snapshot.avg_uniform_deviation < 0.1 {
        println!(
            "\n✓ Uniform deviation {:.4} < 0.1 - VaR EM working!",
            final_snapshot.avg_uniform_deviation
        );
    } else {
        println!(
            "\n⚠ Uniform deviation {:.4} still high",
            final_snapshot.avg_uniform_deviation
        );
    }

    // Count catastrophes
    let cat_count = combined_stats
        .market_series
        .snapshots
        .iter()
        .filter(|s| s.cat_event_occurred)
        .count();
    println!("\nCatastrophe events: {}", cat_count);
}
