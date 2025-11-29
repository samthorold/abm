use evolution_coop::kinship::*;
use evolution_coop::*;

fn main() {
    println!("Evolution of Cooperation: Initial Viability (Finding C)");
    println!("========================================================\n");
    println!("Testing kinship-based emergence of cooperation");
    println!("Starting population: 5% TIT FOR TAT, 95% ALWAYS DEFECT\n");

    // Simulation parameters
    let total_population = 200;
    let tft_percentage = 0.05; // 5%
    let tft_count = (total_population as f64 * tft_percentage) as usize;
    let defector_count = total_population - tft_count;

    let num_kinship_groups = 10;
    let encounters_per_generation = 100;
    let rounds_per_match = 10; // ~10 rounds with w=0.9
    let kinship_preference = 0.8; // 80% chance to match within group

    println!("Parameters:");
    println!("  Total population: {}", total_population);
    println!("  TIT FOR TAT: {} ({}%)", tft_count, tft_percentage * 100.0);
    println!("  ALWAYS DEFECT: {} ({}%)", defector_count, (1.0 - tft_percentage) * 100.0);
    println!("  Kinship groups: {}", num_kinship_groups);
    println!("  TFT kinship group: 0 (all TFT agents)");
    println!("  Kinship preference: {}%", kinship_preference * 100.0);
    println!("  Encounters per generation: {}", encounters_per_generation);
    println!("  Rounds per match: {}", rounds_per_match);
    println!();

    // Create population coordinator (single generation mode: max_generations = 1)
    let mut coordinator = PopulationCoordinator::new(
        total_population,
        encounters_per_generation,
        rounds_per_match,
        kinship_preference,
        0.0, // No mutation in single generation
        1, // Only run 1 generation
    );

    // Create agents
    let mut agents: Vec<Box<dyn des::Agent<KinshipEvent, EvolutionaryStats>>> = Vec::new();
    let mut agent_id = 0;

    // Create TIT FOR TAT agents (all in kinship group 0)
    println!("Creating {} TIT FOR TAT agents in kinship group 0...", tft_count);
    for _ in 0..tft_count {
        coordinator.register_agent(agent_id, "TitForTat".to_string(), 0);
        agents.push(Box::new(EvolutionaryPlayer::new(
            agent_id,
            1, // Generation 1
            Box::new(TitForTat),
            0, // All TFT in group 0
        )));
        agent_id += 1;
    }

    // Create ALWAYS DEFECT agents (distributed across all groups)
    println!("Creating {} ALWAYS DEFECT agents across {} kinship groups...", defector_count, num_kinship_groups);
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

    println!("\nRunning generation 1...\n");
    event_loop.run(1000000);

    // Collect and display results
    println!("\n=================================================");
    println!("Generation 1 Results");
    println!("=================================================\n");

    let stats = event_loop.stats();

    let mut population_stats: Option<PopulationStats> = None;
    let mut agent_stats: Vec<AgentStats> = Vec::new();

    for stat in stats {
        match stat {
            EvolutionaryStats::Population(ps) => {
                population_stats = Some(ps);
            }
            EvolutionaryStats::Agent(as_) => {
                agent_stats.push(as_);
            }
        }
    }

    // Calculate actual population statistics
    let mut tft_agents: Vec<&AgentStats> = agent_stats
        .iter()
        .filter(|a| a.strategy_name == "TitForTat")
        .collect();

    let mut defector_agents: Vec<&AgentStats> = agent_stats
        .iter()
        .filter(|a| a.strategy_name == "AlwaysDefect")
        .collect();

    let tft_total_fitness: f64 = tft_agents.iter().map(|a| a.fitness).sum();
    let defector_total_fitness: f64 = defector_agents.iter().map(|a| a.fitness).sum();

    let avg_tft_fitness = if !tft_agents.is_empty() {
        tft_total_fitness / tft_agents.len() as f64
    } else {
        0.0
    };

    let avg_defector_fitness = if !defector_agents.is_empty() {
        defector_total_fitness / defector_agents.len() as f64
    } else {
        0.0
    };

    println!("Population Summary:");
    println!("  Total agents: {}", agent_stats.len());
    println!("  TIT FOR TAT: {} ({:.1}%)", tft_agents.len(),
             tft_agents.len() as f64 / agent_stats.len() as f64 * 100.0);
    println!("  ALWAYS DEFECT: {} ({:.1}%)", defector_agents.len(),
             defector_agents.len() as f64 / agent_stats.len() as f64 * 100.0);

    if let Some(ps) = population_stats {
        println!("  Total matches: {}", ps.total_matches);
    }
    println!();

    println!("Fitness Summary:");
    println!("  Avg TIT FOR TAT fitness: {:.2}", avg_tft_fitness);
    println!("  Avg ALWAYS DEFECT fitness: {:.2}", avg_defector_fitness);
    println!("  Fitness ratio (TFT/Defector): {:.2}",
             if avg_defector_fitness > 0.0 { avg_tft_fitness / avg_defector_fitness } else { 0.0 });
    println!();

    // Sort agents by fitness
    tft_agents.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());
    defector_agents.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap());

    println!("Top 5 TIT FOR TAT agents:");
    for (i, agent) in tft_agents.iter().take(5).enumerate() {
        println!("  {}. Agent {} (Group {}): Fitness = {:.2}, Matches = {}",
                 i + 1, agent.agent_id, agent.kinship_group, agent.fitness, agent.matches_played);
    }
    println!();

    println!("Top 5 ALWAYS DEFECT agents:");
    for (i, agent) in defector_agents.iter().take(5).enumerate() {
        println!("  {}. Agent {} (Group {}): Fitness = {:.2}, Matches = {}",
                 i + 1, agent.agent_id, agent.kinship_group, agent.fitness, agent.matches_played);
    }
    println!();

    // Analyze kinship effects
    println!("Kinship Group Analysis:");
    for group in 0..num_kinship_groups {
        let group_agents: Vec<&AgentStats> = agent_stats
            .iter()
            .filter(|a| a.kinship_group == group)
            .collect();

        if group_agents.is_empty() {
            continue;
        }

        let group_fitness: f64 = group_agents.iter().map(|a| a.fitness).sum();
        let avg_group_fitness = group_fitness / group_agents.len() as f64;

        let tft_in_group = group_agents.iter().filter(|a| a.strategy_name == "TitForTat").count();
        let defectors_in_group = group_agents.iter().filter(|a| a.strategy_name == "AlwaysDefect").count();

        println!("  Group {}: {} agents (TFT: {}, Defect: {}), Avg fitness: {:.2}",
                 group, group_agents.len(), tft_in_group, defectors_in_group, avg_group_fitness);
    }
    println!();

    println!("\n=================================================");
    println!("Analysis");
    println!("=================================================\n");

    if avg_tft_fitness > avg_defector_fitness {
        println!("✓ SUCCESS: TIT FOR TAT has higher average fitness than ALWAYS DEFECT!");
        println!("  This demonstrates that kinship allows cooperators to thrive");
        println!("  even when rare ({}%).", tft_percentage * 100.0);
        println!();
        println!("Key Finding C Validation:");
        println!("  'A small cluster of individuals using [TIT FOR TAT] with even");
        println!("   a tiny probability of getting together can be initially viable'");
        println!();
        println!("  Mechanism: Kinship-based preferential interaction ({}% within-group)",
                 kinship_preference * 100.0);
        println!("  Result: TFT cooperators in group 0 interact primarily with each other,");
        println!("          achieving high payoffs through mutual cooperation (R=3 per round).");
        println!("          Defectors mostly interact with other defectors (P=1 per round).");
        println!();
        println!("Next steps would include:");
        println!("  - Multi-generation simulation with reproduction proportional to fitness");
        println!("  - Tracking TFT percentage over time to show invasion dynamics");
        println!("  - Demonstrating spread from kinship group 0 to other groups");
    } else if avg_tft_fitness.abs() < 0.01 && avg_defector_fitness.abs() < 0.01 {
        println!("⚠ WARNING: Very low fitness scores suggest simulation issue");
        println!("  Check that matches are actually being played");
    } else {
        println!("⚠ UNEXPECTED: ALWAYS DEFECT has higher fitness");
        println!("  This may indicate:");
        println!("  - Kinship preference too low (currently {}%)", kinship_preference * 100.0);
        println!("  - Too few encounters (currently {})", encounters_per_generation);
        println!("  - Implementation issue");
    }
}
