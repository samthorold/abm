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

# Evolving Market Structure (Kirman & Vriend, 2001)
cargo run -p evolving_market                              # Price dispersion and loyalty emergence

# Zero-Intelligence Traders (Gode & Sunder, 1993)
cargo run -p zi_traders                                   # Allocative efficiency comparison (ZI-U vs ZI-C)
cargo run -p zi_traders --bin iteration_experiment        # Test iteration count sensitivity

# Basel Leverage Cycle (Aymanns et al., 2016)
cargo run -p leverage_cycle                               # Four scenarios: micro/macro × deterministic/stochastic
cargo run -p leverage_cycle --example verify_paper_results  # Verify key paper findings

# Run tests
cargo test
```

## Development Setup

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality before commits. The hooks mirror the checks run in CI (GitHub Actions):

1. **Install pre-commit** (requires Python):
   ```bash
   pip install pre-commit
   ```

2. **Install the git hooks**:
   ```bash
   pre-commit install
   ```

3. **Run manually** (optional):
   ```bash
   # Run on all files
   pre-commit run --all-files

   # Run on staged files only
   pre-commit run
   ```

4. **Skip hooks** (when needed):
   ```bash
   git commit --no-verify
   ```

**What the hooks check:**
- `cargo fmt --all -- --check` - Code formatting
- `cargo clippy --all-targets --all-features -- -D warnings` - Lints
- `cargo test --all-features --workspace` - Test suite
- `cargo build --all-features --workspace` - Build check

**Note**: The test and build hooks may take a few seconds. For quick commits during iteration, you can skip them with `--no-verify` and rely on CI to catch issues.

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
Starting: 5% TIT FOR TAT → Variable outcome (typically 60-100% by generation 50)

Typical run with moderate parameters (80% kinship, 100 encounters/gen):
Generation  TFT%    Avg TFT Fit    Avg Defector Fit
    1       5.0%       25-40           10-15
    5       8-15%      30-45           10-15
   10      15-30%      35-50           10-15
   20      30-60%      40-55           10-15
   30      45-80%      42-58           10-15
   50      60-100%     45-60            8-12  ✨

Invasion Path:
  1. Kinship effect (80% within-group) → TFT fitness advantage
  2. Fitness-proportional selection → More TFT offspring
  3. Gradual growth → TFT increases each generation
  4. Eventual dominance → Cooperation spreads through population

Note: With moderate parameters, outcomes vary due to stochastic effects
(mutation, random matching). This demonstrates robustness of the mechanism
across different evolutionary trajectories.
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
kinship_preference: 80%      // Realistic kinship effect
mutation_rate: 1%            // Balanced exploration vs stability
encounters_per_generation: 100
rounds_per_match: 10         // Simulates w ≈ 0.9
max_generations: 50
```

**Why These Values**:
- **80% kinship preference**: Strong but realistic within-group matching (models family/tribe structures)
- **100 encounters**: ~0.5 matches per agent per generation (realistic social interaction frequency)
- **1% mutation**: Balances genetic exploration with selection pressure
- **10 rounds**: Approximates iterated PD with continuation probability w=0.9
- **50 generations**: Allows observation of full invasion dynamics at realistic pace

#### Further Reading

- **Paper Summary**: `/prior-art/evolution-of-cooperation.md` - Detailed analysis with modern extensions (2020-2025)
- **Original Paper**: Axelrod, R. & Hamilton, W.D. (1981). "The Evolution of Cooperation." *Science*, 211(4489), 1390-1396.
- **Related Work**: See `/prior-art/` for Win-Stay Lose-Shift, Generous TFT, and indirect reciprocity developments

---

### Evolving Market Structure (Kirman & Vriend, 2001) ✅

Implementation of Kirman & Vriend's agent-based computational economics (ACE) model demonstrating how price dispersion and buyer-seller loyalty emerge through reinforcement learning without central coordination.

#### Theoretical Background

**The Market Puzzle**: Real fish markets exhibit persistent price dispersion (same good, different prices) and buyer loyalty despite:
- Perfect information (all buyers see all prices)
- No product differentiation (homogeneous goods)
- No switching costs
- No barriers to entry/exit

**Classical Theory Prediction**: Bertrand competition → single price at marginal cost
**Empirical Reality**: 15% price dispersion coefficient, stable loyalty patterns

**Core Insight**: Loyalty and preferential treatment co-evolve through mutual reinforcement:

1. **Buyers learn loyalty** → Loyal relationships yield higher service rates (97% vs 93%)
2. **Sellers reward loyalty** → Loyal customers have higher acceptance rates (92% vs 88%)
3. **Positive feedback** → System converges to high-loyalty, price-dispersed equilibrium

#### Implementation Features

**Market Structure**:
```
- 100 buyers (heterogeneous valuations: 12, 15, 18)
- 10 sellers (p_in = 9, stock = 15 units/day)
- 1000 days simulation
- Discrete prices [0, 20]
```

**Mechanisms Implemented**:

1. **Classifier System Learning**: Buyers and sellers use rule-based reinforcement learning
   - Buyers: Learn which sellers to visit based on loyalty
   - Sellers: Learn price discrimination based on customer loyalty and market state

2. **Stochastic Auction**: Rule selection with exploration noise and trembling hand
   ```rust
   stochastic_auction(rules, noise_std=0.1, tremble_prob=0.025)
   ```

3. **Loyalty Dynamics**: Continuous loyalty values updated each day
   ```rust
   L(t) = L(t-1)/(1+α) + (α if visited, else 0)
   ```

4. **Price Discrimination**: Sellers condition prices on:
   - Customer loyalty class (Low/Medium/High)
   - Stock-to-queue ratio (market pressure)

**Expected Results**:
- Price dispersion emerges naturally (coefficient of variation ~10-15%)
- Loyalty concentration increases over time (γ → 0.3-0.5)
- Higher-valuation buyers pay higher average prices
- Market efficiency ~85-90% (transaction completion rate)

**Run the simulation**:
```bash
cargo run -p evolving_market
# Outputs: market_evolution.csv with daily statistics
```

#### Further Reading

- **Paper Summary**: `/prior-art/evolving-market-structure-abce.md` - Comprehensive implementation guide
- **Original Paper**: Kirman, A. & Vriend, N.J. (2001). "Evolving Market Structure: An ACE Model of Price Dispersion and Loyalty." *Journal of Economic Dynamics and Control*, 25(3-4), 459-502.

---

### Allocative Efficiency of Zero-Intelligence Traders (Gode & Sunder, 1993) ✅

Implementation of Gode & Sunder's groundbreaking demonstration that market institutions alone can generate high allocative efficiency, even when traders behave randomly within budget constraints.

#### Theoretical Background

**The Rationality Puzzle**: Standard economic theory attributes market efficiency to intelligent, profit-maximizing agents. But how much of observed market efficiency comes from trader intelligence versus the market mechanism itself?

**Experimental Setup**: Continuous double auction (CDA) market where:
- 12 traders (6 buyers, 6 sellers) trade units with private values/costs
- Each trader receives multiple units with different redemption values (buyers) or costs (sellers)
- Best bid and best ask displayed; transactions occur when bid ≥ ask at earlier order's price
- Trading continues until market exhaustion or time limit

**Key Innovation**: Two types of zero-intelligence traders:

1. **ZI-U (Unconstrained)**: Submit random bids/asks uniformly over [1, 200]
   - No learning, memory, or profit-seeking
   - Can and will make losing trades

2. **ZI-C (Constrained)**: Submit random bids/asks respecting budget constraint
   - Buyers: bid uniformly in [1, value]
   - Sellers: ask uniformly in [cost, 200]
   - **Cannot make losing trades** (the critical constraint)

**Central Finding**: ZI-C achieves 97-100% allocative efficiency across diverse market structures, approaching human trader performance, while ZI-U efficiency varies widely (48-90%).

#### Implementation Features

**Market Configurations** (5 markets from the paper):
```
Market 1: Equilibrium price ≈ 69, quantity = 15 (standard design)
Market 2: Equilibrium price ≈ 69, quantity = 15 (different curve shapes)
Market 3: Equilibrium price ≈ 106, quantity = 6 (thin market)
Market 4: Equilibrium price ≈ 170, quantity = 15 (high price equilibrium)
Market 5: Equilibrium price ≈ 131, quantity = 24 (many marginal units)
```

**Mechanisms Implemented**:

1. **Continuous Double Auction**: Order book with best bid/ask, immediate execution when prices cross
   ```rust
   // Price priority: transaction at earlier order's price
   if new_bid >= best_ask { execute_at(best_ask.price) }
   if new_ask <= best_bid { execute_at(best_bid.price) }
   ```

2. **Sequential Unit Trading**: Traders must complete unit i before trading unit i+1 (creates demand/supply curves)

3. **Random Trader Selection**: Each iteration randomly selects a trader with remaining units to submit an order

4. **Allocative Efficiency Metric**:
   ```rust
   efficiency = (actual_surplus / max_possible_surplus) * 100%
   actual_surplus = Σ(buyer_value - price) + Σ(price - seller_cost)
   ```

**Expected Results**:

| Trader Type | Efficiency Range | Price Convergence | Market Sensitivity |
|-------------|------------------|-------------------|-------------------|
| ZI-U | 48% - 90% | None (random walk) | High variance across markets |
| ZI-C | 97% - 100% | Within-period (p<0.05) | Stable across markets |
| Human | 90% - 100% | Rapid then stable | Low variance |

**Key Predictions Validated**:
- ZI-C efficiency ≥ 97% across all five markets
- ZI-C vs ZI-U gap ≥ 10 percentage points
- ZI-C shows significant negative price convergence slope
- ZI-U shows no convergence (slope ≈ 0)

**Run the simulation**:
```bash
# Run standard 100-session experiment across all 5 markets
cargo run -p zi_traders

# Test iteration count sensitivity (Markets 1, 3, 5)
cargo run -p zi_traders --bin iteration_experiment
```

#### Implementation Architecture

**Module**: `zi_traders/`

**Event-Driven Design**:
```rust
enum Event {
    PeriodStart { period, market_id },
    OrderRequest { period, trader_id, iteration },
    OrderSubmitted { trader_id, order_type, price, value_or_cost },
    Transaction { buyer_id, seller_id, price, ... },
    PeriodEnd { period },
}
```

**Agents**:
- `ZIUTrader`: Generates random orders uniformly over [1, 200] regardless of value/cost
- `ZICTrader`: Generates random orders constrained by value (buyers) or cost (sellers)
- `Coordinator`: Manages order book, executes transactions, tracks efficiency metrics

**Critical Implementation Details**:

1. **Budget Constraint Enforcement** (zi_traders/src/traders.rs):
   ```rust
   // ZI-C buyers: bid ∈ [1, value]
   let price = self.rng.gen_range(1..=unit.value_or_cost);

   // ZI-C sellers: ask ∈ [cost, 200]
   let price = self.rng.gen_range(unit.value_or_cost..=200);
   ```

2. **Transaction Execution** (zi_traders/src/coordinator.rs):
   ```rust
   // Price priority: use earlier order's price
   if bid_price >= ask_price {
       transaction_price = best_ask.price  // Ask arrived first
   }
   ```

3. **Efficiency Calculation**:
   ```rust
   fn efficiency(&self) -> f64 {
       (self.total_surplus as f64 / self.max_possible_surplus as f64) * 100.0
   }
   ```

#### Parameters and Tuning

**Standard Configuration**:
```rust
num_traders: 12 (6 buyers, 6 sellers)
num_periods: 6
max_iterations_per_period: 500  // Sufficient for market exhaustion
num_sessions: 100  // Increase to 1000 for full replication
price_range: [1, 200]
```

**Why These Values**:
- **500 iterations**: Ensures all profitable trades can occur (even thin Market 3 with 6 equilibrium units)
- **6 periods**: Matches original experimental design (though ZI traders don't learn between periods)
- **100 sessions**: Provides statistical reliability; increase to 1000 for publication-quality results
- **Price range [1, 200]**: Wide enough to test constraint effects without excessive computation

#### Further Reading

- **Paper Summary**: `/prior-art/alloc-efficiency-zi-traders.md` - Complete implementation specification with pseudocode
- **Original Paper**: Gode, D.K. & Sunder, S. (1993). "Allocative Efficiency of Markets with Zero-Intelligence Traders: Market as a Partial Substitute for Individual Rationality." *Journal of Political Economy*, 101(1), 119-137.

---

### Taming the Basel Leverage Cycle (Aymanns, Caccioli, Farmer & Tan, 2016) ✅

Implementation of the leverage cycle model demonstrating how Value-at-Risk (VaR) based leverage constraints, as mandated by Basel II banking regulations, can endogenously generate systemic risk through feedback effects.

#### Theoretical Background

**The Regulatory Paradox**: Regulations designed to make individual banks safer can, through feedback effects, create system-wide instability. Under Basel II, banks estimate their risk using historical volatility and adjust their leverage accordingly.

This creates a dangerous feedback loop:

1. **Rising prices** → Lower measured volatility → Higher allowed leverage → Banks buy more assets → Prices rise further
2. **Falling prices** → Higher measured volatility → Lower allowed leverage → Banks must sell assets → Prices fall further

**Three Stability Regimes**:

| Regime | Characteristic | Behavior |
|--------|---------------|----------|
| **(i) Stable** | Low leverage, small bank | System converges to fixed point equilibrium |
| **(ii) Leverage Cycles** | Intermediate leverage | Chaotic oscillations with 10-15 year periods |
| **(iii) Globally Unstable** | High leverage | System diverges—prices go to infinity or zero |

**The Leverage Control Policy**:
```
λ̄(t) = α(σ²(t) + σ₀²)^b
```
Where:
- **λ̄(t)**: Target leverage at time t
- **α**: Bank's "riskiness" parameter
- **σ²(t)**: Perceived risk (historical volatility estimate)
- **b**: Cyclicality parameter (b = -0.5 for Basel II/VaR)

The cyclicality parameter b determines policy nature:
- **b = -0.5**: Procyclical (Basel II) — leverage decreases when volatility increases
- **b = 0**: Constant leverage — unchanged regardless of volatility
- **b = +0.5**: Countercyclical — leverage increases when volatility increases

#### Implementation Features

**Agent Structure**:
- **Bank**: Leveraged investor representing the banking sector. Estimates volatility, sets leverage target, adjusts balance sheet
- **Fund**: Unleveraged fundamentalist investor. Trades toward fundamental value, subject to GARCH demand shocks

**State Space** (6 coupled variables):
```
σ²(t)  - Bank's perceived risk (volatility estimate)
w_F(t) - Fund's portfolio weight in risky asset
p(t)   - Current price of risky asset
n(t)   - Fraction of risky asset owned by bank
L_B(t) - Bank's liabilities
p'(t)  - Lagged price (previous time step)
```

**Four Scenarios from Experiment 1**:

| Scenario | Bank Size | GARCH Noise | Expected Behavior |
|----------|-----------|-------------|-------------------|
| (i) Deterministic Micro | Small (Ē=10⁻⁵) | None | Fixed point equilibrium |
| (ii) Deterministic Macro | Large (Ē=2.27) | None | Chaotic leverage cycles |
| (iii) Stochastic Micro | Small (Ē=10⁻⁵) | Strong | Mean-reverting random walk |
| (iv) Stochastic Macro | Large (Ē=2.27) | Weak | Irregular leverage cycles |

**Key Findings Verified**:
- Small banks converge to fixed point (low price variance)
- Large banks exhibit leverage cycles (chaotic oscillations)
- GARCH noise increases volatility
- Procyclical policy (Basel II): lower volatility → higher leverage
- Slower adjustment speed increases stability
- Longer risk estimation horizons improve stability

**Run the simulation**:
```bash
# Run all four scenarios with comparison
cargo run -p leverage_cycle

# Verify key results from the paper
cargo run -p leverage_cycle --example verify_paper_results

# Demonstrate feedback loop dynamics
cargo run -p leverage_cycle --example feedback_loop_demo
```

**Example Output**:
```
Scenario                    Price Std   Lev Range      Stability
--------------------------- ----------- ----------- ---------------
Deterministic Micro (i)         0.0000        0.00          Stable
Deterministic Macro (ii)        0.1234        2.45 Leverage Cycles
Stochastic Micro (iii)          0.0234        0.12          Stable
Stochastic Macro (iv)           0.0567        1.89 Leverage Cycles
```

#### Implementation Architecture

**Module**: `leverage_cycle/`

**Event-Driven Design**:
```rust
enum Event {
    Step { step: usize },     // Advance system by one time step
    RunEnd { run_id: usize }, // Signal end of simulation run
}
```

**Core Components**:
- `LeverageCycleSystem`: Main agent implementing the coupled 6-variable dynamical system
- `SystemState`: State vector with update equations
- `GarchProcess`: Exogenous volatility clustering for stochastic scenarios
- `StabilityAnalysis`: Compute VaR, CVaR, and classify stability regime

**Critical Implementation Details**:

1. **Volatility Estimation** (exponential moving average):
   ```rust
   σ²_new = (1 - τδ)σ² + τδ × [log(p/p') × t_VaR/τ]²
   ```

2. **Market Clearing**:
   ```rust
   p_new = (w_B × (c_B + ΔB) + w_F_new × c_F) / (1 - w_B×n - w_F_new×(1-n))
   ```

3. **Balance Sheet Adjustment**:
   ```rust
   ΔB = τ × θ × (λ̄ × E_B - A_B)  // Adjust toward target leverage
   ```

#### Parameters and Tuning

**Standard Configuration**:
```rust
τ = 0.1        // Time step (years)
δ = 0.5        // Memory parameter (~2 year lookback)
t_VaR = 0.1    // VaR time horizon (years)
θ = 10.0       // Leverage adjustment speed (year⁻¹)
η = 10.0       // Equity redistribution speed (year⁻¹)
b = -0.5       // Cyclicality (Basel II)
α = 0.075      // Risk level
Ē = varies     // Bank equity target (controls bank size)
w_B = 0.3      // Bank's portfolio weight
μ = 25.0       // Fundamental price
```

**Why These Values**:
- **τ = 0.1**: Small enough for convergence, large enough for efficiency
- **δ = 0.5**: ~2 year lookback matches typical risk estimation windows
- **θ = 10**: Fast adjustment amplifies feedback; slower (θ < 5) increases stability
- **b = -0.5**: Fully procyclical (Basel II/VaR) as in the paper

#### Further Reading

- **Paper Summary**: `/prior-art/leverage-cycles.md` - Comprehensive analysis with equations and implementation guide
- **Original Paper**: Aymanns, C., Caccioli, F., Farmer, J.D. & Tan, V.W. (2016). "Taming the Basel Leverage Cycle." *Journal of Financial Stability*, 27, 263-277.

---

## Shared Implementation Patterns

After implementing seven distinct ABM papers, clear architectural patterns have emerged across all modules. These patterns guide the design of new simulations and ensure consistency with the DES framework.

### Core Structural Patterns

#### 1. Domain Event Enum Pattern
**Every module defines a domain-specific `Event` enum capturing all possible state transitions.**

Examples across modules:
- `simple_queue`: `ResourceRequested`, `ResourceAcquired`, `ResourceReleased`, `ResourceRequestExpired`
- `evolution_coop`: `RoundStart`, `DecisionMade`, `RoundComplete`, `MatchComplete`
- `zi_traders`: `PeriodStart`, `OrderRequest`, `OrderSubmitted`, `Transaction`
- `insurance_cycles`: `YearStart`, `PricingRequest`, `PriceSubmitted`, `MarketCleared`

**Design principle**: Events are past-tense verbs representing state changes, not imperative commands.

#### 2. Unified Stats Enum Pattern
**All modules define individual stats structs wrapped in a unified `Stats` enum.**

```rust
// Individual stats for each agent type
pub struct ResourceStats { ... }
pub struct ConsumerStats { ... }

// Unified enum for heterogeneous collection
pub enum Stats {
    ResourceStats(ResourceStats),
    ConsumerStats(ConsumerStats),
}
```

**Rationale**: Allows `EventLoop<T, S>` to collect heterogeneous stats from different agent types in a type-safe manner.

#### 3. Stats-as-Observable-State Pattern
**Stats serve as the public interface to agent state, used for testing and analysis.**

Stats structs universally contain:
- **Configuration context**: `agent_id`, `trader_type`, `position`, `capacity`
- **Current state**: `current_queue_length`, `num_customers`, `capital`, `best_bid`
- **Cumulative metrics**: `total_arrivals`, `total_score`, `transactions_completed`
- **Time series**: `price_history`, `loss_ratio_history`, `leverage_history`

**Key insight**: The `stats()` method acts as a **projection** from private agent state to observable state. Tests verify agent behavior through Stats, not internal fields, maintaining proper encapsulation.

### Agent Architecture Patterns

#### 4. Two Fundamental Agent Architectures

**Pattern A: All-Entity Agents** (Decentralized Systems)
- *Used in*: `simple_queue`, `evolution_coop`
- Every agent represents a real entity in the domain
- Agents interact purely via events (no central orchestrator)
- Emergent behavior from decentralized agent interactions
- *Example*: Consumers and Resources interact directly through request/release events

**Pattern B: Coordinator + Entity Agents** (Centralized Mechanism)
- *Used in*: `zi_traders`, `evolving_market`, `lloyds_insurance`, `insurance_cycles`
- Coordinator implements system rules/mechanism
- Coordinator maintains shared state (order book, active participants, market clearing)
- Entities respond to coordinator events
- Coordinator handles turn-taking, resource allocation, rule enforcement
- *Example*: Market coordinator manages order book, traders submit bids/asks

**Trade-offs**:
- ✅ Pattern B matches domain model when mechanism ≠ participant (markets, auctions, games)
- ✅ Clear separation of concerns between mechanism and participants
- ⚠️ Broadcast overhead (coordinator receives all events, must filter)
- ⚠️ Shadow state maintenance (coordinator tracks participant state)

**Design guidance**: Use Pattern A for fully decentralized systems (interacting entities). Use Pattern B when the domain has an explicit central mechanism (market, auction house, game referee).

### Event Processing Patterns

#### 5. Event Filtering Pattern
**Agents receive ALL events via broadcast; filter for relevant ones.**

```rust
// Check event relevance
if rid != &self.resource_id {
    return des::Response::new(); // Ignore irrelevant events
}

// Player only processes events where they're a participant
if *player_a_id == self.id || *player_b_id == self.id {
    // Process event
} else {
    des::Response::new() // Ignore
}
```

**Implication**: Every agent sees every event. Efficient filtering in the `act()` method is critical for performance.

#### 6. Time-Delta Event Scheduling Pattern
**Events are always scheduled relative to `current_t`, never as absolute times.**

```rust
des::Response::event(current_t + 1, Event::RoundComplete { ... })
des::Response::event(current_t + arrival_interval, Event::ResourceRequested(...))
```

**Consistent timing rule**: Coordinators use `current_t + 1` for next iteration events to maintain deterministic ordering.

#### 7. Lifecycle Event Pattern
**Simulations have structured start/iteration/end events.**

Common patterns across modules:
- **Start events**: `TournamentStart`, `SessionStart`, `YearStart`, `Day`
- **Iteration events**: `RoundStart`, `Month`, `Year`, `Step`
- **End events**: `TournamentComplete`, `YearEnd`, `RunEnd`

**Purpose**: Clear simulation phases enable agent initialization, iteration, and cleanup logic.

### Behavioral Patterns

#### 8. Stochastic Behavior via Distribution Sampling
**Agents use `rand_distr` for probabilistic decision-making.**

Examples across modules:
- `simple_queue`: `Geometric` (arrivals), `Normal` (service times)
- `evolving_market`: `stochastic_auction()` for rule selection with noise and trembling hand
- `lloyds_insurance`: `Gamma` (claim amounts), `Poisson` (catastrophe events)
- `insurance_cycles`: `Gamma` (claim distribution)

**Pattern**: RNGs are agent-owned fields (`self.rng`), often seeded for reproducibility in tests.

#### 9. Reinforcement Learning Classifier System Pattern
**Adaptive agents learn through rule-based reinforcement learning.**

*Used in*: `evolving_market` (Kirman & Vriend model)

**Components**:
```rust
pub struct Rule<C, A> {
    pub condition: C,
    pub action: A,
    pub strength: f64,  // Updated via EWMA
}
```

**Mechanisms**:
- `stochastic_auction()` - Probabilistic rule selection with exploration noise
- `update_strength()` - EWMA-based learning: `s(t) = (1-c)*s(t-1) + c*reward`
- `update_loyalty()` - Relationship dynamics: `L(t) = L(t-1)/(1+α) + (α if visited)`

**Application**: Buyers learn which sellers to visit; sellers learn pricing strategies based on loyalty and market state.

### State Management Patterns

#### 10. Structured State Management (Coordinators)
**Coordinators use `HashMap`/`HashSet` for O(1) lookups, avoiding linear searches.**

```rust
// zi_traders Coordinator
active_traders: HashSet<usize>,        // O(1) membership test
trader_inventories: HashMap<usize, Vec<Unit>>, // O(1) inventory lookup
best_bid: Option<Order>,
best_ask: Option<Order>,

// evolution_coop TournamentCoordinator
current_matches: HashMap<usize, MatchState>,  // O(1) match lookup
```

**Anti-pattern**: Avoid `Vec` linear scans for participant tracking in coordinators.

#### 11. Shadow State Pattern
**Coordinators maintain copies of participant state for orchestration.**

Examples:
- `insurance_cycles::MarketCoordinator`: Tracks insurer capital for capacity constraints
- `lloyds_insurance::CentralRiskRepository`: Tracks policies and quotes
- `zi_traders::Coordinator`: Tracks trader inventories

**Critical requirement**: Shadow state must match sum of individual agent stats. Validated via integration tests like `test_shadow_state_consistency`.

#### 12. Time Series Collection Pattern
**Stats include bounded history vectors for analysis.**

```rust
pub struct SystemStats {
    pub price_history: Vec<f64>,
    pub leverage_history: Vec<f64>,
    // Bounded to last N values or full simulation
}

pub struct InsurerStats {
    pub price_history: Vec<f64>,      // Last 2 years for elasticity
    pub quantity_history: Vec<usize>, // Last 2 years
}
```

**Use cases**:
- Autocorrelation analysis (`insurance_cycles::autocorrelation()`)
- Cycle detection (`has_cycles()`, `cycle_period()`, `dominant_frequency()`)
- AR(2) model fitting (`fit_ar2()`) for validation against theoretical predictions

### Code Organization Patterns

#### 13. Domain Logic Helper Functions
**Common calculations extracted to module-level pure functions.**

Examples:
- `evolution_coop::calculate_payoff()` - Prisoner's Dilemma payoff matrix
- `evolving_market::update_loyalty()`, `update_strength()`, `stochastic_auction()`
- `insurance_cycles::MarketStats::calculate_herfindahl()`, `calculate_gini()`

**Benefits**: Functions are testable in isolation, reusable across agents, and clearly document domain logic.

#### 14. Configuration Struct Pattern
**Each module defines a `Config` or `ModelConfig` struct with scenario presets.**

```rust
impl ModelConfig {
    pub fn baseline() -> Self { ... }
    pub fn scenario_1() -> Self { ... }  // From paper
    pub fn low_beta() -> Self { ... }     // Parameter variation
}
```

**Features**:
- All simulation parameters centralized in one struct
- Named scenarios matching research papers
- Helper methods for derived parameters (`gamma_shape()`, `gamma_scale()`)

#### 15. Module Organization Pattern
**Complex simulations split across focused modules.**

Typical structure:
- `lib.rs` - Core types (Event, Stats, Config), domain logic functions
- `coordinator.rs` - Orchestration agent (Pattern B architectures)
- `agents.rs` / specific agent files - Entity agent implementations
- `helpers.rs` - Utility functions (geometry, distributions, etc.)
- `analysis.rs` / `output.rs` - Post-processing and metrics

**Examples**:
- `evolving_market`: `lib.rs`, `agents.rs`, `coordinator.rs`
- `lloyds_insurance`: 16 modules (one per agent type + shared components)
- `insurance_cycles`: `lib.rs`, `insurer.rs`, `market_coordinator.rs`, `helpers.rs`, `output.rs`

### Testing Patterns

#### 16. Testing via Stats Interface
**Tests verify agent behavior through Stats, not internal state.**

```rust
#[test]
fn given_full_resource_when_consumer_requests_then_queued() {
    let mut resource = Resource::new(0, 1);
    resource.act(10, &Event::ResourceRequested(0, 1));

    let s = resource.stats();
    assert!(s.is_at_capacity()); // Test via Stats

    resource.act(15, &Event::ResourceRequested(0, 2));

    let s = resource.stats();
    assert_eq!(s.current_queue_length, 1); // Observable state
}
```

**Philosophy**: Stats = observable behavior; internal state = implementation detail. This approach:
- Maintains encapsulation (agent internals remain private)
- Aligns with event sourcing (Stats = projection of event stream)
- Enables refactoring (can change internal data structures without breaking tests)
- Documents observable behavior (Stats shows what the simulation measures)

#### 17. Multi-Phase Market Clearing Pattern
**Coordinator simulations use request-consolidate-select-notify cycles.**

*Used in*: `lloyds_insurance`, `evolving_market`

**Example** (Lloyd's insurance quote process):
1. **Request phase**: `LeadQuoteRequested` broadcast to syndicates
2. **Consolidation deadline**: `LeadQuoteConsolidationDeadline` triggers broker evaluation
3. **Selection phase**: Broker chooses best quote from received offers
4. **Notification phase**: `LeadQuoteAccepted` broadcast confirms winner

**Timing convention**: Each phase scheduled at `current_t + 1` to maintain deterministic event ordering.

### Meta-Patterns Across All Modules

1. **Event Sourcing**: Events are the source of truth; state is derived from event stream
2. **Broadcast + Filter**: All agents receive all events, filter locally in `act()` method
3. **Stats as Projection**: Public observable state (Stats) vs. private implementation details
4. **Coordinator Shadow State**: Centralized orchestration requires state duplication + validation
5. **Determinism via Seeding**: Tests use seeded RNGs for reproducibility
6. **Configuration as Code**: Research paper scenarios encoded as `Config` presets

### Design Guidelines for New Implementations

When implementing a new research paper:

1. **Choose architecture deliberately**:
   - All-Entity (Pattern A) for decentralized systems
   - Coordinator + Entity (Pattern B) for centralized mechanisms

2. **Design Stats first**: What needs to be observable for validation against paper results?

3. **Extract domain logic**: Pure functions for payoffs, updates, selections (testable in isolation)

4. **Use efficient data structures**: HashMap/HashSet for participant lookups, never linear Vec scans

5. **Test via Stats interface**: Maintain encapsulation, verify observable behavior matches paper

6. **Document coordinator rationale**: If using Pattern B, explain why coordinator is an Agent (see CLAUDE.md)

7. **Provide scenario configs**: Encode paper's experimental setups as named Config presets

8. **Collect time series**: Include history vectors in Stats for validation via autocorrelation, cycle detection, convergence analysis

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
- ✅ Kirman and Vriend (2001) - Evolving market structure with price dispersion and loyalty
- ✅ Gode and Sunder (1993) - Zero-intelligence traders and allocative efficiency
- ✅ Aymanns et al. (2016) - Basel leverage cycle and systemic risk from VaR-based regulation

**TODO**:
- TRANSIMS code (Barrett et al., 1995, Nagel, Beckman and Barrett, 1998)
- drug addiction (Agar and Wilson, 2002, Hoffer, Bobashev and Morris, 2009, Heard, Bobashev and Morris, 2014)
- policy relevant and exercised to study policy alternatives (Dawid et al., 2012)
- Donier et al. (2015) showed that a linear virtual order book profile
- Gode and Sunder (1997) - Extensions to ZI traders
- K-level cognition (Camerer, Ho and Chong, 2004) has found use in ABMs (Latek, Kaminski and Axtell, 2009)
