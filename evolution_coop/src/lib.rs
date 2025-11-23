use std::collections::HashMap;

// ============================================================================
// Core Domain Types
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Cooperate,
    Defect,
}

#[derive(Clone, Debug)]
pub enum Event {
    // Tournament lifecycle
    TournamentStart,

    // Round orchestration
    RoundStart {
        match_id: usize,
        player_a_id: usize,
        player_b_id: usize,
        round_number: usize,
    },

    // Player decisions
    DecisionMade {
        match_id: usize,
        player_id: usize,
        choice: Choice,
    },

    // Round completion
    RoundComplete {
        match_id: usize,
        round_number: usize,
        player_a_id: usize,
        player_b_id: usize,
        player_a_choice: Choice,
        player_b_choice: Choice,
    },

    // Match completion
    MatchComplete {
        match_id: usize,
        player_a_id: usize,
        player_b_id: usize,
    },

    // Tournament completion
    TournamentComplete,
}

// ============================================================================
// Statistics Types
// ============================================================================

#[derive(Clone, Debug)]
pub struct TournamentStats {
    pub completed_matches: usize,
    pub active_matches: usize,
}

#[derive(Clone, Debug)]
pub struct PlayerStats {
    pub player_id: usize,
    pub strategy_name: &'static str,
    pub total_score: i32,
    pub rounds_played: usize,
    pub cooperations: usize,
    pub defections: usize,
}

#[derive(Clone, Debug)]
pub enum Stats {
    Tournament(TournamentStats),
    Player(PlayerStats),
}

// ============================================================================
// Payoff Calculation
// ============================================================================

pub fn calculate_payoff(my_choice: Choice, opponent_choice: Choice) -> i32 {
    match (my_choice, opponent_choice) {
        (Choice::Cooperate, Choice::Cooperate) => 3,  // Reward
        (Choice::Defect, Choice::Cooperate) => 5,      // Temptation
        (Choice::Cooperate, Choice::Defect) => 0,      // Sucker
        (Choice::Defect, Choice::Defect) => 1,         // Punishment
    }
}

// ============================================================================
// Round Outcome (for player history tracking)
// ============================================================================

#[derive(Clone, Debug)]
pub struct RoundOutcome {
    pub round_number: usize,
    pub my_choice: Choice,
    pub opponent_choice: Choice,
    pub my_payoff: i32,
}

#[derive(Clone, Debug)]
pub struct OpponentHistory {
    pub opponent_id: usize,
    pub rounds: Vec<RoundOutcome>,
}

// ============================================================================
// TournamentCoordinator Agent
// ============================================================================

struct RoundState {
    round_number: usize,
    player_a_choice: Option<Choice>,
    player_b_choice: Option<Choice>,
}

struct MatchState {
    match_id: usize,
    player_a_id: usize,
    player_b_id: usize,
    current_round: usize,
    round_state: RoundState,
}

pub struct TournamentCoordinator {
    num_rounds_per_match: usize,
    num_players: usize,

    // Match orchestration
    next_match_id: usize,
    pending_matches: Vec<(usize, usize)>,
    current_matches: HashMap<usize, MatchState>,

    // Statistics
    completed_matches: usize,
}

impl TournamentCoordinator {
    pub fn new(num_players: usize, num_rounds_per_match: usize) -> Self {
        // Generate round-robin pairings
        let mut pending_matches = Vec::new();
        for i in 0..num_players {
            for j in (i + 1)..num_players {
                pending_matches.push((i, j));
            }
        }

        TournamentCoordinator {
            num_rounds_per_match,
            num_players,
            next_match_id: 0,
            pending_matches,
            current_matches: HashMap::new(),
            completed_matches: 0,
        }
    }

    fn start_match(&mut self, current_t: usize) -> des::Response<Event, Stats> {
        if let Some((player_a_id, player_b_id)) = self.pending_matches.pop() {
            let match_id = self.next_match_id;
            self.next_match_id += 1;

            self.current_matches.insert(
                match_id,
                MatchState {
                    match_id,
                    player_a_id,
                    player_b_id,
                    current_round: 1,
                    round_state: RoundState {
                        round_number: 1,
                        player_a_choice: None,
                        player_b_choice: None,
                    },
                },
            );

            des::Response::event(
                current_t + 1,
                Event::RoundStart {
                    match_id,
                    player_a_id,
                    player_b_id,
                    round_number: 1,
                },
            )
        } else {
            des::Response::event(current_t + 1, Event::TournamentComplete)
        }
    }
}

impl des::Agent<Event, Stats> for TournamentCoordinator {
    fn act(&mut self, current_t: usize, event: &Event) -> des::Response<Event, Stats> {
        match event {
            Event::TournamentStart => self.start_match(current_t),

            Event::DecisionMade {
                match_id,
                player_id,
                choice,
            } => {
                if let Some(match_state) = self.current_matches.get_mut(match_id) {
                    let round = &mut match_state.round_state;

                    // Update the appropriate player's choice
                    if *player_id == match_state.player_a_id {
                        round.player_a_choice = Some(*choice);
                    } else if *player_id == match_state.player_b_id {
                        round.player_b_choice = Some(*choice);
                    }

                    // Check if both players have decided
                    if let (Some(a_choice), Some(b_choice)) =
                        (round.player_a_choice, round.player_b_choice)
                    {
                        return des::Response::event(
                            current_t + 1,
                            Event::RoundComplete {
                                match_id: *match_id,
                                round_number: round.round_number,
                                player_a_id: match_state.player_a_id,
                                player_b_id: match_state.player_b_id,
                                player_a_choice: a_choice,
                                player_b_choice: b_choice,
                            },
                        );
                    }
                }
                des::Response::new()
            }

            Event::RoundComplete { match_id, .. } => {
                if let Some(match_state) = self.current_matches.get_mut(match_id) {
                    match_state.current_round += 1;

                    if match_state.current_round <= self.num_rounds_per_match {
                        // Start next round
                        match_state.round_state = RoundState {
                            round_number: match_state.current_round,
                            player_a_choice: None,
                            player_b_choice: None,
                        };

                        des::Response::event(
                            current_t + 1,
                            Event::RoundStart {
                                match_id: *match_id,
                                player_a_id: match_state.player_a_id,
                                player_b_id: match_state.player_b_id,
                                round_number: match_state.current_round,
                            },
                        )
                    } else {
                        // Match complete
                        des::Response::event(
                            current_t + 1,
                            Event::MatchComplete {
                                match_id: *match_id,
                                player_a_id: match_state.player_a_id,
                                player_b_id: match_state.player_b_id,
                            },
                        )
                    }
                } else {
                    des::Response::new()
                }
            }

            Event::MatchComplete { match_id, .. } => {
                self.current_matches.remove(match_id);
                self.completed_matches += 1;
                self.start_match(current_t)
            }

            _ => des::Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::Tournament(TournamentStats {
            completed_matches: self.completed_matches,
            active_matches: self.current_matches.len(),
        })
    }
}

// ============================================================================
// Strategy Trait
// ============================================================================

pub trait Strategy {
    fn decide(&self, opponent_history: &[RoundOutcome]) -> Choice;
    fn name(&self) -> &'static str;
}

// ============================================================================
// Player Agent
// ============================================================================

pub struct Player {
    id: usize,
    strategy: Box<dyn Strategy>,

    // Match state
    opponent_histories: HashMap<usize, OpponentHistory>,
    current_match_id: Option<usize>,
    current_opponent_id: Option<usize>,

    // Statistics
    total_score: i32,
    rounds_played: usize,
    cooperations: usize,
    defections: usize,
}

impl Player {
    pub fn new(id: usize, strategy: Box<dyn Strategy>) -> Self {
        Player {
            id,
            strategy,
            opponent_histories: HashMap::new(),
            current_match_id: None,
            current_opponent_id: None,
            total_score: 0,
            rounds_played: 0,
            cooperations: 0,
            defections: 0,
        }
    }
}

impl des::Agent<Event, Stats> for Player {
    fn act(&mut self, current_t: usize, event: &Event) -> des::Response<Event, Stats> {
        match event {
            Event::RoundStart {
                match_id,
                player_a_id,
                player_b_id,
                round_number: _,
            } => {
                // Only respond if this player is involved
                if *player_a_id == self.id || *player_b_id == self.id {
                    self.current_match_id = Some(*match_id);

                    let opponent_id = if *player_a_id == self.id {
                        *player_b_id
                    } else {
                        *player_a_id
                    };
                    self.current_opponent_id = Some(opponent_id);

                    // Get opponent history for strategy decision
                    let history = self
                        .opponent_histories
                        .get(&opponent_id)
                        .map(|h| h.rounds.as_slice())
                        .unwrap_or(&[]);

                    // Strategy decides based on actual history
                    let choice = self.strategy.decide(history);

                    // Track decision for stats
                    match choice {
                        Choice::Cooperate => self.cooperations += 1,
                        Choice::Defect => self.defections += 1,
                    }

                    des::Response::event(
                        current_t + 1,
                        Event::DecisionMade {
                            match_id: *match_id,
                            player_id: self.id,
                            choice,
                        },
                    )
                } else {
                    des::Response::new()
                }
            }

            Event::RoundComplete {
                match_id,
                round_number,
                player_a_id,
                player_b_id,
                player_a_choice,
                player_b_choice,
            } => {
                // Only process if this player is involved
                if Some(*match_id) == self.current_match_id {
                    let (my_choice, opponent_choice, opponent_id) = if *player_a_id == self.id {
                        (*player_a_choice, *player_b_choice, *player_b_id)
                    } else {
                        (*player_b_choice, *player_a_choice, *player_a_id)
                    };

                    // Calculate payoff
                    let my_payoff = calculate_payoff(my_choice, opponent_choice);

                    // Update structured history
                    self.opponent_histories
                        .entry(opponent_id)
                        .or_insert_with(|| OpponentHistory {
                            opponent_id,
                            rounds: Vec::new(),
                        })
                        .rounds
                        .push(RoundOutcome {
                            round_number: *round_number,
                            my_choice,
                            opponent_choice,
                            my_payoff,
                        });

                    self.total_score += my_payoff;
                    self.rounds_played += 1;
                }

                des::Response::new()
            }

            Event::MatchComplete { match_id, .. } => {
                if Some(*match_id) == self.current_match_id {
                    self.current_match_id = None;
                    self.current_opponent_id = None;
                }
                des::Response::new()
            }

            _ => des::Response::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats::Player(PlayerStats {
            player_id: self.id,
            strategy_name: self.strategy.name(),
            total_score: self.total_score,
            rounds_played: self.rounds_played,
            cooperations: self.cooperations,
            defections: self.defections,
        })
    }
}

// ============================================================================
// Strategy Implementations
// ============================================================================

/// TIT FOR TAT: Cooperate on first move, then copy opponent's last move
pub struct TitForTat;

impl Strategy for TitForTat {
    fn decide(&self, opponent_history: &[RoundOutcome]) -> Choice {
        if opponent_history.is_empty() {
            Choice::Cooperate // Nice: cooperate first
        } else {
            // Copy opponent's last choice
            opponent_history.last().unwrap().opponent_choice
        }
    }

    fn name(&self) -> &'static str {
        "TitForTat"
    }
}

/// ALWAYS DEFECT: Always defect
pub struct AlwaysDefect;

impl Strategy for AlwaysDefect {
    fn decide(&self, _opponent_history: &[RoundOutcome]) -> Choice {
        Choice::Defect
    }

    fn name(&self) -> &'static str {
        "AlwaysDefect"
    }
}

/// ALWAYS COOPERATE: Always cooperate
pub struct AlwaysCooperate;

impl Strategy for AlwaysCooperate {
    fn decide(&self, _opponent_history: &[RoundOutcome]) -> Choice {
        Choice::Cooperate
    }

    fn name(&self) -> &'static str {
        "AlwaysCooperate"
    }
}

/// RANDOM: Cooperate or defect randomly (50/50)
pub struct Random;

impl Strategy for Random {
    fn decide(&self, _opponent_history: &[RoundOutcome]) -> Choice {
        if rand::random::<bool>() {
            Choice::Cooperate
        } else {
            Choice::Defect
        }
    }

    fn name(&self) -> &'static str {
        "Random"
    }
}

/// GRUDGER: Cooperate until opponent defects once, then always defect against that opponent
pub struct Grudger;

impl Strategy for Grudger {
    fn decide(&self, opponent_history: &[RoundOutcome]) -> Choice {
        // Check if THIS opponent has ever defected
        if opponent_history
            .iter()
            .any(|r| r.opponent_choice == Choice::Defect)
        {
            Choice::Defect
        } else {
            Choice::Cooperate
        }
    }

    fn name(&self) -> &'static str {
        "Grudger"
    }
}
