use des::EventLoop;
use zi_traders::coordinator::Coordinator;
use zi_traders::market_configs::MarketConfig;
use zi_traders::traders::{ZICTrader, ZIUTrader};
use zi_traders::*;

fn run_experiment(
    market: &MarketConfig,
    trader_type: TraderType,
    iterations: usize,
    num_sessions: usize,
) -> Vec<f64> {
    // Note: Cannot use des::parallel here for same reason as main.rs
    use rayon::prelude::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    // Run sessions in parallel with panic isolation
    let results: Vec<Result<f64, String>> = (0..num_sessions)
        .into_par_iter()
        .map(|session| {
            // Catch panics to prevent one bad session from crashing the entire experiment
            catch_unwind(AssertUnwindSafe(|| {
                let seed = session as u64;

                // Build agents
                let mut agents: Vec<Box<dyn des::Agent<Event, Stats>>> = Vec::new();

                // Add coordinator with the specified iteration count
                agents.push(Box::new(Coordinator::new(market.clone(), iterations, seed)));

                // Add traders (IDs must start from 0 to match Coordinator expectations)
                let mut trader_id = 0;
                for buyer_values in &market.buyer_values {
                    let units: Vec<Unit> = buyer_values
                        .iter()
                        .map(|&v| Unit { value_or_cost: v })
                        .collect();
                    let agent: Box<dyn des::Agent<Event, Stats>> = match trader_type {
                        TraderType::ZIU => Box::new(ZIUTrader::new(
                            trader_id,
                            Role::Buyer,
                            units,
                            seed + trader_id as u64,
                        )),
                        TraderType::ZIC => Box::new(ZICTrader::new(
                            trader_id,
                            Role::Buyer,
                            units,
                            seed + trader_id as u64,
                        )),
                    };
                    agents.push(agent);
                    trader_id += 1;
                }
                for seller_costs in &market.seller_costs {
                    let units: Vec<Unit> = seller_costs
                        .iter()
                        .map(|&c| Unit { value_or_cost: c })
                        .collect();
                    let agent: Box<dyn des::Agent<Event, Stats>> = match trader_type {
                        TraderType::ZIU => Box::new(ZIUTrader::new(
                            trader_id,
                            Role::Seller,
                            units,
                            seed + trader_id as u64,
                        )),
                        TraderType::ZIC => Box::new(ZICTrader::new(
                            trader_id,
                            Role::Seller,
                            units,
                            seed + trader_id as u64,
                        )),
                    };
                    agents.push(agent);
                    trader_id += 1;
                }

                // Initial event
                let events = vec![(
                    0,
                    Event::PeriodStart {
                        period: 0,
                        market_id: market.id,
                    },
                )];

                // Create and run simulation
                // Pass a large time to event_loop.run() to ensure it doesn't terminate before Coordinator's max_iterations
                let mut event_loop = EventLoop::new(events, agents);
                event_loop.run(iterations * 100);

                // Collect stats
                let all_stats = event_loop.stats();
                if let Some(Stats::Coordinator(coord_stats)) = all_stats
                    .iter()
                    .find(|s| matches!(s, Stats::Coordinator(_)))
                {
                    let efficiency = coord_stats.efficiency();

                    // Debug: print first session details for each iteration count
                    if session == 0 {
                        eprintln!(
                            "[DEBUG] Market={}, Iter={}, Txns={}, OrdersProcessed={}, Eff={:.2}%",
                            market.id,
                            iterations,
                            coord_stats.num_transactions(),
                            coord_stats.orders_processed,
                            efficiency
                        );
                    }

                    efficiency
                } else {
                    0.0 // Default efficiency if no coordinator stats found
                }
            }))
            .map_err(|e| {
                eprintln!("  Session {} panicked: {:?}", session, e);
                format!("Panic in session {}", session)
            })
        })
        .collect();

    // Filter successful sessions (failed sessions are excluded from analysis)
    let efficiencies: Vec<f64> = results
        .into_iter()
        .filter_map(|r| match r {
            Ok(eff) => Some(eff),
            Err(e) => {
                eprintln!("  Warning: Skipping failed session - {}", e);
                None
            }
        })
        .collect();

    if efficiencies.len() < num_sessions {
        eprintln!(
            "  Warning: {}/{} sessions failed",
            num_sessions - efficiencies.len(),
            num_sessions
        );
    }

    efficiencies
}

fn main() {
    let iteration_counts = vec![100, 250, 500, 1000, 2000, 5000, 10000];
    let num_sessions = 50;

    println!("Testing iteration count impact on Markets 3 and 5 (ZI-C traders)");
    println!("Running {} sessions per configuration\n", num_sessions);

    // Market 3
    println!("=== MARKET 3 (Thin Market) ===");
    println!("Equilibrium: price={}, quantity={}", 106, 6);
    println!(
        "{:>10} | {:>8} | {:>8} | {:>8}",
        "Iterations", "Mean %", "Std Dev", "Min-Max"
    );
    println!("{:-<10}-+-{:-<8}-+-{:-<8}-+-{:-<8}", "", "", "", "");

    let market_3 = MarketConfig::market_3();
    for &iterations in &iteration_counts {
        let efficiencies = run_experiment(&market_3, TraderType::ZIC, iterations, num_sessions);
        let mean = efficiencies.iter().sum::<f64>() / efficiencies.len() as f64;
        let variance = efficiencies.iter().map(|e| (e - mean).powi(2)).sum::<f64>()
            / efficiencies.len() as f64;
        let std_dev = variance.sqrt();
        let min = efficiencies.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = efficiencies
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            "{:>10} | {:>7.2}% | {:>8.2} | {:>4.0}-{:<3.0}",
            iterations, mean, std_dev, min, max
        );
    }

    println!("\n=== MARKET 5 (Many Marginal Units) ===");
    println!("Equilibrium: price={}, quantity={}", 131, 24);
    println!(
        "{:>10} | {:>8} | {:>8} | {:>8}",
        "Iterations", "Mean %", "Std Dev", "Min-Max"
    );
    println!("{:-<10}-+-{:-<8}-+-{:-<8}-+-{:-<8}", "", "", "", "");

    let market_5 = MarketConfig::market_5();
    for &iterations in &iteration_counts {
        let efficiencies = run_experiment(&market_5, TraderType::ZIC, iterations, num_sessions);
        let mean = efficiencies.iter().sum::<f64>() / efficiencies.len() as f64;
        let variance = efficiencies.iter().map(|e| (e - mean).powi(2)).sum::<f64>()
            / efficiencies.len() as f64;
        let std_dev = variance.sqrt();
        let min = efficiencies.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = efficiencies
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            "{:>10} | {:>7.2}% | {:>8.2} | {:>4.0}-{:<3.0}",
            iterations, mean, std_dev, min, max
        );
    }

    println!("\n=== MARKET 1 (Baseline for Comparison) ===");
    println!("Equilibrium: price={}, quantity={}", 69, 15);
    println!(
        "{:>10} | {:>8} | {:>8} | {:>8}",
        "Iterations", "Mean %", "Std Dev", "Min-Max"
    );
    println!("{:-<10}-+-{:-<8}-+-{:-<8}-+-{:-<8}", "", "", "", "");

    let market_1 = MarketConfig::market_1();
    for &iterations in &iteration_counts {
        let efficiencies = run_experiment(&market_1, TraderType::ZIC, iterations, num_sessions);
        let mean = efficiencies.iter().sum::<f64>() / efficiencies.len() as f64;
        let variance = efficiencies.iter().map(|e| (e - mean).powi(2)).sum::<f64>()
            / efficiencies.len() as f64;
        let std_dev = variance.sqrt();
        let min = efficiencies.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = efficiencies
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            "{:>10} | {:>7.2}% | {:>8.2} | {:>4.0}-{:<3.0}",
            iterations, mean, std_dev, min, max
        );
    }
}
