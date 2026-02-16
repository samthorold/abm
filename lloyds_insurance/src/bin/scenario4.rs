use des::EventLoop;
use lloyds_insurance::{
    AttritionalLossGenerator, BrokerPool, CatastropheLossGenerator, CentralRiskRepository, Event,
    MarketStatisticsCollector, ModelConfig, Stats, Syndicate, TimeGenerator,
};
use std::fs::File;
use std::io::Write as IoWrite;

fn main() {
    println!("Running Scenario 4: Lead-Follow Syndication");
    println!("============================================\n");

    let config = ModelConfig::scenario_4();
    let seed = 99999u64; // Try different seed
    let sim_years = 50;

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

    // Export syndicate time series
    let output_path = "scenario4_syndicate_time_series.csv";
    let mut file = File::create(output_path).expect("Failed to create file");

    writeln!(
        file,
        "year,syndicate_id,capital,markup_m_t,loss_ratio,num_policies,annual_premiums,annual_claims"
    )
    .unwrap();

    for snapshot in &combined_stats.syndicate_series.snapshots {
        writeln!(
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
        )
        .unwrap();
    }

    println!("✓ Exported: {}", output_path);
    println!(
        "  {} syndicate-year records",
        combined_stats.syndicate_series.snapshots.len()
    );

    // Print summary
    let final_snapshot = combined_stats.market_series.snapshots.last().unwrap();
    println!("\nFinal State (Year 50):");
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

    if final_snapshot.num_insolvent_syndicates == 0 {
        println!("\n✓ ZERO INSOLVENCIES - Paper claim validated!");
    } else {
        println!(
            "\n⚠ {} insolvencies occurred (paper claims zero)",
            final_snapshot.num_insolvent_syndicates
        );
    }
}
