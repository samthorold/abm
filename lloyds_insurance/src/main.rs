use des::EventLoop;
use lloyds_insurance::{
    Event, Stats, ModelConfig,
    TimeGenerator, Broker, BrokerSyndicateNetwork, CentralRiskRepository,
    AttritionalLossGenerator, CatastropheLossGenerator, Syndicate,
};

fn main() {
    println!("Lloyd's of London Insurance Market Simulation");
    println!("==============================================\n");

    // Use Scenario 1 configuration: Base case with attritional losses only
    let config = ModelConfig::scenario_1();

    println!("Configuration: Scenario 1 (Base Case - Attritional Only)");
    println!("  - Risks per day: {}", config.risks_per_day);
    println!("  - Syndicate initial capital: ${:.0}", config.initial_capital);
    println!("  - Simulation time: 50 years (18,250 days)\n");

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

    // Add 25 brokers (1:5 ratio)
    for i in 0..25 {
        agents.push(Box::new(Broker::new(i, config.clone(), (i as u64) * 12345)));
    }

    // Add broker-syndicate network
    agents.push(Box::new(BrokerSyndicateNetwork::new(config.clone(), 5, 54321)));

    // Add central risk repository
    agents.push(Box::new(CentralRiskRepository::new()));

    // Add attritional loss generator
    agents.push(Box::new(AttritionalLossGenerator::new(config.clone(), 99999)));

    // Add catastrophe loss generator
    agents.push(Box::new(CatastropheLossGenerator::new(config.clone(), 50, 77777)));

    println!("Agents initialized:");
    println!("  - 1 Time Generator");
    println!("  - 5 Syndicates");
    println!("  - 25 Brokers");
    println!("  - 1 Broker-Syndicate Network");
    println!("  - 1 Central Risk Repository");
    println!("  - 1 Attritional Loss Generator");
    println!("  - 1 Catastrophe Loss Generator\n");

    // Create event loop
    let mut event_loop = EventLoop::new(events, agents);

    println!("Running simulation...\n");

    // Run simulation for 50 years (18,250 days)
    let sim_end = 365 * 50;
    event_loop.run(sim_end);

    println!("Simulation complete!\n");

    // Collect and display statistics
    let stats = event_loop.stats();

    // Filter syndicate stats
    let syndicate_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s {
            Stats::SyndicateStats(ss) => Some(ss),
            _ => None,
        })
        .collect();

    println!("==============================================");
    println!("Syndicate Results:");
    println!("==============================================");

    for s in &syndicate_stats {
        println!("\nSyndicate {}:", s.syndicate_id);
        println!("  Capital: ${:.2} (Initial: ${:.2})", s.capital, s.initial_capital);
        println!("  Policies: {}", s.num_policies);
        println!("  Premiums Collected: ${:.2}", s.total_premiums_collected);
        println!("  Claims Paid: ${:.2}", s.total_claims_paid);
        println!("  Loss Ratio: {:.2}", s.loss_ratio);
        println!("  Profit: ${:.2}", s.profit);
        println!("  Insolvent: {}", s.is_insolvent);
    }

    // Filter broker stats
    let broker_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s {
            Stats::BrokerStats(bs) => Some(bs),
            _ => None,
        })
        .collect();

    println!("\n==============================================");
    println!("Broker Summary:");
    println!("==============================================");
    let total_risks: usize = broker_stats.iter().map(|b| b.risks_generated).sum();
    let total_bound: usize = broker_stats.iter().map(|b| b.risks_bound).sum();
    println!("  Total risks generated: {}", total_risks);
    println!("  Total risks bound: {}", total_bound);

    // Repository stats
    let repo_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s {
            Stats::CentralRiskRepositoryStats(rs) => Some(rs),
            _ => None,
        })
        .collect();

    if let Some(rs) = repo_stats.first() {
        println!("\n==============================================");
        println!("Market Summary:");
        println!("==============================================");
        println!("  Total risks: {}", rs.total_risks);
        println!("  Total policies: {}", rs.total_policies);
        println!("  Total lead quotes: {}", rs.total_lead_quotes);
        println!("  Total follow quotes: {}", rs.total_follow_quotes);
    }

    // Attritional loss generator stats
    let loss_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s {
            Stats::AttritionalLossGeneratorStats(ls) => Some(ls),
            _ => None,
        })
        .collect();

    if let Some(ls) = loss_stats.first() {
        println!("\n==============================================");
        println!("Attritional Loss Summary:");
        println!("==============================================");
        println!("  Total losses generated: {}", ls.total_losses_generated);
        println!("  Total loss amount: ${:.2}", ls.total_loss_amount);
        if ls.total_losses_generated > 0 {
            println!("  Average loss: ${:.2}", ls.total_loss_amount / ls.total_losses_generated as f64);
        }
    }

    // Catastrophe loss generator stats
    let cat_stats: Vec<_> = stats.iter()
        .filter_map(|s| match s {
            Stats::CatastropheLossGeneratorStats(cs) => Some(cs),
            _ => None,
        })
        .collect();

    if let Some(cs) = cat_stats.first() {
        println!("\n==============================================");
        println!("Catastrophe Loss Summary:");
        println!("==============================================");
        println!("  Total catastrophes: {}", cs.total_catastrophes);
        println!("  Total catastrophe loss: ${:.2}", cs.total_catastrophe_loss);
        if cs.total_catastrophes > 0 {
            println!("  Average catastrophe loss: ${:.2}", cs.total_catastrophe_loss / cs.total_catastrophes as f64);
        }
        if !cs.catastrophes_by_region.is_empty() {
            println!("  Catastrophes by region:");
            let mut regions: Vec<_> = cs.catastrophes_by_region.iter().collect();
            regions.sort_by_key(|(region, _)| *region);
            for (region, count) in regions {
                println!("    Region {}: {} catastrophe(s)", region, count);
            }
        }
    }

    // Export time-series data to CSV
    export_time_series_csv(&syndicate_stats);

    println!("\n==============================================");
    println!("Simulation completed successfully!");
    println!("==============================================");
}

fn export_time_series_csv(syndicate_stats: &[&lloyds_insurance::SyndicateStats]) {
    use std::fs::File;
    use std::io::Write;

    // For now, export a simple CSV with final statistics
    // In a full implementation, we'd track these metrics over time
    let output_path = "lloyds_insurance/time_series.csv";

    let mut file = match File::create(output_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Warning: Could not create time series CSV: {}", e);
            return;
        }
    };

    // Write header
    if let Err(e) = writeln!(
        file,
        "syndicate_id,initial_capital,final_capital,total_premiums,total_claims,loss_ratio,profit,is_insolvent,num_policies"
    ) {
        eprintln!("Warning: Could not write CSV header: {}", e);
        return;
    }

    // Write data for each syndicate
    for s in syndicate_stats {
        if let Err(e) = writeln!(
            file,
            "{},{},{},{},{},{},{},{},{}",
            s.syndicate_id,
            s.initial_capital,
            s.capital,
            s.total_premiums_collected,
            s.total_claims_paid,
            s.loss_ratio,
            s.profit,
            s.is_insolvent,
            s.num_policies
        ) {
            eprintln!("Warning: Could not write CSV row: {}", e);
            return;
        }
    }

    println!("\nTime series data exported to: {}", output_path);
}
