use evolution_coop::kinship::*;
use evolution_coop::*;

fn main() {
    println!("Evolution of Cooperation: Multi-Generation Takeover (Finding C)");
    println!("=================================================================\n");
    println!("Demonstrating cooperation invasion through kinship and reproduction");
    println!("Starting: 5% TIT FOR TAT → Watching spread to dominance\n");

    // Simulation parameters
    let total_population = 200;
    let tft_percentage = 0.05; // Start at 5%
    let tft_count = (total_population as f64 * tft_percentage) as usize;
    let defector_count = total_population - tft_count;

    let num_kinship_groups = 10;
    let encounters_per_generation = 100; // Moderate number of encounters
    let rounds_per_match = 10;
    let kinship_preference = 0.8; // 80% within-group matching (realistic kinship effect)
    let mutation_rate = 0.01; // 1% mutation rate (allows exploration)
    let max_generations = 50; // Longer to observe full dynamics

    println!("Parameters:");
    println!("  Population size: {}", total_population);
    println!("  Initial TFT: {} ({}%)", tft_count, tft_percentage * 100.0);
    println!("  Initial Defectors: {} ({}%)", defector_count, (1.0 - tft_percentage) * 100.0);
    println!("  Kinship groups: {}", num_kinship_groups);
    println!("  Kinship preference: {}%", kinship_preference * 100.0);
    println!("  Mutation rate: {}%", mutation_rate * 100.0);
    println!("  Encounters per generation: {}", encounters_per_generation);
    println!("  Rounds per match: {}", rounds_per_match);
    println!("  Max generations: {}", max_generations);
    println!();

    // Create population coordinator
    let mut coordinator = PopulationCoordinator::new(
        total_population,
        encounters_per_generation,
        rounds_per_match,
        kinship_preference,
        mutation_rate,
        max_generations,
    );

    // Create initial generation agents
    let mut agents: Vec<Box<dyn des::Agent<KinshipEvent, EvolutionaryStats>>> = Vec::new();
    let mut agent_id = 0;

    println!("Creating Generation 1 population:");
    println!("  {} TIT FOR TAT agents in kinship group 0", tft_count);
    println!("  {} ALWAYS DEFECT agents across {} groups\n", defector_count, num_kinship_groups);

    // Create TIT FOR TAT agents (all in kinship group 0)
    for _ in 0..tft_count {
        coordinator.register_agent(agent_id, "TitForTat".to_string(), 0);
        agents.push(Box::new(EvolutionaryPlayer::new(
            agent_id,
            1, // Generation 1
            Box::new(TitForTat),
            0,
        )));
        agent_id += 1;
    }

    // Create ALWAYS DEFECT agents (distributed across all groups)
    for i in 0..defector_count {
        let kinship_group = i % num_kinship_groups;
        coordinator.register_agent(agent_id, "AlwaysDefect".to_string(), kinship_group);
        agents.push(Box::new(EvolutionaryPlayer::new(
            agent_id,
            1, // Generation 1
            Box::new(AlwaysDefect),
            kinship_group,
        )));
        agent_id += 1;
    }

    // Add coordinator as agent
    agents.insert(0, Box::new(coordinator));

    // Initial event
    let events = vec![(0, KinshipEvent::GenerationStart { generation: 1 })];

    // Create and run event loop
    let mut event_loop = des::EventLoop::new(events, agents);

    println!("=================================================================");
    println!("Running {} generations...", max_generations);
    println!("=================================================================\n");

    println!("{:<4} | {:<6} | {:<8} | {:<8} | {:<10} | {:<10}",
             "Gen", "TFT%", "Avg TFT", "Avg Def", "TFT Fit", "Def Fit");
    println!("{}", "-".repeat(70));

    // Run simulation
    event_loop.run(10000000);

    // Collect final stats
    let stats = event_loop.stats();

    // Organize stats by generation
    let mut gen_stats: std::collections::HashMap<usize, Vec<AgentStats>> = std::collections::HashMap::new();

    for stat in stats {
        if let EvolutionaryStats::Agent(agent_stat) = stat {
            gen_stats
                .entry(agent_stat.generation)
                .or_insert_with(Vec::new)
                .push(agent_stat);
        }
    }

    // Print per-generation stats
    for gen in 1..=max_generations {
        if let Some(agents) = gen_stats.get(&gen) {
            let tft_count = agents.iter().filter(|a| a.strategy_name == "TitForTat").count();
            let def_count = agents.iter().filter(|a| a.strategy_name == "AlwaysDefect").count();
            let tft_pct = if !agents.is_empty() {
                tft_count as f64 / agents.len() as f64 * 100.0
            } else {
                0.0
            };

            let tft_agents: Vec<&AgentStats> = agents.iter().filter(|a| a.strategy_name == "TitForTat").collect();
            let def_agents: Vec<&AgentStats> = agents.iter().filter(|a| a.strategy_name == "AlwaysDefect").collect();

            let avg_tft = if !tft_agents.is_empty() {
                tft_agents.iter().map(|a| a.fitness).sum::<f64>() / tft_agents.len() as f64
            } else {
                0.0
            };

            let avg_def = if !def_agents.is_empty() {
                def_agents.iter().map(|a| a.fitness).sum::<f64>() / def_agents.len() as f64
            } else {
                0.0
            };

            println!("{:<4} | {:<6.1} | {:<8.2} | {:<8.2} | {:<10} | {:<10}",
                     gen, tft_pct, avg_tft, avg_def, tft_count, def_count);
        }
    }

    // Print final generation summary
    println!("\n{}", "=".repeat(70));
    println!("Final Results (Generation {})", max_generations);
    println!("{}", "=".repeat(70));

    // Get final generation agents only
    let all_agents: Vec<AgentStats> = gen_stats
        .get(&max_generations)
        .map(|v| v.clone())
        .unwrap_or_default();

    let tft_agents: Vec<&AgentStats> = all_agents
        .iter()
        .filter(|a| a.strategy_name == "TitForTat")
        .collect();

    let defector_agents: Vec<&AgentStats> = all_agents
        .iter()
        .filter(|a| a.strategy_name == "AlwaysDefect")
        .collect();

    let tft_final_pct = if !all_agents.is_empty() {
        tft_agents.len() as f64 / all_agents.len() as f64 * 100.0
    } else {
        0.0
    };

    let tft_fitness_sum: f64 = tft_agents.iter().map(|a| a.fitness).sum();
    let def_fitness_sum: f64 = defector_agents.iter().map(|a| a.fitness).sum();

    let avg_tft_fitness = if !tft_agents.is_empty() {
        tft_fitness_sum / tft_agents.len() as f64
    } else {
        0.0
    };

    let avg_def_fitness = if !defector_agents.is_empty() {
        def_fitness_sum / defector_agents.len() as f64
    } else {
        0.0
    };

    println!("\nPopulation Composition:");
    println!("  Total agents: {}", all_agents.len());
    println!("  TIT FOR TAT: {} ({:.1}%)", tft_agents.len(), tft_final_pct);
    println!("  ALWAYS DEFECT: {} ({:.1}%)", defector_agents.len(), 100.0 - tft_final_pct);
    println!("\nFitness Summary:");
    println!("  Avg TFT fitness: {:.2}", avg_tft_fitness);
    println!("  Avg Defector fitness: {:.2}", avg_def_fitness);
    println!("  Fitness ratio (TFT/Defector): {:.2}",
             if avg_def_fitness > 0.0 { avg_tft_fitness / avg_def_fitness } else { 0.0 });

    println!("\n{}", "=".repeat(70));
    println!("Analysis: Initial Viability → Invasion Dynamics");
    println!("{}", "=".repeat(70));

    if tft_final_pct > 50.0 {
        println!("\n✓ SUCCESS: Cooperation has invaded and taken over!");
        println!("\n  Initial state: {}% TIT FOR TAT (rare cooperators)", tft_percentage * 100.0);
        println!("  Final state: {:.1}% TIT FOR TAT (majority)", tft_final_pct);
        println!("\n  Invasion path:");
        println!("    1. Kinship (80% within-group) → TFT agents meet each other");
        println!("    2. Mutual cooperation → TFT agents achieve higher fitness");
        println!("    3. Reproduction → Fitness-proportional selection favors TFT");
        println!("    4. Spread → TFT offspring fill population over generations");
        println!("\n  Key Finding C Validated:");
        println!("    'A small cluster can be initially viable and eventually invade'");
        println!("\n  This demonstrates Axelrod & Hamilton's evolutionary ratchet:");
        println!("    - Nice strategies can invade in clusters (kinship)");
        println!("    - Once established, they resist invasion by defectors");
        println!("    - Cooperation evolves through individual selection!");
    } else if tft_final_pct > tft_percentage * 100.0 {
        println!("\n⚡ PARTIAL SUCCESS: Cooperation increased but hasn't fully taken over");
        println!("  Initial: {:.1}%", tft_percentage * 100.0);
        println!("  Final: {:.1}%", tft_final_pct);
        println!("\n  May need:");
        println!("    - More generations");
        println!("    - Higher kinship preference");
        println!("    - More encounters per generation");
    } else {
        println!("\n⚠ UNEXPECTED: Cooperation did not increase");
        println!("  This suggests a configuration issue");
    }

    println!();
}
