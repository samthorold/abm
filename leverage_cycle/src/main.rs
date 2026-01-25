use leverage_cycle::analysis::StabilityAnalysis;
use leverage_cycle::scenarios::{run_all_scenarios, run_scenario, ScenarioConfig};

fn main() {
    println!("========================================");
    println!("Basel Leverage Cycle Simulation");
    println!("Based on Aymanns, Caccioli, Farmer & Tan");
    println!("========================================");

    // Run all four scenarios from the paper's Experiment 1
    let steps = 5000; // 500 years at τ=0.1
    let seed = 42;

    println!(
        "\nRunning {} steps (= {} years at τ=0.1)",
        steps,
        steps as f64 * 0.1
    );
    println!("\nScenario | Bank Size | GARCH | Expected Behavior");
    println!("---------+-----------+-------+------------------");
    println!("(i)      | Small     | None  | Fixed point equilibrium");
    println!("(ii)     | Large     | None  | Chaotic leverage cycles");
    println!("(iii)    | Small     | Strong| Mean-reverting random walk");
    println!("(iv)     | Large     | Weak  | Irregular leverage cycles");

    let results = run_all_scenarios(steps, seed);

    for result in &results {
        result.print_summary();

        let analysis = StabilityAnalysis::from_stats(&result.stats);
        println!();
        analysis.print_summary();
    }

    println!("\n========================================");
    println!("Summary Comparison");
    println!("========================================\n");

    println!(
        "{:<25} {:>12} {:>12} {:>15}",
        "Scenario", "Price Std", "Lev Range", "Stability"
    );
    println!("{:-<25} {:->12} {:->12} {:->15}", "", "", "", "");

    for result in &results {
        let analysis = StabilityAnalysis::from_stats(&result.stats);
        println!(
            "{:<25} {:>12.4} {:>12.2} {:>15}",
            result.config.name,
            result.stats.price_std,
            analysis.leverage_range,
            result.stats.stability_class
        );
    }

    // Demonstrate parameter sensitivity
    println!("\n========================================");
    println!("Parameter Sensitivity: Cyclicality (b)");
    println!("========================================\n");

    let base_params = leverage_cycle::ModelParams::deterministic_macro();

    println!(
        "{:<12} {:>12} {:>12} {:>15}",
        "b value", "Price Std", "Lev Range", "Stability"
    );
    println!("{:-<12} {:->12} {:->12} {:->15}", "", "", "", "");

    for b in [-0.5, -0.25, 0.0, 0.25, 0.5] {
        let mut params = base_params.clone();
        params.b = b;

        let config = ScenarioConfig {
            name: format!("b = {:.2}", b),
            params,
            steps: 2000,
            seed: 42,
        };

        let result = run_scenario(config);
        let analysis = StabilityAnalysis::from_stats(&result.stats);

        println!(
            "{:<12} {:>12.4} {:>12.2} {:>15}",
            format!("b = {:.2}", b),
            result.stats.price_std,
            analysis.leverage_range,
            result.stats.stability_class
        );
    }
}
