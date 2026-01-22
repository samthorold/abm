# Taming the Basel Leverage Cycle: A Comprehensive Analysis

## Overview and Motivation

This paper by Aymanns, Caccioli, Farmer, and Tan investigates how Value-at-Risk (VaR) based leverage constraints, as mandated by Basel II banking regulations, can endogenously generate systemic risk through what the authors term the **Basel leverage cycle**. The core insight is profoundly counterintuitive: regulations designed to make individual banks safer can, through feedback effects, create system-wide instability.

---

## Key Points from the Paper

### The Central Problem

Leverage—borrowing to amplify returns—is fundamental to banking. Because higher leverage means higher risk, regulators and internal risk managers impose leverage constraints that adapt to perceived risk levels. Under Basel II, banks estimate their risk using historical volatility and adjust their leverage accordingly. When volatility appears low, banks can take on more leverage; when volatility appears high, they must reduce it.

This creates a dangerous feedback loop:

1. **Rising prices** → Lower measured volatility → Higher allowed leverage → Banks buy more assets → Prices rise further
2. **Falling prices** → Higher measured volatility → Lower allowed leverage → Banks must sell assets → Prices fall further

The result is that microprudential regulation (designed to keep individual banks safe) can generate macroprudential instability (systemic crises affecting the entire financial system).

### Three Stability Regimes

The model exhibits three distinct behavioral regimes depending on parameter values:

| Regime | Characteristic | Behavior |
|--------|---------------|----------|
| **(i) Stable** | Low leverage, small bank | System converges to fixed point equilibrium |
| **(ii) Locally Unstable** | Intermediate leverage | Chaotic leverage cycles with 10-15 year periods |
| **(iii) Globally Unstable** | High leverage | System diverges—prices go to infinity or zero |

The most empirically relevant finding is regime (ii), where the system generates endogenous oscillations resembling the Great Financial Moderation (gradual price rise, declining volatility) followed by sudden crashes (2008 crisis).

### The Leverage Control Policy Spectrum

The paper introduces a parameterized leverage control policy:

$$\bar{\lambda}(t) = \alpha(\sigma^2(t) + \sigma_0^2)^b$$

Where:
- **λ̄(t)** is the target leverage at time t
- **α** is the bank's "riskiness" parameter (higher α = more aggressive leverage)
- **σ²(t)** is the bank's perceived risk (historical volatility estimate)
- **σ₀²** is a small offset to bound leverage when volatility is very low
- **b** is the **cyclicality parameter** (the key policy variable)

The cyclicality parameter b determines the nature of the policy:
- **b = -0.5**: Fully procyclical (Basel II/VaR) — leverage decreases when volatility increases
- **b = 0**: Constant leverage — leverage unchanged regardless of volatility
- **b = +0.5**: Countercyclical — leverage increases when volatility increases

### Key Empirical Findings

1. **The critical leverage threshold is independent of cyclicality** — the transition from stability to instability occurs at roughly the same leverage level regardless of whether the policy is procyclical or countercyclical.

2. **Procyclical policies create chaotic cycles; countercyclical policies create explosive instability** — there's no "safe" direction of deviation from constant leverage.

3. **Optimal policy depends on market structure**:
   - Small bank + high exogenous noise → Basel II (b = -0.5) is optimal
   - Large bank + low exogenous noise → Constant leverage (b ≈ 0) is optimal

4. **Slower adjustment speeds dramatically increase stability** — banks that adjust leverage slowly create far less systemic risk than those that adjust quickly.

5. **Longer risk estimation horizons improve stability** — using historical windows longer than 2-3 years reduces crisis frequency.

---

## The Model: First Principles Explanation

### Agent Structure

The model consists of two representative agents interacting through a single risky asset market:

**The Bank**: A leveraged investor representing the entire banking sector. The bank:
- Maintains a fixed portfolio weight between the risky asset and cash
- Estimates future volatility using an exponential moving average of past returns
- Sets a leverage target based on this estimated volatility
- Adjusts its balance sheet toward this target leverage
- Maintains a constant equity target (paying dividends when above, raising capital when below)

**The Fund**: An unleveraged fundamentalist investor representing the rest of the financial system. The fund:
- Trades the risky asset toward a fundamental value μ
- Is subject to exogenous demand shocks with clustered volatility (GARCH process)
- Absorbs the equity flows from the bank's dividend payments/capital raises

**The Lender**: A passive entity providing unlimited credit to the bank at zero interest (implicit in the model).

### The State Space

The system is fully described by six state variables:

| Variable | Description |
|----------|-------------|
| σ²(t) | Bank's perceived risk (historical volatility estimate) |
| w_F(t) | Fund's portfolio weight in the risky asset |
| p(t) | Current price of the risky asset |
| n(t) | Fraction of risky asset owned by the bank |
| L_B(t) | Bank's liabilities |
| p'(t) | Lagged price (price at previous time step) |

### The Dynamical System

The model evolves as a discrete-time iterated map x(t+τ) = g(x(t)) with six coupled equations:

**Equation 1: Volatility Estimation**
$$\sigma^2(t+\tau) = (1-\tau\delta)\sigma^2(t) + \tau\delta\left[\log\frac{p(t)}{p'(t)}\frac{t_{VaR}}{\tau}\right]^2$$

This is an exponential moving average where δ controls the memory length. The term t_VaR/τ rescales returns to the regulatory time horizon.

**Equation 2: Fund's Portfolio Weight**
$$w_F(t+\tau) = w_F(t) + \frac{w_F(t)}{p(t)}\left[\tau\rho(\mu - p(t)) + \sqrt{\tau}s(t)\xi(t)\right]$$

The fund's weight reverts toward the fundamental value μ at rate ρ, plus a GARCH noise term that provides exogenous volatility clustering.

**Equation 3: Market Clearing Price**
$$p(t+\tau) = \frac{w_B(c_B(t) + \Delta B(t)) + w_F(t+\tau)c_F(t)}{1 - w_Bn(t) - (1-n(t))w_F(t+\tau)}$$

The price is determined by equating supply (one unit of the risky asset) with demand from both agents.

**Equation 4: Bank Ownership**
$$n(t+\tau) = \frac{w_B(n(t)p(t+\tau) + c_B(t) + \Delta B(t))}{p(t+\tau)}$$

**Equation 5: Bank Liabilities**
$$L_B(t+\tau) = L_B(t) + \Delta B(t)$$

**Equation 6: Price Lag**
$$p'(t) = p(t+\tau)$$

### The Feedback Mechanism

The core instability arises from the interaction between equations (1) and (3). Here's the causal chain:

1. **Bank estimates volatility** from historical prices (Eq. 1)
2. **Bank sets leverage target** based on estimated volatility (the leverage policy function)
3. **Bank adjusts holdings** to reach target leverage (through ΔB)
4. **Price changes** as bank buys or sells (Eq. 3)
5. **New prices affect volatility estimate** (back to Eq. 1)

When leverage is high, this feedback loop amplifies perturbations. Small price movements cause large leverage adjustments, which cause large price movements, which cause larger leverage adjustments—a classic positive feedback instability.

---

## The Experiments Conducted

### Experiment 1: Four Scenarios Comparing Micro vs. Macro Prudential Cases

The authors systematically vary two binary conditions to create four scenarios:

| Scenario | Bank Size | Noise | Result |
|----------|-----------|-------|--------|
| (i) Deterministic Microprudential | Small (Ē=10⁻⁵) | None | Fixed point equilibrium |
| (ii) Deterministic Macroprudential | Large (Ē=2.27) | None | Chaotic leverage cycles |
| (iii) Stochastic Microprudential | Small (Ē=10⁻⁵) | GARCH | Mean-reverting random walk |
| (iv) Stochastic Macroprudential | Large (Ē=2.27) | GARCH | Irregular leverage cycles |

**Purpose**: Demonstrate that leverage cycles emerge endogenously from the interaction between leverage targeting and price dynamics, independent of external shocks.

**Key Finding**: The Basel leverage cycle occurs even in the deterministic limit (scenario ii), proving that VaR-based risk management is sufficient to generate financial crises.

### Experiment 2: Stability Analysis via Bifurcation Diagram

The authors vary the risk parameter α (which controls leverage) and the cyclicality parameter b, mapping out the three stability regimes.

**Method**:
1. Compute the Jacobian matrix of the dynamical system at the fixed point
2. Find eigenvalues numerically
3. Classify stability based on whether the largest eigenvalue modulus exceeds 1

**Key Finding**: The critical leverage λ*_c at which instability emerges is independent of b—the system becomes unstable at roughly the same leverage regardless of policy cyclicality.

### Experiment 3: Lyapunov Exponent Analysis

For the stochastic case, traditional eigenvalue analysis doesn't apply. The authors compute Lyapunov exponents to characterize stability.

**Method**: Track separation between two trajectories with identical noise sequences but slightly different initial conditions. The Lyapunov exponent measures the average exponential rate of separation.

**Key Finding**: Strongly procyclical policies (b < -0.2) are destabilized by noise—the stochastic critical leverage is lower than the deterministic one. Near constant leverage (b ≈ 0), noise has little effect on stability.

### Experiment 4: Effect of Adjustment Speed

The authors vary θ, the speed at which banks adjust toward their leverage target.

**Key Finding**: Decreasing adjustment speed dramatically increases the critical leverage and critical bank size. Slow-adjusting banks create far less systemic risk.

### Experiment 5: Effect of Risk Estimation Horizon

The authors vary t_δ = 1/δ, the characteristic time over which volatility is estimated.

**Key Finding**: Longer estimation horizons increase stability. Short-term risk management (t_δ ≈ 1 year) is significantly more destabilizing than long-term management (t_δ > 7 years).

### Experiment 6: Optimal Leverage Control Policy

The central policy experiment varies the cyclicality parameter b while holding average leverage constant, measuring realized shortfall (tail risk) as the outcome.

**Three Sub-scenarios**:

1. **Microprudential risk dominates**: Small bank (R̂ = 10⁻⁵), strong GARCH
   - Optimal policy: b* = -0.5 (Basel II)

2. **Mixed micro/macro risk**: Intermediate bank (R̂ = 0.1), weaker GARCH
   - Optimal policy: b* ≈ -0.2 (mildly procyclical)

3. **Macroprudential risk dominates**: Large bank (R̂ = 0.27), weaker GARCH
   - Optimal policy: b* ≈ 0 (constant leverage)

---

## Implementation Guide

### Architecture Overview

An implementation of this model requires the following components:

1. **State representation**: A vector of six floats representing the current system state
2. **Parameter storage**: A structure holding all model parameters
3. **Update functions**: Six functions computing the next value of each state variable
4. **Market clearing**: A solver for the price given demands
5. **GARCH noise generator**: A stochastic process for exogenous shocks
6. **Analysis tools**: Functions for computing Lyapunov exponents, eigenvalues, and statistics

### Step 1: Define the Parameter Structure

Create a structure holding all model parameters. **Key decision**: Choose units carefully—the paper uses years as the base time unit and DXA-like monetary units where the fundamental price μ = 25 and bank equity target Ē can be varied.

```
Parameters:
    τ = 0.1          # Time step (years) — DECISION: Must be small enough for convergence
    δ = 0.5          # Memory parameter for volatility (year⁻¹) — gives ~2 year lookback
    t_VaR = 0.1      # VaR time horizon (years)
    θ = 10           # Leverage adjustment speed (year⁻¹)
    η = 10           # Equity redistribution speed (year⁻¹)
    b = -0.5         # Cyclicality parameter — DECISION: This is your policy variable
    σ₀² = 10⁻⁶       # Risk offset
    α = 0.075        # Risk level — DECISION: Controls average leverage
    Ē = 2.27         # Bank equity target — DECISION: Controls bank size
    w_B = 0.3        # Bank's portfolio weight in risky asset
    μ = 25           # Fundamental price
    ρ = 0.1          # Fund's mean reversion rate (year⁻¹)
    a₀ = 10⁻³        # GARCH baseline variance
    a₁ = 0.016       # GARCH error autoregressive term
    b₁ = 0.87        # GARCH variance autoregressive term
```

### Step 2: Initialize the State

**Key Decision**: Initial conditions matter for transient behavior but not for long-run dynamics (which settle onto an attractor). Start near the fixed point:

```
Initial state:
    σ² = σ₀²                           # Start at minimal perceived risk
    w_F = 0.5                          # Fund equally weighted
    p = μ                              # Price at fundamental value
    n = (1/μ) × α × σ₀^(2b) × Ē × w_B  # Bank ownership consistent with fixed point
    L_B = (α × σ₀^(2b) - 1) × Ē        # Liabilities consistent with target leverage
    p' = μ                             # Lagged price
```

### Step 3: Implement the GARCH Process

The exogenous noise follows a GARCH(1,1) process:

```
At each time step:
    s²(t) = a₀ + a₁ × χ²(t-1) + b₁ × s²(t-1)
    χ(t) = s(t) × ξ(t)

Where ξ(t) is a standard normal random variable.
```

**Key Decision**: The GARCH parameters determine how "bursty" the exogenous volatility is. The paper uses two settings:
- **Strong GARCH**: a₀=0.001, a₁=0.04, b₁=0.95 — highly clustered volatility
- **Weak GARCH**: a₀=0.001, a₁=0.016, b₁=0.874 — more stable volatility

### Step 4: Implement the Update Equations

**4a. Volatility Update**

```
function update_volatility(σ², p_current, p_lagged, τ, δ, t_VaR):
    log_return = log(p_current / p_lagged)
    scaled_return = log_return × (t_VaR / τ)
    σ²_new = (1 - τ × δ) × σ² + (τ × δ) × scaled_return²
    return σ²_new
```

**Key Decision**: The paper uses log returns rather than simple returns for consistency with standard risk measurement conventions.

**4b. Target Leverage Calculation**

```
function target_leverage(σ², α, σ₀², b):
    return α × (σ² + σ₀²)^b
```

**4c. Compute Auxiliary Quantities**

```
function compute_auxiliary(state, params):
    # Bank assets
    A_B = p × n / w_B

    # Target leverage
    λ_bar = target_leverage(σ², α, σ₀², b)

    # Equity
    E_B = A_B - L_B

    # Balance sheet adjustment needed to reach target leverage
    ΔB = τ × θ × (λ_bar × E_B - A_B)

    # Equity redistribution (toward target Ē)
    κ_B = τ × η × (Ē - E_B)
    κ_F = -κ_B  # Conservation: what leaves bank enters fund

    # Cash positions
    c_B = (1 - w_B) × n × p / w_B + κ_B
    c_F = (1 - w_F) × (1 - n) × p / w_F + κ_F

    return ΔB, κ_B, c_B, c_F
```

**Key Decision**: The equity redistribution (κ) ensures stationarity. Without it, wealth would accumulate in either the bank or fund indefinitely.

**4d. Fund Weight Update**

```
function update_fund_weight(w_F, p, μ, ρ, τ, s, ξ):
    mean_reversion = τ × ρ × (μ - p)
    noise = sqrt(τ) × s × ξ
    w_F_new = w_F + (w_F / p) × (mean_reversion + noise)
    return w_F_new
```

**Key Decision**: The noise scaling by √τ ensures proper behavior in the continuum limit τ→0.

**4e. Market Clearing**

```
function clear_market(n, w_B, w_F_new, c_B, c_F, ΔB):
    numerator = w_B × (c_B + ΔB) + w_F_new × c_F
    denominator = 1 - w_B × n - w_F_new × (1 - n)
    p_new = numerator / denominator
    return p_new
```

**Key Decision**: This assumes instantaneous market clearing with perfect liquidity. In reality, price impact is more complex.

**4f. Update Bank Ownership**

```
function update_ownership(n, p_new, c_B, ΔB, w_B):
    n_new = w_B × (n × p_new + c_B + ΔB) / p_new
    return n_new
```

**4g. Update Liabilities**

```
function update_liabilities(L_B, ΔB):
    return L_B + ΔB
```

### Step 5: Main Simulation Loop

```
function simulate(params, initial_state, T_steps):
    state = initial_state
    history = []

    # Initialize GARCH state
    s² = a₀ / (1 - a₁ - b₁)  # Unconditional variance
    χ_prev = 0

    for t in range(T_steps):
        # Generate GARCH noise
        ξ = random_normal(0, 1)
        s² = a₀ + a₁ × χ_prev² + b₁ × s²
        s = sqrt(s²)
        χ = s × ξ
        χ_prev = χ

        # Update fund weight (needs noise)
        w_F_new = update_fund_weight(state.w_F, state.p, μ, ρ, τ, s, ξ)

        # Compute auxiliary quantities
        ΔB, κ_B, c_B, c_F = compute_auxiliary(state, params)

        # Clear market
        p_new = clear_market(state.n, w_B, w_F_new, c_B, c_F, ΔB)

        # Update all state variables
        σ²_new = update_volatility(state.σ², state.p, state.p_prime, τ, δ, t_VaR)
        n_new = update_ownership(state.n, p_new, c_B, ΔB, w_B)
        L_B_new = update_liabilities(state.L_B, ΔB)
        p_prime_new = p_new

        # Package new state
        state = State(σ²_new, w_F_new, p_new, n_new, L_B_new, p_prime_new)
        history.append(state)

    return history
```

### Step 6: Computing Diagnostics

**6a. Realized Leverage**

```
function compute_leverage(state):
    A_B = state.p × state.n / w_B
    E_B = A_B - state.L_B
    return A_B / E_B
```

**6b. Equity Returns**

```
function compute_equity_return(state_prev, state_curr):
    E_prev = state_prev.p × state_prev.n / w_B - state_prev.L_B
    ΔE = state_curr.n × (state_curr.p - state_prev.p)
    return log((E_prev + ΔE) / E_prev)
```

**6c. Realized Shortfall**

```
function compute_realized_shortfall(returns, q):
    # Sort returns ascending
    sorted_returns = sort(returns)
    # Take worst q fraction
    cutoff_idx = floor(q × length(returns))
    worst_returns = sorted_returns[0:cutoff_idx]
    # Return average loss (negative of average return)
    return -mean(worst_returns)
```

### Step 7: Running the Experiments

**Experiment 1: Four Scenarios**

```
# Scenario (i): Deterministic microprudential
params_i = default_params.copy()
params_i.Ē = 1e-5
params_i.a₀ = params_i.a₁ = params_i.b₁ = 0  # No noise
history_i = simulate(params_i, initial_state, 5000)

# Scenario (ii): Deterministic macroprudential
params_ii = default_params.copy()
params_ii.Ē = 2.27
params_ii.a₀ = params_ii.a₁ = params_ii.b₁ = 0
history_ii = simulate(params_ii, initial_state, 5000)

# Scenarios (iii) and (iv): Same but with GARCH noise enabled
```

**Key Decision**: Run for at least 3000-5000 time steps to let transients decay and observe steady-state behavior.

**Experiment 2: Stability Boundary**

```
for b in linspace(-0.5, 0.5, 50):
    for α in logspace(-3, 0, 100):
        params = default_params.copy()
        params.b = b
        params.α = α

        # Compute Jacobian at fixed point
        J = compute_jacobian(params)
        eigenvalues = compute_eigenvalues(J)
        max_eigenvalue = max(abs(eigenvalues))

        if max_eigenvalue < 1:
            stability[b, α] = "stable"
        else:
            # Run simulation to check global stability
            history = simulate(params, initial_state, 1000)
            if is_bounded(history):
                stability[b, α] = "leverage_cycles"
            else:
                stability[b, α] = "globally_unstable"
```

**Key Decision**: Global stability cannot be determined analytically—must simulate and check if prices remain bounded.

**Experiment 3: Optimal Policy Search**

```
for scenario in [microprudential, mixed, macroprudential]:
    set_garch_params(scenario.garch)
    target_leverage = 5.8
    target_relative_size = scenario.R_hat

    for b in linspace(-0.5, 0.5, 50):
        # Adjust α and Ē to maintain targets
        α, Ē = calibrate_to_targets(b, target_leverage, target_relative_size)

        params = default_params.copy()
        params.b = b
        params.α = α
        params.Ē = Ē

        # Run long simulation
        history = simulate(params, initial_state, 50000)

        # Compute equity returns and risk measure
        returns = [compute_equity_return(history[t], history[t+1])
                   for t in range(len(history)-1)]
        risk[scenario, b] = compute_realized_shortfall(returns, q=0.05)

    optimal_b[scenario] = argmin(risk[scenario, :])
```

**Key Decision**: The calibration function `calibrate_to_targets` must iteratively adjust α and Ē because the realized leverage and relative size are emergent properties, not directly set parameters.

---

## Critical Implementation Decisions Summary

| Decision Point | Choice Made | Rationale |
|---------------|-------------|-----------|
| **Time step τ** | 0.1 years | Small enough for convergence, large enough for efficiency |
| **Return calculation** | Log returns | Standard in finance, additive over time |
| **Noise scaling** | √τ factor | Ensures proper continuum limit |
| **Equity target mechanism** | Redistributes between bank and fund | Maintains stationarity |
| **Market clearing** | Instantaneous | Simplifies analysis; real markets have friction |
| **Stability test** | Eigenvalues + simulation | Analytical for local, numerical for global |
| **Risk measure** | Realized shortfall at 5% | Consistent with Basel III |
| **Simulation length** | 5000+ steps | Sufficient for ergodic averaging |
| **Initial conditions** | At/near fixed point | Minimizes transient effects |

---

## Potential Extensions

The paper suggests several directions for more realistic implementations:

1. **Variable portfolio weights**: Allow flight-to-quality dynamics
2. **Default possibility**: Model bank failure and recapitalization
3. **Heterogeneous agents**: Multiple banks with different leverage targets
4. **Realistic Basel III rules**: Include risk weights, asset-price-dependent buffers, macro-economic countercyclicality
5. **Network effects**: Banks lending to each other, creating contagion channels

---

## Conclusion

This paper provides a striking demonstration of how simple, individually rational risk management rules can generate complex, systemically dangerous dynamics. The implementation reveals that the key feedback loop operates through just six coupled equations, yet produces rich behavior including fixed points, limit cycles, chaos, and explosive instability.

The central policy message is nuanced: there is no universally optimal leverage control policy. The right choice depends on the structure of the financial system—specifically, on whether endogenous or exogenous volatility dominates. When banks are small relative to markets, procyclical VaR-based leverage targeting (Basel II) is optimal. When banks are large and can move markets, constant leverage becomes superior because it avoids amplifying the feedback loop that creates systemic risk.

For practitioners implementing such models, the key is to carefully track the interplay between perceived risk (historical volatility), leverage decisions, and price impact. The model's behavior is highly sensitive to the adjustment speed θ and the memory parameter δ, suggesting that regulatory interventions targeting these parameters (e.g., requiring longer risk-estimation windows or slower balance sheet adjustments) could be highly effective at reducing systemic risk.
