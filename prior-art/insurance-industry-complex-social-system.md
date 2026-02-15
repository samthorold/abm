# The Insurance Industry as a Complex Social System: Competition, Cycles, and Crises

**Authors:** M. I. Owadally, F. Zhou, I. D. Wright
**Published:** Journal of Artificial Societies and Social Simulation, 2018, 21(4), 2
**DOI:** 10.18564/jasss.3819

## Overview

This paper investigates behavioral and socio-anthropological hypotheses for underwriting cycles and insurance crises through agent-based modeling. The central research question is whether simple individual-level behavior and social interaction can generate complex industry-wide cyclical behavior including cycles (5-8 year periods of price/profitability swings) and crises (severe unavailability or expense of insurance).

## Main Aims and Findings

### Research Goals
1. Test the transmission mechanism hypothesis: can simple firm-level behavior and interaction propagate to create complex industry-wide cycles?
2. Validate socio-anthropological theories (Theory of Plural Rationalities) and behavioral economics explanations for underwriting cycles
3. Demonstrate that heterogeneity and interaction at the micro level must be understood to manage macro-level phenomena

### Key Findings
- **Simple behavior DOES generate complex cycles**: Agent-based model with realistic but simple behavioral rules produces endogenous cycles matching real-world data
- **Cycle period**: Model generates ~5.9 year cycles, consistent with observed 5-8 year underwriting cycles
- **Role dynamics matter**: The interplay between underwriters (profit-maximizers) and actuaries (risk managers) significantly influences cycle amplitude and persistence
- **Tradeoff exists**: Less volatile short-term premiums may come at the cost of greater long-term cyclicality

## Theoretical Background

### Behavioral Hypotheses Being Tested

**Fear and Career Concerns (Fitzpatrick 2004)**
- Underwriters compete for market share, fearing job loss to peers
- Short-term incentives dominate, creating mispricing that manifests later
- Fear of insolvency drives conservative pricing after losses

**Mass Psychology (Feldblum 2001)**
- Underwriters exhibit herd behavior and "conjectural variation"
- Industry-wide sentiment swings between optimism and pessimism
- Price signaling through trade magazines coordinates industry rate changes

**Theory of Plural Rationalities (Ingram & Underwood 2010)**
- Four risk attitude types exist within insurance firms:
  - Underwriters = "Individualists" (risk-taking profit-maximizers)
  - Claims adjusters = "Egalitarians" (risk-averse conservators)
  - Actuaries = "Authoritarians" (prudent risk managers)
  - Operations managers = "Fatalists" (pragmatists)
- Shifting bureaucratic power between groups drives cycles
- When profits are high, underwriters dominate and relax standards
- When distressed, conservators and risk managers take control

**Behavioral Economics (Kunreuther et al. 2013)**
- Insurance managers are risk-averse AND ambiguity-averse
- After catastrophic losses, managers add irrational premiums due to "unknown unknowns"
- Conservatism bias (under-reaction) alternates with representativeness heuristic (over-reaction)

## Experiment Setup

### Agent Architecture

**Insurers (N = 20)**
- Fixed locations on circular preference landscape (representing product attributes, branding, distribution channels, etc.)
- Heterogeneous by design
- Maintain capital that changes with profitability
- All start with equal capital

**Customers (M = 1000)**
- Fixed locations on circular preference landscape (representing preferences)
- Must purchase insurance annually
- Make independent random claims during year

### Circular Preference Landscape
- Abstract 1D circular space
- Insurer position = product attributes (reliability, branding, distribution, payment methods)
- Customer position = preferences
- Distance = affinity/disutility
- Captures non-price factors in insurance choice

### Insurer Pricing Mechanism

Two-stage process at each time step t:

**Stage 1: Actuarial Rate-Making**

Actuary calculates pure premium P̃ᵢₜ:
- Weighted average of insurer's own average past claims and industry-wide average claim
- Uses credibility factor z (z = 0.2): higher z = more weight on own experience
- Own average calculated as exponentially weighted moving average with parameter w (w = 0.2)

Actuary adds risk loading:
- Actuarial premium rate = P̃ᵢₜ + αFᵢₜ
- Fᵢₜ = standard deviation of claims in year (t-1, t)
- α = loading factor (α = 0.001)
- Captures that riskier experience requires higher premiums

**Stage 2: Underwriter Pricing**

Underwriter calculates market price:

```
Pᵢₜ = (P̃ᵢₜ + αFᵢₜ) × e^(mᵢₜ)
```

Where mᵢₜ is mark-up based on:
- Arc price-elasticity of demand calculated from previous two years
- Smoothed update: mᵢₜ = β × m̂ᵢₜ + (1-β) × mᵢ,ₜ₋₁
- β = 0.3: controls how aggressively underwriters respond to market conditions
- **β is critical parameter**: Low β gives actuaries more influence; high β gives underwriters more influence

### Customer Purchase Decision

Customer j calculates total cost from insurer i:

```
TCᵢⱼₜ = Pᵢₜ + γΔᵢⱼ
```

Where:
- Pᵢₜ = insurance price
- Δᵢⱼ = distance along shorter arc on circular landscape
- γ = 0.08 = cost/disutility per unit distance

**Allocation Process:**
1. Each customer ranks insurers by total cost
2. Customer chooses lowest-cost insurer
3. If insurer at capacity (capital constraint), customer goes to second choice
4. Process continues until all customers have policies

### Claims Generation

**Frequency:** Bernoulli distribution with parameter b = 1 (every customer makes claim)

**Severity:** Gamma distribution
- Mean μG = 100
- Standard deviation σG = 10
- Independent across customers

### Loss Ratio Calculation

At year end:
```
Loss Ratio = Total Claims Paid / Total Premiums Earned
```

Industry-wide metric aggregated across all insurers.

## Parameter Values

### Critical Parameters (affect qualitative behavior)
- **α = 0.001**: Risk loading factor
- **β = 0.3**: Mark-up smoothing weight (lower = more actuarial influence)
- **γ = 0.08**: Distance cost weight (customer brand loyalty)

### Structural Parameters
- **N = 20**: Number of insurers
- **M = 1000**: Number of customers
- **T = 1000**: Simulation time horizon

### Actuarial Parameters
- **z = 0.2**: Credibility factor (20/80 rule of thumb)
- **w = 0.2**: Exponential smoothing parameter

### Claims Parameters
- **μG = 100**: Mean claim amount
- **σG = 10**: Standard deviation of claims
- **b = 1**: Claim frequency parameter

Parameters estimated using method of moments with grid search to match:
1. Mean loss ratio in UK property insurance data
2. Standard deviation of loss ratio
3. Lag-1 autocorrelation of loss ratio

## Expected Outcomes

### 1. Endogenous Cycle Generation

**Visual Pattern:**
- Loss ratio time series should show regular oscillations
- Period of approximately 5-8 years
- Sharp turning points indicating crisis-like behavior
- Pattern should persist indefinitely (not transient)

**Quantitative Validation:**
The following tests verify cycle presence:

### 2. AR(2) Model Fitting

Fit autoregressive model:
```
Πₜ = a₀ + a₁Πₜ₋₁ + a₂Πₜ₋₂ + εₜ
```

**Expected coefficients:**
- a₀ ≈ 0.937 (intercept)
- a₁ ≈ 0.467 (positive, indicating persistence)
- a₂ ≈ -0.100 (negative, enabling oscillation)

**Cycle conditions:**
For stationary cycles in AR(2):
1. a₁ > 0 ✓
2. -1 < a₂ < 0 ✓
3. a₁² + 4a₂ < 0 ✓ (ensures complex roots)

**Model fit:**
- AIC should favor AR(2) over AR(1) or AR(3)
- Adjusted R² around 0.19 (weak but significant)

### 3. Spectral Analysis

**Periodogram characteristics:**
- Single dominant peak at frequency ≈ 0.17 cycles/year
- Corresponds to period of 1/0.17 ≈ 5.9 years
- Peak should occur at approximately same frequency as UK data
- Shape of periodogram should match real data

### 4. Distribution Matching

**Empirical distribution function:**
- Simulated loss ratio distribution should closely match UK property insurance data
- Kolmogorov-Smirnov test should fail to reject null hypothesis (distributions are same) at 5% significance
- Note: Model may slightly under-represent very low loss ratios (high profitability periods)

### 5. Stationarity Properties

**Augmented Dickey-Fuller (ADF) test:**
- Should reject unit root null hypothesis at 5% significance
- Indicates loss ratios are stationary around mean
- Autocorrelations should decay with increasing lag

**Ergodicity:**
- Ensemble average (across simulation runs) should equal time average (along single run)
- Means and standard deviations stable across different run lengths and ensemble sizes

### 6. Parameter β Experiment Results

Testing different β values reveals actuary-underwriter dynamics:

**Low β (e.g., β = 0.2) - Actuaries Dominant:**
- Loss ratio less volatile (smaller swings)
- Higher autocorrelation (more persistent patterns)
- Clear cyclical structure in autocorrelation function
- More stable, predictable behavior

**Medium β (e.g., β = 0.4):**
- Moderate volatility
- Moderate autocorrelation
- Cycles still visible but less regular

**High β (e.g., β = 0.6-1.0) - Underwriters Dominant:**
- Loss ratio highly volatile (sharp spikes)
- Low autocorrelation (approaching white noise)
- Cycles disappear into random fluctuations
- Underwriter reactions amplify rather than dampen market dynamics

**Key Insight:** Actuarial influence provides stabilizing feedback, while underwriter dominance creates destabilizing positive feedback loops.

### 7. Crisis Indicators

**Sharp turning points:**
- Occasional extreme peaks in loss ratio (>1.2)
- Rapid transitions from profitability to loss
- Asymmetric: crises (upward spikes) more severe than soft markets (downward swings)

**Frequency:**
- Severe events (loss ratio > 1.2) should occur roughly 1-2 times per 30-year period
- Matches historical crisis frequency

## Validation Strategy

### Internal Validation

**Sensitivity Analysis:**
Expected relationships should hold:
- Increasing M (customers) → lower mean loss ratio, lower volatility
- Increasing N (insurers) → higher mean loss ratio, higher volatility (more competition)
- Increasing μG or σG → proportional increase in loss ratio mean/variance

**Reliability:**
- Loss ratios stationary (ADF test)
- Model ergodic (ensemble = time averages)
- Statistics stable across run lengths and ensemble sizes

### Outcome Validation

**Distribution comparison:**
- Kolmogorov-Smirnov test between simulated and actual UK data
- Should fail to reject at 5% significance
- Empirical CDFs should be visually close

### Process Validation

**Realism of mechanisms:**
- Actuarial pricing follows standard credibility theory and rate-making practice
- Underwriter mark-up pricing is standard cost-plus pricing (60% of UK firms use this)
- Customer decision includes non-price factors (consistent with branding/marketing expenditures)
- Heterogeneity in insurers and customers matches reality
- Interaction mechanisms (data pooling, competition) are realistic

## Implementation Verification

To verify correct implementation, check these properties:

### Statistical Properties
1. Loss ratio mean ≈ 1.0 (actuarially fair pricing in long run)
2. Loss ratio standard deviation matches UK data
3. Lag-1 autocorrelation matches UK data
4. Stationarity confirmed by ADF test

### Behavioral Properties
1. Insurers with better locations (more customers nearby) have stable market share
2. Price wars emerge occasionally when multiple insurers compete for same customers
3. Capital constraints sometimes bind (insurers can't accept all bids)
4. Market clearing occurs (all customers get insurance)

### Cycle Properties
1. Cycles persist indefinitely (not transient initialization effect)
2. Cycle period approximately 5-9 years
3. Cycles more regular when β is low
4. Cycles break down into noise when β approaches 1

### Sensitivity to Parameters
1. Results robust to changes in w, z (actuarial parameters)
2. Results robust to claims parameters (μG, σG) - these only affect scale
3. Results sensitive to α, β, γ (interaction parameters)
4. N and M must be large enough to avoid quantization effects

## Key Implementation Notes

### Initialization
- Discard first 100 time periods to avoid transient effects
- All insurers start with equal capital
- Use fixed random seed for reproducibility in validation

### Time Step Flow
1. Year begins (time t)
2. Each insurer calculates actuarial premium (using data through t-1)
3. Each insurer calculates market price (using elasticity from t-2, t-1)
4. Each customer calculates total cost for all insurers
5. Market clearing algorithm allocates customers to insurers
6. During year: claims randomly generated and paid
7. Year ends: update capital, calculate industry loss ratio
8. Repeat

### Circular Distance Calculation
Distance Δᵢⱼ is shorter arc length on unit circle:
```
Δᵢⱼ = min(|θᵢ - θⱼ|, 2π - |θᵢ - θⱼ|)
```

### Capital Constraints
Insurer i can accept maximum premium:
```
Max Premium = Capital_i × leverage_ratio
```
(Paper doesn't specify exact leverage ratio; can be calibrated)

### Data Structures
- Insurers: position, capital, price history, claims history, customer list
- Customers: position, insurer choice, claim amount
- Market: aggregate premium, aggregate claims, loss ratio time series

## Research Interpretation

### What This Model Proves
1. Simple individual behavior CAN generate complex macro behavior
2. Behavioral hypotheses have plausible transmission mechanism
3. Heterogeneity and interaction are essential to cycles
4. Role dynamics within firms matter for industry dynamics

### What This Model Does NOT Prove
1. That behavioral factors are the ONLY cause of cycles
2. That other explanations (capacity constraints, interest rates) are wrong
3. Exact quantitative predictions about future cycles
4. Causality in real-world insurance industry

### Policy Implications (from paper)
- Managing cycles requires understanding micro-level behavior
- Regulatory focus on individual firm behavior may be insufficient
- Interaction effects and feedback loops must be considered
- Tradeoff between short-term stability and long-term cyclicality

## Extensions and Future Work

The paper suggests several directions:

1. **Test other behavioral theories:** Implement specific cognitive biases (ambiguity aversion, overconfidence)
2. **Entry and exit:** Allow insurer population to change
3. **Catastrophic losses:** Add correlated claims events
4. **Regulation:** Model capital requirements, price controls
5. **Different product lines:** Test if cycles occur independently
6. **Network structure:** Replace circular landscape with explicit network
7. **Learning:** Give agents ability to adapt strategies

## References to Key Theories

The model synthesizes ideas from:
- **Fitzpatrick (2004):** Fear-based explanation, shifting bureaucratic power
- **Feldblum (2001):** Mass psychology, conjectural variation, customer inertia
- **Ingram & Underwood (2010):** Theory of Plural Rationalities
- **Kunreuther et al. (2013):** Behavioral economics, ambiguity aversion
- **Boyer et al. (2013):** Conservatism bias and representativeness heuristic
- **Harrington & Danzon (1994):** Winner's curse, heterogeneous insurers

## Summary for Implementation

**Core Mechanism:** Actuaries provide stabilizing feedback based on past data; underwriters provide destabilizing feedback based on competitive pressure. The tension between these two forces, combined with customer inertia (γ parameter) and market heterogeneity, generates endogenous cycles.

**Essential Features for Replication:**
1. Heterogeneous insurers on preference landscape
2. Two-stage pricing (actuarial + underwriter)
3. Customer choice based on price + distance
4. Capital constraints on insurers
5. Random independent claims
6. Parameter β controlling actuary/underwriter influence

**Success Criteria:**
- Cycles with ~6 year period
- AR(2) model with coefficients satisfying cycle conditions
- Periodogram peak at frequency ~0.17
- Distribution matching UK data (K-S test)
- Higher β → more volatility, less cyclicality
