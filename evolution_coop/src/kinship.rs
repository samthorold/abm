use crate::{Choice, RoundOutcome, Strategy};
use rand::Rng;
use std::collections::HashMap;

// ============================================================================
// Evolutionary Event Types
// ============================================================================

#[derive(Clone, Debug)]
pub enum KinshipEvent {
    // Start a generation of encounters
    GenerationStart { generation: usize },

    // Request encounter partner for an agent
    EncounterRequest { agent_id: usize },

    // Two agents play iterated prisoner's dilemma
    PlayMatch {
        agent_a_id: usize,
        agent_b_id: usize,
        rounds: usize,
    },

    // Agent plays one round in a match
    PlayRound {
        match_id: usize,
        agent_a_id: usize,
        agent_b_id: usize,
        round_num: usize,
    },

    // Round decision made
    RoundDecision {
        match_id: usize,
        agent_id: usize,
        choice: Choice,
    },

    // Round completed
    RoundResult {
        match_id: usize,
        round_num: usize,
        agent_a_id: usize,
        agent_b_id: usize,
        agent_a_choice: Choice,
        agent_b_choice: Choice,
    },

    // Match completed, update fitness
    MatchComplete {
        match_id: usize,
        agent_a_id: usize,
        agent_b_id: usize,
        agent_a_payoff: i32,
        agent_b_payoff: i32,
    },

    // Generation complete, trigger reproduction
    GenerationComplete { generation: usize },

    // Reproduction event - collect fitness and create next generation
    Reproduction { generation: usize },
}

// ============================================================================
// Evolutionary Statistics
// ============================================================================

#[derive(Clone, Debug)]
pub struct PopulationStats {
    pub generation: usize,
    pub total_agents: usize,
    pub tft_count: usize,
    pub defector_count: usize,
    pub tft_percentage: f64,
    pub avg_tft_fitness: f64,
    pub avg_defector_fitness: f64,
    pub total_matches: usize,
}

#[derive(Clone, Debug)]
pub struct AgentStats {
    pub agent_id: usize,
    pub generation: usize,
    pub kinship_group: usize,
    pub strategy_name: &'static str,
    pub fitness: f64,
    pub matches_played: usize,
}

#[derive(Clone, Debug)]
pub enum EvolutionaryStats {
    Population(PopulationStats),
    Agent(AgentStats),
}

// ============================================================================
// Evolutionary Player Agent
// ============================================================================

pub struct EvolutionaryPlayer {
    pub id: usize,
    pub generation: usize,
    pub strategy: Box<dyn Strategy>,
    pub kinship_group: usize,

    // Match state
    current_match_id: Option<usize>,
    current_opponent_id: Option<usize>,
    opponent_histories: HashMap<usize, Vec<RoundOutcome>>,

    // Fitness tracking
    pub fitness: f64,
    matches_played: usize,
    current_match_payoff: i32,
}

impl EvolutionaryPlayer {
    pub fn new(id: usize, generation: usize, strategy: Box<dyn Strategy>, kinship_group: usize) -> Self {
        EvolutionaryPlayer {
            id,
            generation,
            strategy,
            kinship_group,
            current_match_id: None,
            current_opponent_id: None,
            opponent_histories: HashMap::new(),
            fitness: 0.0,
            matches_played: 0,
            current_match_payoff: 0,
        }
    }

    pub fn strategy_name(&self) -> &'static str {
        self.strategy.name()
    }
}

impl des::Agent<KinshipEvent, EvolutionaryStats> for EvolutionaryPlayer {
    fn act(
        &mut self,
        current_t: usize,
        event: &KinshipEvent,
    ) -> des::Response<KinshipEvent, EvolutionaryStats> {
        // Only respond to events from our generation
        let event_generation = match event {
            KinshipEvent::GenerationStart { generation } => *generation,
            KinshipEvent::PlayMatch { .. } => self.generation, // Matches don't have generation, use current
            KinshipEvent::PlayRound { .. } => self.generation,
            KinshipEvent::RoundDecision { .. } => self.generation,
            KinshipEvent::RoundResult { .. } => self.generation,
            KinshipEvent::MatchComplete { .. } => self.generation,
            KinshipEvent::GenerationComplete { generation } => *generation,
            KinshipEvent::Reproduction { generation } => *generation,
            _ => return des::Response::new(),
        };

        if event_generation != self.generation {
            return des::Response::new();
        }

        match event {
            KinshipEvent::PlayMatch {
                agent_a_id,
                agent_b_id,
                rounds,
            } => {
                // Only respond if this agent is involved
                if *agent_a_id == self.id || *agent_b_id == self.id {
                    let match_id = (*agent_a_id << 16) | *agent_b_id; // Create unique match ID
                    self.current_match_id = Some(match_id);
                    self.current_match_payoff = 0;

                    let opponent_id = if *agent_a_id == self.id {
                        *agent_b_id
                    } else {
                        *agent_a_id
                    };
                    self.current_opponent_id = Some(opponent_id);

                    // Start first round
                    des::Response::event(
                        current_t + 1,
                        KinshipEvent::PlayRound {
                            match_id,
                            agent_a_id: *agent_a_id,
                            agent_b_id: *agent_b_id,
                            round_num: 1,
                        },
                    )
                } else {
                    des::Response::new()
                }
            }

            KinshipEvent::PlayRound {
                match_id,
                agent_a_id,
                agent_b_id,
                round_num,
            } => {
                if Some(*match_id) == self.current_match_id {
                    let opponent_id = self.current_opponent_id.unwrap();

                    // Get history with this opponent
                    let history = self
                        .opponent_histories
                        .get(&opponent_id)
                        .map(|h| h.as_slice())
                        .unwrap_or(&[]);

                    // Make decision based on strategy
                    let choice = self.strategy.decide(history);

                    des::Response::event(
                        current_t + 1,
                        KinshipEvent::RoundDecision {
                            match_id: *match_id,
                            agent_id: self.id,
                            choice,
                        },
                    )
                } else {
                    des::Response::new()
                }
            }

            KinshipEvent::RoundResult {
                match_id,
                round_num,
                agent_a_id,
                agent_b_id,
                agent_a_choice,
                agent_b_choice,
            } => {
                if Some(*match_id) == self.current_match_id {
                    let (my_choice, opponent_choice) = if *agent_a_id == self.id {
                        (*agent_a_choice, *agent_b_choice)
                    } else {
                        (*agent_b_choice, *agent_a_choice)
                    };

                    let opponent_id = self.current_opponent_id.unwrap();

                    // Calculate payoff
                    let payoff = crate::calculate_payoff(my_choice, opponent_choice);
                    self.current_match_payoff += payoff;

                    // Store in history
                    self.opponent_histories
                        .entry(opponent_id)
                        .or_insert_with(Vec::new)
                        .push(RoundOutcome {
                            round_number: *round_num,
                            my_choice,
                            opponent_choice,
                            my_payoff: payoff,
                        });
                }
                des::Response::new()
            }

            KinshipEvent::MatchComplete {
                match_id,
                agent_a_id,
                agent_b_id,
                agent_a_payoff: _,
                agent_b_payoff: _,
            } => {
                if Some(*match_id) == self.current_match_id {
                    // Add match payoff to fitness
                    self.fitness += self.current_match_payoff as f64;
                    self.matches_played += 1;

                    // Clear match state
                    self.current_match_id = None;
                    self.current_match_payoff = 0;

                    // Don't clear opponent_id - keep for potential future matches
                }
                des::Response::new()
            }

            _ => des::Response::new(),
        }
    }

    fn stats(&self) -> EvolutionaryStats {
        EvolutionaryStats::Agent(AgentStats {
            agent_id: self.id,
            generation: self.generation,
            kinship_group: self.kinship_group,
            strategy_name: self.strategy_name(),
            fitness: self.fitness,
            matches_played: self.matches_played,
        })
    }
}

// ============================================================================
// Population Coordinator
// ============================================================================

struct MatchState {
    agent_a_id: usize,
    agent_b_id: usize,
    rounds_remaining: usize,
    agent_a_payoff: i32,
    agent_b_payoff: i32,
    agent_a_choice: Option<Choice>,
    agent_b_choice: Option<Choice>,
    current_round: usize,
}

#[derive(Clone, Debug)]
struct AgentData {
    id: usize,
    strategy_name: String,
    kinship_group: usize,
    fitness: f64,
}

pub struct PopulationCoordinator {
    // Population tracking
    agent_registry: HashMap<usize, AgentData>, // id -> agent data
    agent_fitness: HashMap<usize, f64>, // Track fitness updates

    // Simulation parameters
    population_size: usize,
    encounters_per_generation: usize,
    rounds_per_match: usize,
    kinship_preference: f64, // Probability of matching within kinship group
    mutation_rate: f64, // Probability of strategy mutation

    // Current state
    current_generation: usize,
    next_agent_id: usize,
    encounters_completed: usize,
    active_matches: HashMap<usize, MatchState>,

    // Statistics
    total_matches: usize,
    max_generations: usize,
}

impl PopulationCoordinator {
    pub fn new(
        population_size: usize,
        encounters_per_generation: usize,
        rounds_per_match: usize,
        kinship_preference: f64,
        mutation_rate: f64,
        max_generations: usize,
    ) -> Self {
        PopulationCoordinator {
            agent_registry: HashMap::new(),
            agent_fitness: HashMap::new(),
            population_size,
            encounters_per_generation,
            rounds_per_match,
            kinship_preference,
            mutation_rate,
            current_generation: 0,
            next_agent_id: 0,
            encounters_completed: 0,
            active_matches: HashMap::new(),
            total_matches: 0,
            max_generations,
        }
    }

    pub fn register_agent(&mut self, id: usize, strategy_name: String, kinship_group: usize) {
        self.agent_registry.insert(
            id,
            AgentData {
                id,
                strategy_name,
                kinship_group,
                fitness: 0.0,
            },
        );
        self.next_agent_id = self.next_agent_id.max(id + 1);
    }

    pub fn update_fitness(&mut self, agent_id: usize, fitness: f64) {
        self.agent_fitness.insert(agent_id, fitness);
        if let Some(agent) = self.agent_registry.get_mut(&agent_id) {
            agent.fitness = fitness;
        }
    }

    fn fitness_proportional_selection(&self) -> Vec<AgentData> {
        let mut rng = rand::rng();
        let mut new_population = Vec::new();

        // Get current generation agents with their actual fitness
        let agents: Vec<&AgentData> = self.agent_registry.values().collect();

        if agents.is_empty() {
            return new_population;
        }

        // Calculate total fitness using the accumulated agent_fitness HashMap
        // Use a minimum of 0.1 to avoid division by zero and give all agents some chance
        let total_fitness: f64 = agents
            .iter()
            .map(|a| self.agent_fitness.get(&a.id).copied().unwrap_or(0.0).max(0.1))
            .sum();

        // Create new population via roulette wheel selection
        for _ in 0..self.population_size {
            let mut spin = rng.random::<f64>() * total_fitness;
            let mut selected = agents[0];

            for agent in &agents {
                let agent_fitness = self.agent_fitness.get(&agent.id).copied().unwrap_or(0.0).max(0.1);
                spin -= agent_fitness;
                if spin <= 0.0 {
                    selected = agent;
                    break;
                }
            }

            // Clone the agent data (with potential mutation)
            let mut new_strategy = selected.strategy_name.clone();
            if rng.random::<f64>() < self.mutation_rate {
                // Mutation: flip strategy
                new_strategy = if new_strategy == "TitForTat" {
                    "AlwaysDefect".to_string()
                } else {
                    "TitForTat".to_string()
                };
            }

            new_population.push(AgentData {
                id: self.next_agent_id + new_population.len(),
                strategy_name: new_strategy,
                kinship_group: selected.kinship_group,
                fitness: 0.0, // Reset fitness for new generation
            });
        }

        new_population
    }

    fn select_match_pair(&self) -> Option<(usize, usize)> {
        let agents: Vec<(usize, usize)> = self
            .agent_registry
            .values()
            .map(|agent| (agent.id, agent.kinship_group))
            .collect();

        if agents.len() < 2 {
            return None;
        }

        let mut rng = rand::rng();

        // Pick first agent randomly
        let idx_a = rng.random_range(0..agents.len());
        let (agent_a, group_a) = agents[idx_a];

        // Pick second agent with kinship preference
        let use_kinship = rng.random::<f64>() < self.kinship_preference;

        let agent_b = if use_kinship {
            // Try to find someone in same kinship group
            let same_group: Vec<usize> = agents
                .iter()
                .filter(|(id, g)| *id != agent_a && *g == group_a)
                .map(|(id, _)| *id)
                .collect();

            if !same_group.is_empty() {
                same_group[rng.random_range(0..same_group.len())]
            } else {
                // No one in same group, pick anyone else
                let others: Vec<usize> = agents
                    .iter()
                    .filter(|(id, _)| *id != agent_a)
                    .map(|(id, _)| *id)
                    .collect();
                others[rng.random_range(0..others.len())]
            }
        } else {
            // Pick anyone else
            let others: Vec<usize> = agents
                .iter()
                .filter(|(id, _)| *id != agent_a)
                .map(|(id, _)| *id)
                .collect();
            others[rng.random_range(0..others.len())]
        };

        Some((agent_a, agent_b))
    }
}

impl des::Agent<KinshipEvent, EvolutionaryStats> for PopulationCoordinator {
    fn act(
        &mut self,
        current_t: usize,
        event: &KinshipEvent,
    ) -> des::Response<KinshipEvent, EvolutionaryStats> {
        match event {
            KinshipEvent::GenerationStart { generation } => {
                self.current_generation = *generation;
                self.encounters_completed = 0;

                // Start first encounter
                if let Some((a, b)) = self.select_match_pair() {
                    des::Response::event(
                        current_t + 1,
                        KinshipEvent::PlayMatch {
                            agent_a_id: a,
                            agent_b_id: b,
                            rounds: self.rounds_per_match,
                        },
                    )
                } else {
                    des::Response::new()
                }
            }

            KinshipEvent::RoundDecision {
                match_id,
                agent_id,
                choice,
            } => {
                if let Some(match_state) = self.active_matches.get_mut(match_id) {
                    // Record choice
                    if *agent_id == match_state.agent_a_id {
                        match_state.agent_a_choice = Some(*choice);
                    } else if *agent_id == match_state.agent_b_id {
                        match_state.agent_b_choice = Some(*choice);
                    }

                    // If both choices made, process round
                    if let (Some(a_choice), Some(b_choice)) =
                        (match_state.agent_a_choice, match_state.agent_b_choice)
                    {
                        // Calculate payoffs
                        let a_payoff = crate::calculate_payoff(a_choice, b_choice);
                        let b_payoff = crate::calculate_payoff(b_choice, a_choice);

                        match_state.agent_a_payoff += a_payoff;
                        match_state.agent_b_payoff += b_payoff;

                        // Emit round result
                        let round_num = match_state.current_round;
                        let event: des::Response<KinshipEvent, EvolutionaryStats> = des::Response::event(
                            current_t + 1,
                            KinshipEvent::RoundResult {
                                match_id: *match_id,
                                round_num,
                                agent_a_id: match_state.agent_a_id,
                                agent_b_id: match_state.agent_b_id,
                                agent_a_choice: a_choice,
                                agent_b_choice: b_choice,
                            },
                        );

                        // Check if more rounds needed
                        match_state.rounds_remaining -= 1;
                        match_state.current_round += 1;
                        match_state.agent_a_choice = None;
                        match_state.agent_b_choice = None;

                        if match_state.rounds_remaining > 0 {
                            // Continue to next round
                            return des::Response {
                                events: vec![
                                    event.events[0].clone(),
                                    (
                                        current_t + 2,
                                        KinshipEvent::PlayRound {
                                            match_id: *match_id,
                                            agent_a_id: match_state.agent_a_id,
                                            agent_b_id: match_state.agent_b_id,
                                            round_num: match_state.current_round,
                                        },
                                    ),
                                ],
                                agents: vec![],
                            };
                        } else {
                            // Match complete
                            return des::Response {
                                events: vec![
                                    event.events[0].clone(),
                                    (
                                        current_t + 2,
                                        KinshipEvent::MatchComplete {
                                            match_id: *match_id,
                                            agent_a_id: match_state.agent_a_id,
                                            agent_b_id: match_state.agent_b_id,
                                            agent_a_payoff: match_state.agent_a_payoff,
                                            agent_b_payoff: match_state.agent_b_payoff,
                                        },
                                    ),
                                ],
                                agents: vec![],
                            };
                        }
                    }
                }
                des::Response::new()
            }

            KinshipEvent::PlayMatch {
                agent_a_id,
                agent_b_id,
                rounds,
            } => {
                // Create match state
                let match_id = (*agent_a_id << 16) | *agent_b_id;
                self.active_matches.insert(
                    match_id,
                    MatchState {
                        agent_a_id: *agent_a_id,
                        agent_b_id: *agent_b_id,
                        rounds_remaining: *rounds,
                        agent_a_payoff: 0,
                        agent_b_payoff: 0,
                        agent_a_choice: None,
                        agent_b_choice: None,
                        current_round: 1,
                    },
                );

                self.total_matches += 1;

                // Event will be handled by agents
                des::Response::new()
            }

            KinshipEvent::MatchComplete {
                match_id,
                agent_a_id,
                agent_b_id,
                agent_a_payoff,
                agent_b_payoff,
            } => {
                self.active_matches.remove(match_id);
                self.encounters_completed += 1;

                // Accumulate fitness for both agents
                *self.agent_fitness.entry(*agent_a_id).or_insert(0.0) += *agent_a_payoff as f64;
                *self.agent_fitness.entry(*agent_b_id).or_insert(0.0) += *agent_b_payoff as f64;

                // Also update the registry for stats tracking
                if let Some(agent_a) = self.agent_registry.get_mut(agent_a_id) {
                    agent_a.fitness += *agent_a_payoff as f64;
                }
                if let Some(agent_b) = self.agent_registry.get_mut(agent_b_id) {
                    agent_b.fitness += *agent_b_payoff as f64;
                }

                // Check if generation complete
                if self.encounters_completed >= self.encounters_per_generation {
                    des::Response::event(
                        current_t + 1,
                        KinshipEvent::GenerationComplete {
                            generation: self.current_generation,
                        },
                    )
                } else {
                    // Start next encounter
                    if let Some((a, b)) = self.select_match_pair() {
                        des::Response::event(
                            current_t + 1,
                            KinshipEvent::PlayMatch {
                                agent_a_id: a,
                                agent_b_id: b,
                                rounds: self.rounds_per_match,
                            },
                        )
                    } else {
                        des::Response::new()
                    }
                }
            }

            KinshipEvent::GenerationComplete { generation } => {
                // Trigger reproduction
                des::Response::event(
                    current_t + 1,
                    KinshipEvent::Reproduction {
                        generation: *generation,
                    },
                )
            }

            KinshipEvent::Reproduction { generation } => {
                // Collect fitness from agents and create new generation
                let new_population = self.fitness_proportional_selection();

                // Clear old registry and prepare for new generation
                self.agent_registry.clear();
                self.agent_fitness.clear();

                // Create new agents
                let new_generation = generation + 1;
                let mut new_agents: Vec<Box<dyn des::Agent<KinshipEvent, EvolutionaryStats>>> =
                    Vec::new();

                for agent_data in new_population {
                    // Register new agent
                    self.agent_registry.insert(agent_data.id, agent_data.clone());

                    // Create strategy
                    let strategy: Box<dyn Strategy> = if agent_data.strategy_name == "TitForTat" {
                        Box::new(crate::TitForTat)
                    } else {
                        Box::new(crate::AlwaysDefect)
                    };

                    // Create new player
                    new_agents.push(Box::new(EvolutionaryPlayer::new(
                        agent_data.id,
                        new_generation,
                        strategy,
                        agent_data.kinship_group,
                    )));
                }

                self.next_agent_id += new_agents.len();
                self.current_generation = new_generation;
                self.encounters_completed = 0;

                // Check if we should continue
                if new_generation <= self.max_generations {
                    // Start next generation
                    des::Response {
                        events: vec![(
                            current_t + 1,
                            KinshipEvent::GenerationStart {
                                generation: new_generation,
                            },
                        )],
                        agents: new_agents,
                    }
                } else {
                    // Simulation complete, just spawn the new agents for stats
                    des::Response {
                        events: vec![],
                        agents: new_agents,
                    }
                }
            }

            _ => des::Response::new(),
        }
    }

    fn stats(&self) -> EvolutionaryStats {
        EvolutionaryStats::Population(PopulationStats {
            generation: self.current_generation,
            total_agents: self.agent_registry.len(),
            tft_count: 0, // Will be filled from agent stats
            defector_count: 0,
            tft_percentage: 0.0,
            avg_tft_fitness: 0.0,
            avg_defector_fitness: 0.0,
            total_matches: self.total_matches,
        })
    }
}
