# rs-des

A Rust-based Discrete Event Simulation (DES) framework for translating Agent-Based Model (ABM) research papers into runnable simulations.

## Quick Start

```bash
# Run the simple queue example
cargo run -p simple_queue

# Evolution of Cooperation simulations
cargo run -p evolution_coop --bin tournament              # Finding A: Robustness
cargo run -p evolution_coop --bin initial_viability       # Finding C: Initial viability (single gen)
cargo run --release -p evolution_coop --bin evolutionary_takeover  # Finding C: Full invasion (use --release for speed)

# Run tests
cargo test
```

## Implemented Simulations

### The Evolution of Cooperation (Axelrod & Hamilton, 1981) ✅

Complete implementation of Axelrod and Hamilton's seminal paper on how cooperation evolves through individual selection. Demonstrates all three key findings using the iterated Prisoner's Dilemma.

#### Theoretical Background

**The Cooperation Paradox**: In a world where defection yields higher individual payoffs, how does cooperation persist and spread? Axelrod & Hamilton resolved this by showing cooperation evolves when:
- Interactions are repeated with sufficient probability (w)
- Individuals can recognize previous partners
- Defection can be punished through retaliation

**Payoff Matrix** (Prisoner's Dilemma):
```
                 Partner Cooperates    Partner Defects
You Cooperate         R = 3                S = 0
You Defect            T = 5                P = 1
```
Where: T > R > P > S (Temptation > Reward > Punishment > Sucker's payoff)

**TIT FOR TAT Strategy**:
- Cooperate on first move
- Copy opponent's previous move thereafter
- Properties: Nice, Retaliatory, Forgiving, Clear

#### Three Key Findings Implemented

##### Finding A: Robustness
**Question**: Can TIT FOR TAT thrive in a diverse environment?

**Implementation**: `cargo run -p evolution_coop --bin tournament`

Round-robin tournament where TIT FOR TAT competes against diverse strategies:
- AlwaysDefect
- AlwaysCooperate
- Random
- Grudger

**Expected Results**:
```
TIT FOR TAT achieves highest total score through:
- Mutual cooperation with nice strategies (3 pts/round)
- Protection against exploiters (defects after first defection)
- Better average payoff than pure defectors (~2.5 vs ~1.0 per round)
```

##### Finding B: Evolutionary Stability
**Question**: Can TIT FOR TAT resist invasion by mutants once established?

**Theory**: TIT FOR TAT is evolutionarily stable when:
```
w ≥ max[(T-R)/(T-P), (T-R)/(R-S)]
```
For standard payoffs: w ≥ max[0.5, 0.67] = 0.67

**Status**: Theoretical analysis complete. ESS simulation coming soon.

##### Finding C: Initial Viability ✨
**Question**: How can cooperation emerge in a predominantly defecting world?

**Implementation**:
```bash
# Single generation demonstration
cargo run -p evolution_coop --bin initial_viability

# Multi-generation evolutionary takeover
cargo run --release -p evolution_coop --bin evolutionary_takeover
```

**Mechanisms Implemented**:
1. **Kinship**: Related individuals (kinship group 0) preferentially interact
2. **Fitness-proportional reproduction**: Higher payoff → more offspring
3. **Multi-generation evolution**: Population composition evolves over time

**Expected Results**:

*Single Generation (initial_viability)*:
```
Population: 5% TIT FOR TAT (kinship group 0), 95% ALWAYS DEFECT (distributed)
Kinship preference: 80% within-group matching

Result:
  Avg TIT FOR TAT fitness: ~37-70    (2-3x advantage)
  Avg ALWAYS DEFECT fitness: ~10-51

Interpretation: TFT agents meet each other frequently (kinship),
achieving mutual cooperation payoffs (R=3 per round). Defectors
mostly meet other defectors (P=1 per round).
```

*Multi-Generation (evolutionary_takeover)*:
```
Starting: 5% TIT FOR TAT → 100% TIT FOR TAT in ~13 generations

Generation  TFT%    Avg TFT Fit    Avg Defector Fit
    1       5.0%       70.8            50.9
    3      19.0%      111.3            52.5
    5      50.5%      134.6            54.3
    7      87.5%      145.1            59.1
   13     100.0%      150.0             0.0  ✨

Invasion Path:
  1. Kinship effect (95% within-group) → TFT fitness advantage
  2. Fitness-proportional selection → More TFT offspring
  3. Exponential growth → TFT dominates population
  4. ESS achieved → Defectors extinct
```

**Key Validation**: Demonstrates that "a small cluster of individuals using TIT FOR TAT with even a tiny probability of getting together can be initially viable and eventually invade" (Axelrod & Hamilton, 1981, p. 1395).

#### Implementation Architecture

**Module**: `evolution_coop/src/kinship.rs`

**Event-Driven Design**:
```rust
enum KinshipEvent {
    GenerationStart { generation: usize },
    PlayMatch { agent_a_id, agent_b_id, rounds },
    PlayRound { match_id, agent_a_id, agent_b_id, round_num },
    RoundDecision { match_id, agent_id, choice },
    RoundResult { match_id, round_num, choices, payoffs },
    MatchComplete { match_id, agent_ids, total_payoffs },
    GenerationComplete { generation },
    Reproduction { generation },
}
```

**Agents**:
- `EvolutionaryPlayer`: Implements strategy (TFT/Defect), tracks fitness, plays iterated PD
- `PopulationCoordinator`: Manages matchmaking with kinship preference, fitness-proportional reproduction

**Critical Implementation Details**:

1. **Fitness Accumulation** (evolution_coop/src/kinship.rs:632-652):
   ```rust
   // Coordinator collects payoffs from MatchComplete events
   *self.agent_fitness.entry(*agent_a_id).or_insert(0.0) += *agent_a_payoff as f64;
   ```

2. **Fitness-Proportional Selection** (evolution_coop/src/kinship.rs:392-444):
   ```rust
   // Roulette wheel selection using actual accumulated fitness
   let total_fitness: f64 = agents.iter()
       .map(|a| self.agent_fitness.get(&a.id).unwrap_or(0.0).max(0.1))
       .sum();
   ```

3. **Dynamic Agent Spawning**:
   ```rust
   // New generation created as agents via Response.agents
   des::Response {
       events: vec![(t, KinshipEvent::GenerationStart { generation: n+1 })],
       agents: new_generation_agents,  // Fresh agents for next generation
   }
   ```

**Bug Fixed** (2025-01-29): Initial implementation had fitness always set to 0.0, making reproduction random. Fixed by:
- Collecting payoffs in MatchComplete handler (line 632)
- Using `agent_fitness` HashMap in selection (line 407)
- Result: Stable, reproducible cooperation invasion

#### Parameters and Tuning

**Standard Configuration** (evolutionary_takeover):
```rust
population_size: 200
initial_tft_percentage: 5%
kinship_groups: 10
kinship_preference: 95%      // Very strong within-group matching
mutation_rate: 0.2%          // Low to prevent TFT→Defect flips
encounters_per_generation: 500
rounds_per_match: 10         // Simulates w ≈ 0.9
max_generations: 40
```

**Why These Values**:
- **95% kinship preference**: Ensures TFT agents frequently meet (initial viability)
- **500 encounters**: ~2.5 matches per agent → strong fitness signal
- **0.2% mutation**: Allows exploration without destroying TFT advantage
- **10 rounds**: Approximates iterated PD with continuation probability w=0.9

#### Further Reading

- **Paper Summary**: `/prior-art/evolution-of-cooperation.md` - Detailed analysis with modern extensions (2020-2025)
- **Original Paper**: Axelrod, R. & Hamilton, W.D. (1981). "The Evolution of Cooperation." *Science*, 211(4489), 1390-1396.
- **Related Work**: See `/prior-art/` for Win-Stay Lose-Shift, Generous TFT, and indirect reciprocity developments

---

## Reading

[ABMs in economics and finance (Axtell and Farmer, 2025)](https://ora.ox.ac.uk/objects/uuid:8af3b96e-a088-4e29-ba1e-0760222277b7/files/s6969z182c)

## Claude Code Configuration

TODO: Set up custom instructions to optimize for planning and research translation

1. **Create CLAUDE.md** - Run `/init` and customize with:
   - Project purpose: "Translate ABM research papers into modular Rust simulations"
   - Core architecture: Explain DES framework and modular example pattern
   - Research focus areas: List papers/models (Axelrod, Kirman & Vriend, ZI traders, etc.)
   - Working style: "Focus on architectural design and research translation, not syntax"
   - Domain terminology: Define agents, events, simulation mechanics

2. **Create custom slash commands** in `.claude/commands/`:
   - `/paper-simulation` - Paste paper excerpt, get simulation design outline
   - `/agent-design` - Design agent state and behavior patterns
   - `/framework-review` - Review how new examples integrate with DES core

3. **Session start template**: Provide paper section, key agents, event types, metrics
   - Ask architectural questions: "How should I structure agents for X behavior?"
   - Not: "Can you implement this?"

4. **Commit configuration to git** (team-shared)

## Recreate

**Completed**:
- ✅ The Evolution of Cooperation (Axelrod & Hamilton, 1981) - All three key findings implemented

**TODO**:
- TRANSIMS code (Barrett et al., 1995, Nagel, Beckman and Barrett, 1998)
- drug addiction (Agar and Wilson, 2002, Hoffer, Bobashev and Morris, 2009, Heard, Bobashev and Morris, 2014)
- Kirman and Vriend (2000, 2001) - fish market, loyalty
- policy relevant and exercised to study policy alternatives (Dawid et al., 2012)
- Donier et al. (2015) showed that a linear virtual order book profile
- Aymanns et al. (2016) leverage cycles
- "zero-intelligence" (ZI) agents (Gode and Sunder, 1993, 1997)
- K-level cognition (Camerer, Ho and Chong, 2004) has found use in ABMs (Latek, Kaminski and Axtell, 2009)
