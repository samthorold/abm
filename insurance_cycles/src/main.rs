//! Insurance Industry Complex Social System - Main Simulation
//!
//! Implements the Owadally et al. (2018) model demonstrating endogenous
//! underwriting cycles from simple firm-level behavior.

use des::EventLoop;
use insurance_cycles::claim_generator::ClaimGenerator;
use insurance_cycles::insurer::Insurer;
use insurance_cycles::market_coordinator::MarketCoordinator;
use insurance_cycles::{Customer, Event, ModelConfig, Stats, DAYS_PER_YEAR};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;
use std::f64::consts::PI;

fn main() {
    println!("=== Insurance Industry Complex Social System ===");
    println!("Owadally et al. (2018) - Endogenous Underwriting Cycles\n");

    // Configuration
    let config = ModelConfig::baseline();
    let num_years = 100; // Run for 100 years to see clear cycles
    let seed = 42;

    println!("Configuration:");
    println!("  Insurers: {}", config.num_insurers);
    println!("  Customers: {}", config.num_customers);
    println!(
        "  Underwriter smoothing (β): {}",
        config.underwriter_smoothing
    );
    println!("  Credibility factor (z): {}", config.credibility_factor);
    println!("  Distance cost (γ): {}", config.distance_cost);
    println!("  Years to simulate: {}\n", num_years);

    // Initialize RNG for setup
    let mut setup_rng = StdRng::seed_from_u64(seed);

    // Create customers uniformly distributed on circle
    let customers: Vec<Customer> = (0..config.num_customers)
        .map(|i| {
            let position = setup_rng.gen_range(0.0..(2.0 * PI));
            Customer::new(i, position)
        })
        .collect();

    println!("Created {} customers", customers.len());

    // Create insurers uniformly distributed on circle
    let insurer_positions: HashMap<usize, f64> = (0..config.num_insurers)
        .map(|i| {
            let position = setup_rng.gen_range(0.0..(2.0 * PI));
            (i, position)
        })
        .collect();

    println!("Created {} insurers", insurer_positions.len());

    // Create all agents
    let mut agents: Vec<Box<dyn des::Agent<Event, Stats>>> = Vec::new();

    // Add insurers as agents
    for (insurer_id, &position) in &insurer_positions {
        let insurer = Insurer::new(
            *insurer_id,
            position,
            config.clone(),
            seed + (*insurer_id as u64),
        );
        agents.push(Box::new(insurer));
    }

    // Add market coordinator
    let coordinator =
        MarketCoordinator::new(config.clone(), customers.clone(), insurer_positions.clone());
    agents.push(Box::new(coordinator));

    // Add claim generator
    let claim_generator = ClaimGenerator::new(config.clone(), seed + 1000);
    agents.push(Box::new(claim_generator));

    println!("Agents initialized: {} agents", agents.len());

    // Schedule initial YearStart events for each year
    println!("Scheduling {} years of simulation...", num_years);
    let mut initial_events = Vec::new();
    for year in 1..=num_years {
        let time = year * DAYS_PER_YEAR;
        initial_events.push((time, Event::YearStart { year }));
    }

    // Initialize EventLoop with events and agents
    let mut event_loop: EventLoop<Event, Stats> = EventLoop::new(initial_events, agents);

    // Run simulation
    println!("Running simulation...\n");
    let max_time = (num_years + 1) * DAYS_PER_YEAR;
    event_loop.run(max_time);

    println!("Simulation complete!\n");

    // Collect and analyze results
    println!("=== Results ===\n");

    let all_stats = event_loop.stats();

    // Extract market statistics
    let market_stats: Vec<_> = all_stats
        .iter()
        .filter_map(|s| {
            if let Stats::Market(ms) = s {
                Some(ms)
            } else {
                None
            }
        })
        .collect();

    if let Some(final_market) = market_stats.first() {
        println!("Final Market State (Year {}):", final_market.year);
        println!(
            "  Industry loss ratio: {:.3}",
            final_market.industry_loss_ratio
        );
        println!(
            "  Industry avg claim: ${:.2}",
            final_market.industry_avg_claim
        );
        println!(
            "  Solvent insurers: {}/{}",
            final_market.num_solvent_insurers, final_market.total_insurers
        );
        println!(
            "  Price range: ${:.2} - ${:.2}",
            final_market.min_price, final_market.max_price
        );
        println!("  Average price: ${:.2}\n", final_market.avg_price);

        // Loss ratio time series
        println!("Loss Ratio History:");
        println!("  Years tracked: {}", final_market.loss_ratio_history.len());

        if !final_market.loss_ratio_history.is_empty() {
            println!("  Mean: {:.3}", final_market.mean_loss_ratio());
            println!("  Std Dev: {:.3}", final_market.std_loss_ratio());

            // Check for cycles
            if final_market.has_cycles() {
                println!("  ✓ Cycles detected!");
                if let Some(period) = final_market.cycle_period() {
                    println!("  Estimated cycle period: {:.1} years", period);
                }
            } else {
                println!("  ✗ No clear cycles detected");
            }

            // Print recent history
            println!("\n  Recent loss ratios:");
            let start = final_market.loss_ratio_history.len().saturating_sub(10);
            for (i, &ratio) in final_market.loss_ratio_history[start..].iter().enumerate() {
                let year = start + i + 1;
                println!("    Year {}: {:.3}", year, ratio);
            }
        }
    }

    // Insurer statistics
    println!("\n=== Insurer Statistics ===\n");

    let insurer_stats: Vec<_> = all_stats
        .iter()
        .filter_map(|s| {
            if let Stats::Insurer(is) = s {
                Some(is)
            } else {
                None
            }
        })
        .collect();

    if !insurer_stats.is_empty() {
        let total_insurers = insurer_stats.len();
        let solvent_insurers = insurer_stats.iter().filter(|s| s.is_solvent()).count();

        println!(
            "Solvency: {}/{} insurers solvent",
            solvent_insurers, total_insurers
        );

        // Calculate aggregate statistics
        let total_premiums: f64 = insurer_stats.iter().map(|s| s.total_premiums).sum();
        let total_claims: f64 = insurer_stats.iter().map(|s| s.total_claims).sum();
        let aggregate_loss_ratio = if total_premiums > 0.0 {
            total_claims / total_premiums
        } else {
            0.0
        };

        println!("Aggregate loss ratio: {:.3}", aggregate_loss_ratio);

        // Sample individual insurers
        println!("\nSample Insurers (first 5):");
        for stats in insurer_stats.iter().take(5) {
            println!("  Insurer {}:", stats.insurer_id);
            println!("    Capital: ${:.2}", stats.capital);
            println!("    Loss ratio: {:.3}", stats.loss_ratio);
            println!("    Current price: ${:.2}", stats.current_market_price);
            println!("    Customers: {}", stats.num_customers);
            println!("    Markup: {:.3}", stats.current_markup);
        }
    }

    println!("\n=== Simulation Complete ===");
}
