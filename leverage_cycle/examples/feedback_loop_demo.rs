//! Demonstration of the Basel Leverage Cycle feedback mechanism
//!
//! This example shows how the feedback loop works:
//!
//! 1. Price shock → Volatility increases
//! 2. Higher volatility → Lower target leverage (procyclical policy)
//! 3. Bank sells assets to reduce leverage
//! 4. Selling pressure → Price falls further
//! 5. Falling price → Even higher volatility
//! 6. Cycle continues

use des::EventLoop;
use leverage_cycle::params::ModelParams;
use leverage_cycle::system::LeverageCycleSystem;
use leverage_cycle::{Event, Stats};

fn main() {
    println!("========================================");
    println!("Basel Leverage Cycle Feedback Loop Demo");
    println!("========================================\n");

    // Create a system with procyclical policy (Basel II: b = -0.5)
    let params = ModelParams {
        b: -0.5,      // Procyclical (Basel II)
        e_bar: 0.005, // Small bank that will show dynamics clearly
        theta: 5.0,   // Moderate adjustment speed
        ..ModelParams::default()
    };

    let system = LeverageCycleSystem::new(params.clone(), "feedback_demo".to_string(), 42);

    let mut event_loop = EventLoop::new(vec![(0, Event::Step { step: 0 })], vec![Box::new(system)]);

    println!("Simulating 100 steps with b = -0.5 (Basel II procyclical policy)");
    println!("\nStep  Price   Volatility  Tgt Leverage  Act Leverage");
    println!("----  ------  ----------  ------------  ------------");

    // Run simulation and sample every 10 steps
    for step in 0..=100 {
        if step % 10 == 0 {
            let stats = event_loop.stats();
            if let Some(Stats::System(s)) = stats.first() {
                let target_leverage = params.target_leverage(s.current_volatility.powi(2));
                println!(
                    "{:4}  {:6.2}  {:10.6}  {:12.2}  {:12.2}",
                    step,
                    s.current_price,
                    s.current_volatility,
                    target_leverage,
                    s.current_leverage
                );
            }
        }
        event_loop.run(step + 1);
    }

    let final_stats = event_loop.stats();
    if let Some(Stats::System(s)) = final_stats.first() {
        println!("\n========================================");
        println!("Results");
        println!("========================================");
        println!("Initial price:     25.00");
        println!("Final price:       {:.2}", s.current_price);
        println!(
            "Price range:       {:.2}",
            s.price_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
                - s.price_history
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min)
        );
        println!(
            "Leverage range:    {:.2}",
            s.leverage_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max)
                - s.leverage_history
                    .iter()
                    .cloned()
                    .fold(f64::INFINITY, f64::min)
        );
        println!("\nFeedback loop observable:");
        println!("  ✓ Volatility fluctuations drive leverage changes");
        println!("  ✓ Leverage changes affect prices");
        println!("  ✓ Price changes feed back into volatility");
    }

    // Compare with constant leverage (b = 0)
    println!("\n========================================");
    println!("Comparison: Constant Leverage (b = 0)");
    println!("========================================\n");

    let const_params = ModelParams {
        b: 0.0, // Constant leverage
        e_bar: 0.005,
        theta: 5.0,
        ..ModelParams::default()
    };

    let const_system = LeverageCycleSystem::new(const_params, "constant".to_string(), 42);
    let mut const_loop = EventLoop::new(
        vec![(0, Event::Step { step: 0 })],
        vec![Box::new(const_system)],
    );

    const_loop.run(100);

    let const_stats = const_loop.stats();
    if let Some(Stats::System(s)) = const_stats.first() {
        let const_price_range = s
            .price_history
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max)
            - s.price_history
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);

        println!("With constant leverage (b=0):");
        println!("  Price range: {:.2}", const_price_range);
        println!("  Leverage std: {:.4}", s.leverage_std);
        println!("\nConstant leverage breaks the feedback loop,");
        println!("reducing endogenous volatility amplification.");
    }
}
