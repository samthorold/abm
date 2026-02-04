# Lloyd's of London Specialty Insurance Market DES Model

**Paper:** Exploring the Dynamics of the Specialty Insurance Market Using a Novel Discrete Event Simulation Framework: a Lloyd's of London Case Study
**Authors:** Sedar Olmez, Akhil Ahmed, Keith Kam, Zhe Feng, Alan Tua
**Published:** 2024, Journal of Artificial Societies and Social Simulation 27(2)
**Domain:** Financial Markets, Insurance Economics, Discrete Event Simulation

## Executive Summary

This paper presents a novel Discrete Event Simulation (DES) of the Lloyd's of London specialty insurance market, exploring complex market dynamics including the underwriting cycle, risk syndication, and exposure management. The model uses the HADES framework (open-sourced by Ki Insurance) and demonstrates that:

1. **Catastrophe events** drive pronounced underwriting cycles
2. **Sophisticated exposure management** (VaR-based) reduces insolvency risk
3. **Lead-follow syndication** reduces volatility and couples loss experiences across syndicates
4. The regulatory structure at Lloyd's creates a healthier, more stable marketplace

This is the **first discrete event simulation** of the specialty insurance market inspired by real-world actors within Lloyd's of London.

## Core Problem

The specialty insurance market is a large-scale complex system with:
- Many uncertainties and complex business relationships
- Non-linear dynamics and interactions among participants
- The **underwriting cycle**: periods alternating between high profitability/less competition and low profitability/high competition

Traditional approaches (time-series methods, differential equation models) fail to capture micro-scale interactions that produce macro-level emergent phenomena like the underwriting cycle.

## Novel Contributions

### 1. DES Framework vs. Traditional ABM

Unlike traditional Agent-Based Models (ABMs), the DES approach:
- Handles **time-irregular and asynchronous events** seamlessly
- Events trigger at irregular intervals (from few to many simultaneously)
- Captures the unpredictable nature of insurance claims
- Uses the HADES framework: Processes + Events architecture

### 2. Lloyd's-Specific Features

The model incorporates unique Lloyd's market structures:
- **Lead and follow insurers**: One lead syndicate shapes policy terms, multiple followers agree to terms
- **Risk syndication**: Multiple syndicates underwrite portions of each risk
- **Line sizes**: Fraction of risk each syndicate covers
- **Broker-syndicate networks**: Relationships determining quote requests

### 3. Comprehensive Loss Modeling

- **Attritional losses**: High-frequency, low-severity, uncorrelated (e.g., individual claims)
- **Catastrophe losses**: Low-frequency, high-severity, spatially correlated by peril region
- **Peril regions**: Risks assigned to geographic/peril zones; catastrophes affect entire regions

## Model Architecture

### Key Processes (Agents)

1. **Time Process**
   - Generates Day, Month, Year events
   - Enables multi-granularity temporal dynamics

2. **Broker Process**
   - Generates new risks (Poisson distributed, λ = risks per day)
   - Broadcasts risks to syndicates
   - Sets quote deadlines (lead consolidation, lead selection, follow consolidation, follow selection)

3. **Broker-Syndicate Network Process**
   - Manages relationships between brokers and syndicates
   - Topologies: Circular (distance-based), Network (edge weights), Random
   - Uses `top_k` parameter to select k best syndicates for quote requests

4. **Central Risk Repository Process**
   - Tracks all risks, quotes, and policies
   - Applies attritional and catastrophe losses to syndicates
   - Selects lead (cheapest quote) and followers (line size allocation)

5. **Syndicate Process** (most complex)
   - Coordinates multiple sub-processes
   - Manages capital, handles claims, pays dividends
   - Can go insolvent if capital depleted

### Syndicate Sub-Processes

#### A. **Actuarial Sub-Process**
Calculates "fair price" based on expected losses:

```
P̃_t = z·X̄_t + (1-z)·λ'_t·μ'_t
```
Where:
- `X̄_t` = syndicate's past weighted average claims
- `λ'_t` = industry-wide average claim frequency
- `μ'_t` = industry-wide expected claim cost
- `z` = internal experience weight (0.5 default)

Final actuarial price includes risk loading:
```
P_at = P̃_t + α·F_t
```
Where:
- `F_t` = standard deviation of syndicate's claims
- `α` = volatility weight

#### B. **Underwriting Sub-Process**
Scales actuarial price based on market supply/demand:

```
P_t = P_at · e^(m_t)
```
Where `m_t` = underwriter log markup (exponentially weighted moving average capturing competitive pressure)

#### C. **Exposure Management Sub-Processes**

**Premium Exposure Management (simple):**
- Proxy: Premium-to-capital ratio
- Scales quotes based on current exposure
- Rejects quotes if over-exposed

**VaR Exposure Management (sophisticated):**
- Runs Monte Carlo simulations by peril region
- Calculates Value at Risk at α exceedance probability (default 5%)
- Ensures capital exceeds VaR threshold (similar to Solvency II regulations)
- Promotes uniform distribution across peril regions

#### D. **Line Size Sub-Module**
- Lead syndicates: Offer default line size (e.g., 50%)
- Follow syndicates: Calculate "pricing strength" = follower's price / lead price
  - If >1, offer larger line size (price is good)
  - If <1, offer smaller line size or decline

#### E. **Dividend Sub-Module**
```
D = γ · Pr_t
```
Where:
- `Pr_t` = profit made by syndicate
- `γ` = profit fraction (default 0.4)

### Loss Generator Processes

**Attritional Loss Generator:**
- Pre-generates claim events when risk created
- Number of claims: Poisson(λ = yearly claim frequency)
- Severity: Gamma distribution (shape = 1/COV², scale = μ·COV²)
- Events scheduled within risk expiration period

**Catastrophe Loss Generator:**
- Pre-generates catastrophe events at simulation start
- Number of events: Poisson(λ = mean cat events per year × simulation years)
- Each event assigned random peril region
- Total loss: Truncated Pareto (min = minimum catastrophe damage)
- Loss cascades to all risks in affected region, then to syndicates on those policies

### Industry Statistics Process
- Aggregates market-wide metrics
- Distributes to actuarial sub-processes for pricing calculations

## Key Experimental Findings

### Scenario 1: Base Case (Attritional Only)
**Setup:** 5 syndicates, 25 brokers, actuarial pricing, premium exposure management, attritional losses only

**Results:**
- Premiums converge to fair price (~$300k)
- Capital fluctuates, some syndicates go insolvent
- Loss ratios show early cyclicality (periods above/below 1.0)
- Demonstrates basic market profitability cycles

### Scenario 2: Catastrophe Events
**Setup:** Same as Scenario 1 + catastrophe events

**Results:**
- **Pronounced cyclicality** in premiums (see Figure 6 in paper)
- Mechanism: Premiums converge → catastrophe hits → large losses → syndicates raise prices → effect wears off → cycle repeats
- Loss ratios spike above 1.0 during catastrophes
- More insolvencies than Scenario 1
- **Key finding**: Catastrophes are primary driver of underwriting cycles

### Scenario 3: VaR Exposure Management
**Setup:** Replace premium EM with VaR EM

**Results:**
- **Uniform deviation** (measure of peril region distribution) approaches zero
- Syndicates distribute risk uniformly across peril regions
- Fewer insolvencies
- Better capitalized portfolios
- **Key finding**: Sophisticated EM = better tail risk management

### Scenario 4: Lead-Follow Dynamics
**Setup:** 5 syndicates, 25 brokers, lead-follow enabled, attritional losses only

**Results:**
- **Tightly coupled premium convergence** to fair price (much lower volatility than Scenario 1)
- **Highly correlated loss ratios** across syndicates (similar loss experience)
- **Zero insolvencies** (vs. insolvencies in Scenario 1)
- Mechanism: Risk syndication → shared losses → similar experience → similar pricing
- **Key finding**: Lloyd's lead-follow structure creates market stability (validates regulatory design)

## Implementation Details for rs-des

### Agent Types Needed

1. **TimeGenerator** (generates Day/Month/Year events)
2. **Broker** (generates risks, sets deadlines)
3. **BrokerSyndicateNetwork** (selects syndicates for quotes)
4. **CentralRiskRepository** (tracks policies, applies losses)
5. **Syndicate** (coordinates sub-agents)
6. **ActuarialPricer** (sub-agent of Syndicate)
7. **Underwriter** (sub-agent of Syndicate)
8. **PremiumExposureManager** (sub-agent of Syndicate)
9. **VaRExposureManager** (sub-agent of Syndicate)
10. **LineSizeCalculator** (sub-module of Syndicate)
11. **DividendCalculator** (sub-module of Syndicate)
12. **AttritionalLossGenerator**
13. **CatastropheLossGenerator**
14. **IndustryStatsAggregator**

### Event Types Needed

```rust
pub enum Event {
    // Time events
    Day,
    Month,
    Year,

    // Risk lifecycle
    RiskBroadcasted { risk_id: usize, peril_region: usize, limit: f64 },
    LeadQuoteRequested { risk_id: usize, syndicate_id: usize },
    FollowQuoteRequested { risk_id: usize, syndicate_id: usize, lead_price: f64 },

    // Quote deadlines
    LeadQuoteConsolidationDeadline { risk_id: usize },
    LeadQuoteSelectionDeadline { risk_id: usize },
    FollowQuoteConsolidationDeadline { risk_id: usize },
    FollowQuoteSelectionDeadline { risk_id: usize },

    // Quote components (internal to syndicate)
    QuoteComponentComputed { risk_id: usize, component: QuoteComponent },

    // Quote offers
    LeadQuoteOffered { risk_id: usize, syndicate_id: usize, price: f64, line_size: f64 },
    FollowQuoteOffered { risk_id: usize, syndicate_id: usize, line_size: f64 },

    // Acceptances
    LeadQuoteAccepted { risk_id: usize, syndicate_id: usize },
    FollowQuoteAccepted { risk_id: usize, syndicate_id: usize, line_size: f64 },

    // Losses
    AttritionalLossOccurred { risk_id: usize, amount: f64 },
    CatastropheLossOccurred { peril_region: usize, total_loss: f64 },
    ClaimReceived { risk_id: usize, syndicate_id: usize, amount: f64 },

    // Capital/statistics
    SyndicateCapitalReported { syndicate_id: usize, capital: f64 },
    SyndicateBankrupted { syndicate_id: usize },
    IndustryLossStatsReported { stats: IndustryLossStats },
    IndustryPricingStatsReported { stats: IndustryPricingStats },
}

pub enum QuoteComponent {
    ActuarialPrice(f64),
    UnderwritingMarkup(f64),
    ExposureManagementDecision(ExposureDecision),
    LineSize(f64),
}

pub enum ExposureDecision {
    Accept,
    Reject,
    ScalePremium(f64),
}
```

### Key State Variables

**Risk:**
```rust
pub struct Risk {
    id: usize,
    peril_region: usize,
    limit: f64,
    expiration_time: usize,
    broker_id: usize,
}
```

**Syndicate:**
```rust
pub struct Syndicate {
    id: usize,
    capital: f64,
    initial_capital: f64,
    policies: Vec<PolicyParticipation>,
    loss_history: Vec<f64>,
    premium_history: Vec<f64>,
    // Internal state for quote assembly
    pending_quotes: HashMap<usize, Vec<QuoteComponent>>,
}

pub struct PolicyParticipation {
    risk_id: usize,
    line_size: f64,
    premium_collected: f64,
    is_lead: bool,
}
```

**ActuarialPricer state:**
```rust
pub struct ActuarialPricer {
    syndicate_id: usize,
    loss_experience: Vec<f64>,
    loss_recency_weight: f64,
    internal_experience_weight: f64,
    volatility_weight: f64,
}
```

**VaRExposureManager state:**
```rust
pub struct VaRExposureManager {
    syndicate_id: usize,
    peril_regions: Vec<PerilRegionExposure>,
    var_exceedance_prob: f64,
    safety_factor: f64,
    num_simulations: usize,
}

pub struct PerilRegionExposure {
    region_id: usize,
    risks_underwritten: Vec<usize>,
    total_exposure: f64,
    var_estimate: f64,
}
```

### Statistics Types

**Syndicate Stats:**
```rust
pub struct SyndicateStats {
    syndicate_id: usize,
    capital: f64,
    initial_capital: f64,

    // Current state
    num_policies: usize,
    total_premium_written: f64,
    total_line_size: f64,

    // Cumulative metrics
    total_premiums_collected: f64,
    total_claims_paid: f64,
    num_claims: usize,

    // Performance
    loss_ratio: f64,  // claims / premiums
    profit: f64,
    is_insolvent: bool,

    // Exposure
    exposure_by_peril_region: HashMap<usize, f64>,
    uniform_deviation: f64,
}
```

**Industry Stats:**
```rust
pub struct IndustryStats {
    num_active_syndicates: usize,
    num_bankruptcies: usize,
    total_risks: usize,
    total_policies: usize,

    avg_premium: f64,
    avg_loss_ratio: f64,
    avg_syndicate_capital: f64,

    industry_avg_claim_frequency: f64,
    industry_avg_claim_cost: f64,
}
```

### Probability Distributions Used

1. **Poisson** for:
   - Number of risks per day (λ = RPD)
   - Number of attritional claims per risk (λ = yearly claim frequency)
   - Number of catastrophe events (λ = mean cat events × years)

2. **Gamma** for:
   - Attritional loss severity
   - Shape = 1/COV², Scale = μ·COV²

3. **Truncated Pareto** for:
   - Catastrophe loss severity
   - Shape parameter (default 5)
   - Minimum = minimum catastrophe damage

### Key Parameters (from Table 13)

```rust
pub struct ModelConfig {
    // Broker
    risks_per_day: f64,              // 0.06
    num_peril_regions: usize,        // 10
    risk_limit: f64,                 // $10M
    lead_top_k: usize,               // 2
    follow_top_k: usize,             // 5

    // Attritional losses
    yearly_claim_frequency: f64,     // 0.1
    gamma_cov: f64,                  // 1.0
    gamma_mean: f64,                 // $3M

    // Catastrophe losses
    mean_cat_events_per_year: f64,   // 0.05
    pareto_shape: f64,               // 5.0
    min_cat_damage_fraction: f64,    // 0.25

    // Syndicate
    initial_capital: f64,            // $10M
    default_lead_line_size: f64,     // 0.5
    default_follow_line_size: f64,   // 0.1

    // Actuarial pricing
    internal_experience_weight: f64, // 0.5
    loss_recency_weight: f64,        // 0.2
    volatility_weight: f64,          // 0.0

    // Underwriting
    underwriter_recency_weight: f64, // 0.2

    // Dividend
    profit_fraction: f64,            // 0.4

    // VaR EM
    var_exceedance_prob: f64,        // 0.05
    var_safety_factor: f64,          // 1.0

    // Premium EM
    premium_reserve_ratio: f64,      // 0.5
    min_capital_reserve_ratio: f64,  // 1.0
    max_scaling_factor: f64,         // 1.0
}
```

## Implementation Strategy

### Phase 1: Core Infrastructure
1. Implement Time, Broker, CentralRiskRepository agents
2. Basic risk generation and tracking
3. Simple attritional loss generator

### Phase 2: Basic Pricing
1. Implement Syndicate with ActuarialPricer
2. Lead quote mechanism
3. Premium EM

### Phase 3: Advanced Features
1. Underwriter markup pricing
2. VaR EM
3. Catastrophe loss generator

### Phase 4: Lloyd's-Specific
1. Lead-follow dynamics
2. Line size calculations
3. Broker-syndicate networks

### Testing Strategy

Following CLAUDE.md testing philosophy:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn given_actuarial_pricing_when_fair_losses_then_price_converges() {
        let mut pricer = ActuarialPricer::new_with_seed(0, 12345);

        // Simulate 100 losses at mean $3M
        for _ in 0..100 {
            pricer.record_loss(3_000_000.0);
        }

        let price = pricer.calculate_price(/* industry stats */);

        // Should converge to ~$300k (0.1 frequency × $3M mean)
        assert!((price - 300_000.0).abs() < 50_000.0);
    }

    #[test]
    fn given_catastrophe_event_when_affects_region_then_all_policies_hit() {
        let mut repo = CentralRiskRepository::new();

        // Create 10 risks in region 0
        for i in 0..10 {
            let risk = Risk { id: i, peril_region: 0, limit: 10_000_000.0, /* ... */ };
            repo.register_risk(risk);
            repo.bind_policy(i, 0 /* syndicate */, 1.0 /* full line */);
        }

        // Generate catastrophe
        let cat_loss = 50_000_000.0;
        let claims = repo.apply_catastrophe(0 /* region */, cat_loss);

        // All 10 policies should receive claims
        assert_eq!(claims.len(), 10);
    }

    #[test]
    fn given_var_em_when_region_overexposed_then_quote_rejected() {
        let mut em = VaRExposureManager::new_with_seed(0, 10 /* regions */, 12345);
        em.update_capital(10_000_000.0);

        // Overexpose to region 0
        for _ in 0..20 {
            em.record_exposure(0, 1_000_000.0);
        }

        let decision = em.evaluate_quote(0 /* region */, 1_000_000.0);

        assert_eq!(decision, ExposureDecision::Reject);
    }

    #[test]
    fn given_lead_follow_when_losses_shared_then_experience_coupled() {
        // Test that loss ratios converge when risks syndicated
        let mut sim = create_simulation_with_lead_follow();
        sim.run_until(365 * 10); // 10 years

        let stats: Vec<SyndicateStats> = sim.stats();
        let loss_ratios: Vec<f64> = stats.iter().map(|s| s.loss_ratio).collect();

        // Calculate standard deviation of loss ratios
        let mean: f64 = loss_ratios.iter().sum::<f64>() / loss_ratios.len() as f64;
        let variance: f64 = loss_ratios.iter()
            .map(|&lr| (lr - mean).powi(2))
            .sum::<f64>() / loss_ratios.len() as f64;
        let std_dev = variance.sqrt();

        // Should be tightly coupled (low std dev)
        assert!(std_dev < 0.1);
    }
}
```

## Key Insights for Replication

1. **Actuarial fair price** emerges from weighted average of syndicate + industry experience
2. **Underwriting cycles** are driven by catastrophe events, not just pricing competition
3. **Lead-follow mechanics** are crucial for stability (not just a regulatory curiosity)
4. **Exposure management** sophistication directly correlates with survival rates
5. **Temporal granularity** matters: Day events for risk generation, Month for stats, Year for dividends

## Validation Approach

The paper validates qualitatively by:
1. Showing premiums converge to analytical fair price
2. Demonstrating cyclicality matches empirical observations (Figure 3)
3. Confirming VaR EM leads to uniform peril distribution
4. Verifying lead-follow reduces volatility (as Lloyd's claims)

For rs-des implementation:
- Compare premium convergence to analytical fair price ($300k)
- Verify loss ratios fluctuate around 1.0
- Confirm catastrophes cause premium spikes
- Validate VaR EM uniform deviation approaches 0
- Check lead-follow scenario has no insolvencies vs. baseline

## Extensions & Future Work

From paper's conclusion:
1. **Reinsurance**: Model reinsurance availability and cost
2. **Risk heterogeneity**: Currently homogeneous risks; add risk classes
3. **Underwriter markup experiments**: Compare actuarial vs. market-driven pricing
4. **Dividend impact**: Study capital provider behavior
5. **Calibration**: Use proprietary Ki/Brit data for parameter fitting
6. **Global sensitivity analysis**: Systematic parameter exploration

## References to Other Models

The paper builds on:
- **Owadally et al. (2018, 2019)**: Circular topology, actuarial pricing equations
- **Zhou (2013)**: Underwriter markup formulation
- **Heinrich et al. (2021)**: VaR exposure management, catastrophe modeling
- **Venezian (1985)**: Underwriting cycle hypothesis
- **Boyer et al. (2012)**: Critique of time-series approaches

Key differences from prior work:
- **DES vs. ABM**: Handles asynchronous events
- **Lead-follow**: First model of Lloyd's-specific syndication
- **Dual loss types**: Both attritional and catastrophe
- **Modular architecture**: Plug-and-play components

## Conclusion

This paper provides a comprehensive blueprint for simulating specialty insurance markets using DES. The model successfully reproduces key market phenomena (underwriting cycles, capital volatility, insolvency patterns) and validates unique Lloyd's structures (lead-follow, risk syndication).

For rs-des implementation, the key challenges are:
1. Managing complex event dependencies (quote consolidation deadlines)
2. Coordinating syndicate sub-processes (quote component assembly)
3. Implementing catastrophe cascade logic (region → risks → syndicates)
4. Balancing computational cost (VaR Monte Carlo simulations)

The modular architecture and clear event flows make this an excellent candidate for translation to the rs-des framework, with potential for extension to other insurance markets beyond Lloyd's.
