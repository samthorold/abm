use evolution_coop::*;

fn main() {
    println!("Evolution of Cooperation: Robustness Tournament");
    println!("=================================================\n");

    // Tournament parameters
    let num_rounds_per_match = 200;

    // Create players with different strategies
    let agents: Vec<Box<dyn des::Agent<Event, Stats>>> = vec![
        // Tournament coordinator
        Box::new(TournamentCoordinator::new(5, num_rounds_per_match)),
        // Players
        Box::new(Player::new(0, Box::new(TitForTat))),
        Box::new(Player::new(1, Box::new(AlwaysDefect))),
        Box::new(Player::new(2, Box::new(AlwaysCooperate))),
        Box::new(Player::new(3, Box::new(Random))),
        Box::new(Player::new(4, Box::new(Grudger))),
    ];

    // Initial event to start the tournament
    let events = vec![(0, Event::TournamentStart)];

    // Create and run event loop
    let mut event_loop = des::EventLoop::new(events, agents);

    println!("Running tournament...\n");
    event_loop.run(100000);

    // Collect and display results
    println!("Tournament Results:");
    println!("===================\n");

    let stats = event_loop.stats();

    // Separate tournament stats from player stats
    let mut player_stats: Vec<PlayerStats> = Vec::new();

    for stat in stats {
        match stat {
            Stats::Tournament(ts) => {
                println!("Tournament Statistics:");
                println!("  Completed matches: {}", ts.completed_matches);
                println!("  Active matches: {}\n", ts.active_matches);
            }
            Stats::Player(ps) => {
                player_stats.push(ps);
            }
        }
    }

    // Sort players by total score (descending)
    player_stats.sort_by(|a, b| b.total_score.cmp(&a.total_score));

    println!("Player Rankings:");
    println!("----------------");
    for (rank, ps) in player_stats.iter().enumerate() {
        let avg_score = if ps.rounds_played > 0 {
            ps.total_score as f64 / ps.rounds_played as f64
        } else {
            0.0
        };
        let coop_rate = if ps.rounds_played > 0 {
            ps.cooperations as f64 / ps.rounds_played as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "{}. {} (Player {})",
            rank + 1,
            ps.strategy_name,
            ps.player_id
        );
        println!("   Total Score: {}", ps.total_score);
        println!("   Avg Score/Round: {:.2}", avg_score);
        println!("   Rounds Played: {}", ps.rounds_played);
        println!("   Cooperation Rate: {:.1}%", coop_rate);
        println!("   Cooperations: {}, Defections: {}\n", ps.cooperations, ps.defections);
    }

    println!("\nRobustness Analysis:");
    println!("--------------------");
    println!("TitForTat demonstrates robustness by performing well against");
    println!("diverse opponents. It achieves mutual cooperation with");
    println!("cooperative strategies while protecting itself from");
    println!("exploitation by defectors.");
}
