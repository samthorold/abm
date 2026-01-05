use zi_traders::analysis::{AggregateResults, PeriodResults, SessionResults};
use zi_traders::coordinator::Coordinator;
use zi_traders::market_configs::MarketConfig;
use zi_traders::traders::{ZICTrader, ZIUTrader};
use zi_traders::{Event, Role, Stats, TraderType};

const NUM_PERIODS: usize = 6;
const MAX_ITERATIONS_PER_PERIOD: usize = 500;

/// Run a single trading period
fn run_period(
    market_config: &MarketConfig,
    trader_type: TraderType,
    period: usize,
    seed: u64,
) -> PeriodResults {
    // Create coordinator
    let coordinator = Coordinator::new(market_config.clone(), MAX_ITERATIONS_PER_PERIOD, seed);

    // Create traders
    let mut traders: Vec<Box<dyn des::Agent<Event, Stats>>> = Vec::new();

    // Add coordinator first
    traders.push(Box::new(coordinator));

    // Create buyers (IDs 0..num_buyers)
    for buyer_id in 0..market_config.num_buyers() {
        let units = market_config.buyer_values[buyer_id]
            .iter()
            .map(|&value| zi_traders::Unit {
                value_or_cost: value,
            })
            .collect();

        let trader_seed = seed * 1000 + buyer_id as u64;

        let trader: Box<dyn des::Agent<Event, Stats>> = match trader_type {
            TraderType::ZIU => Box::new(ZIUTrader::new(buyer_id, Role::Buyer, units, trader_seed)),
            TraderType::ZIC => Box::new(ZICTrader::new(buyer_id, Role::Buyer, units, trader_seed)),
        };

        traders.push(trader);
    }

    // Create sellers (IDs num_buyers..num_buyers+num_sellers)
    for seller_idx in 0..market_config.num_sellers() {
        let seller_id = market_config.num_buyers() + seller_idx;
        let units = market_config.seller_costs[seller_idx]
            .iter()
            .map(|&cost| zi_traders::Unit {
                value_or_cost: cost,
            })
            .collect();

        let trader_seed = seed * 1000 + seller_id as u64;

        let trader: Box<dyn des::Agent<Event, Stats>> = match trader_type {
            TraderType::ZIU => {
                Box::new(ZIUTrader::new(seller_id, Role::Seller, units, trader_seed))
            }
            TraderType::ZIC => {
                Box::new(ZICTrader::new(seller_id, Role::Seller, units, trader_seed))
            }
        };

        traders.push(trader);
    }

    // Create event loop with period start event
    let initial_events = vec![(
        0,
        Event::PeriodStart {
            period,
            market_id: market_config.id,
        },
    )];
    let mut event_loop = des::EventLoop::new(initial_events, traders);

    // Run until period end (max iterations * 2 to account for all events)
    let max_time = MAX_ITERATIONS_PER_PERIOD * 2 + 100;
    event_loop.run(max_time);

    // Collect stats from coordinator
    let stats = event_loop.stats();
    if let Some(Stats::Coordinator(coord_stats)) = stats.first() {
        PeriodResults::from_coordinator_stats(coord_stats)
    } else {
        panic!("Expected coordinator stats");
    }
}

/// Run a single trading session (multiple periods)
fn run_session(
    market_config: &MarketConfig,
    trader_type: TraderType,
    session_id: usize,
) -> SessionResults {
    let mut period_results = Vec::new();

    for period in 0..NUM_PERIODS {
        let seed = session_id as u64 * 100 + period as u64;
        let results = run_period(market_config, trader_type, period, seed);
        period_results.push(results);
    }

    SessionResults::from_periods(session_id, period_results)
}

/// Run multiple sessions and aggregate results
fn run_experiment(
    market_config: &MarketConfig,
    trader_type: TraderType,
    num_sessions: usize,
) -> AggregateResults {
    let mut sessions = Vec::new();

    for session_id in 0..num_sessions {
        if session_id % 100 == 0 && session_id > 0 {
            println!("  Completed {}/{} sessions", session_id, num_sessions);
        }
        let results = run_session(market_config, trader_type, session_id);
        sessions.push(results);
    }

    AggregateResults::from_sessions(&sessions)
}

fn main() {
    println!("Zero-Intelligence Traders Simulation");
    println!("Replicating Gode & Sunder (1993)");
    println!("=====================================\n");

    let markets = MarketConfig::all_markets();
    let trader_types = [TraderType::ZIU, TraderType::ZIC];
    let num_sessions = 100; // Start with 100, can increase to 1000 for full replication

    for market in &markets {
        println!(
            "\n{} (Eq. Price: {}, Eq. Qty: {})",
            market.name, market.equilibrium_price, market.equilibrium_quantity
        );
        println!("Max Surplus: {}", market.calculate_max_surplus());
        println!("{}", "=".repeat(60));

        for trader_type in &trader_types {
            println!(
                "\nRunning {} sessions with {}...",
                num_sessions, trader_type
            );
            let results = run_experiment(market, *trader_type, num_sessions);
            results.print_summary(&market.name, &format!("{}", trader_type));
        }
    }

    println!("\n\nValidation Summary");
    println!("==================");
    println!("\nExpected Results (from paper):");
    println!("1. ZI-C efficiency: ≥97% across all markets");
    println!("2. ZI-U vs ZI-C gap: ≥10 percentage points");
    println!("3. ZI-C convergence: Negative slope (p<0.05)");
    println!("4. ZI-U non-convergence: Slope ≈ 0");
    println!("\nRefer to output above to verify these criteria.");
}
